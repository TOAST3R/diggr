//! Bleep pages, sent by the browser extension as data, never fetched by the app.
//!
//! bleep.com answers anything but a browser with a challenge, so the extension reads the
//! records of the page the user has open: a release page with its tracks, or a list (label,
//! artist, search) whose tiles show no tracks. Each field is checked here, a record that fails
//! is dropped alone, and every address (release page, cover) is rebuilt from checked ids.
//! Bleep's previews are issued per request by bleep.com itself, so Bleep tracks are searched
//! for, as Traxsource's are. A record from a list stands as one track, its title the record's,
//! until a Discogs link brings in its release's tracks (see `ui`'s stores).

use serde::Deserialize;
use serde_json::Value;

use crate::stores::{ListError, Listed, Stock, Store, StoreRecord, StoreTrack};
use crate::stores::{looks_like, text, valid_id, valid_slug};

/// The most records one send may hold.
pub const MAX_RECORDS: usize = 500;
/// The most tracks one record, and one send, may hold.
pub const MAX_TRACKS: usize = 60;
pub const MAX_SEND_TRACKS: usize = 5_000;
const MAX_FORMATS: usize = 8;
const MAX_SECS: u32 = 7_200;

const PAGE_HOST: &str = "https://bleep.com";
const COVER_HOST: &str = "https://d1rgjmn2wmqeif.cloudfront.net/r/s/";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct BleepFormat {
    /// "LP", "Download", "CD"… as shown.
    name: String,
    /// As shown ("22.99 €"): only displayed.
    #[serde(default)]
    price: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct BleepTrack {
    id: String,
    n: u8,
    title: String,
    /// Empty when it is the record's artist.
    #[serde(default)]
    artist: String,
    #[serde(default)]
    secs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct BleepRecord {
    /// The release id and its page's name part: `/release/‹id›-‹slug›`.
    id: String,
    slug: String,
    artist: String,
    title: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    catno: String,
    /// The cover's file name: `100168.jpg`, `509246-1.jpg`.
    #[serde(default)]
    image: String,
    #[serde(default)]
    formats: Vec<BleepFormat>,
    /// None on a list's tiles.
    #[serde(default)]
    tracks: Vec<BleepTrack>,
}

/// `100168.jpg` or `509246-1.jpg`.
fn valid_image(s: &str) -> bool {
    let Some(stem) = s.strip_suffix(".jpg") else {
        return false;
    };
    let (id, n) = stem.split_once('-').unwrap_or((stem, "1"));
    valid_id(id) && (1..=3).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit())
}

fn valid_record(r: &BleepRecord) -> bool {
    valid_id(&r.id)
        && valid_slug(&r.slug)
        && text(&r.artist, 1, 200)
        && text(&r.title, 1, 200)
        && text(&r.label, 0, 200)
        && text(&r.catno, 0, 40)
        && (r.image.is_empty() || valid_image(&r.image))
        && r.formats.len() <= MAX_FORMATS
        && r.formats
            .iter()
            .all(|f| text(&f.name, 1, 30) && text(&f.price, 0, 20))
        && r.tracks.len() <= MAX_TRACKS
        && r.tracks.iter().all(|t| {
            valid_id(&t.id)
                && (1..=99).contains(&t.n)
                && text(&t.title, 1, 200)
                && text(&t.artist, 0, 200)
                && t.secs <= MAX_SECS
        })
}

/// Whether `page` is a bleep.com address (only kept to say where a send came from).
fn bleep_page(page: &str) -> bool {
    page.len() <= 2048
        && ["https://bleep.com", "https://www.bleep.com"]
            .iter()
            .any(|h| {
                page.strip_prefix(h)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            })
}

/// Checks a send's `bleep` object: `{ "page": …, "records": [ … ] }`.
pub fn check(value: &Value) -> Result<Listed, ListError> {
    let obj = value.as_object().ok_or(ListError::Unsupported)?;
    if obj.keys().any(|k| k != "page" && k != "records") {
        return Err(ListError::Unsupported);
    }
    let page = obj.get("page").and_then(Value::as_str).unwrap_or_default();
    if !bleep_page(page) {
        return Err(ListError::Unsupported);
    }
    let raw = obj
        .get("records")
        .and_then(Value::as_array)
        .ok_or(ListError::Unsupported)?;
    let tracks: usize = raw
        .iter()
        .filter_map(|r| r.get("tracks").and_then(Value::as_array))
        .map(Vec::len)
        .sum();
    if raw.len() > MAX_RECORDS || tracks > MAX_SEND_TRACKS {
        return Err(ListError::TooMany);
    }
    let mut out = Listed::default();
    for v in raw {
        match serde_json::from_value::<BleepRecord>(v.clone()) {
            Ok(r) if valid_record(&r) => out.records.push(to_store(&r)),
            _ => out.refused += 1,
        }
    }
    if out.records.is_empty() {
        return Err(ListError::NothingUsable {
            refused: out.refused,
        });
    }
    Ok(out)
}

/// The record in the shape every store shares. A record shown without tracks (a list's tile)
/// stands as one track: the record's title, no position.
fn to_store(r: &BleepRecord) -> StoreRecord {
    let tracks = match r.tracks.is_empty() {
        true => vec![StoreTrack {
            artist: r.artist.clone(),
            title: r.title.clone(),
            track: format!("r{}", r.id),
            ..StoreTrack::default()
        }],
        false => r
            .tracks
            .iter()
            .map(|t| StoreTrack {
                position: t.n.to_string(),
                artist: match t.artist.is_empty() {
                    true => r.artist.clone(),
                    false => t.artist.clone(),
                },
                title: t.title.clone(),
                secs: t.secs,
                track: t.id.clone(),
                ..StoreTrack::default()
            })
            .collect(),
    };
    let names: Vec<&str> = r.formats.iter().map(|f| f.name.as_str()).collect();
    let prices: Vec<String> = r
        .formats
        .iter()
        .filter(|f| !f.price.is_empty())
        .map(|f| format!("{} {}", f.name, f.price))
        .collect();
    StoreRecord {
        store: Store::Bleep,
        id: r.id.clone(),
        page: release_url(&r.id, &r.slug).unwrap_or_default(),
        slug: r.slug.clone(),
        artist: r.artist.clone(),
        title: r.title.clone(),
        label: r.label.clone(),
        catno: r.catno.clone(),
        format: names.join(", "),
        stock: Stock::Unknown,
        price: prices.join(", "),
        cover: match valid_image(&r.image) {
            true => format!("{COVER_HOST}{}", r.image),
            false => String::new(),
        },
        tracks,
    }
}

/// A release's page, rebuilt from checked parts.
pub fn release_url(id: &str, slug: &str) -> Option<String> {
    (valid_id(id) && valid_slug(slug)).then(|| format!("{PAGE_HOST}/release/{id}-{slug}"))
}

/// Whether `text` looks like a Bleep address, old domain included (pasted ones are refused
/// with a hint).
pub fn looks_like_bleep(text: &str) -> bool {
    looks_like(text, "bleep.com") || looks_like(text, "bleep.co.uk")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn release() -> Value {
        json!({
            "id": "100168", "slug": "bibio-zen-drumsdada-drums", "artist": "Bibio",
            "title": "Zen Drums/Dada Drums", "label": "Warp Records", "catno": "WAP419",
            "image": "100168.jpg",
            "formats": [{ "name": "Download", "price": "2.49 €" }],
            "tracks": [
                { "id": "635821", "n": 1, "title": "Zen Drums", "secs": 397 },
                { "id": "635822", "n": 2, "title": "Dada Drums" },
            ],
        })
    }

    fn send(records: Vec<Value>) -> Value {
        json!({ "page": "https://bleep.com/label/6593-warp-records", "records": records })
    }

    #[test]
    fn a_release_page_becomes_a_record_with_its_tracks() {
        let out = check(&send(vec![release()])).unwrap();
        let r = &out.records[0];
        assert_eq!(
            r.page,
            "https://bleep.com/release/100168-bibio-zen-drumsdada-drums"
        );
        assert_eq!(
            r.cover,
            "https://d1rgjmn2wmqeif.cloudfront.net/r/s/100168.jpg"
        );
        assert_eq!(
            (r.format.as_str(), r.price.as_str()),
            ("Download", "Download 2.49 €")
        );
        let t = &r.tracks[0];
        assert_eq!(
            (t.position.as_str(), t.title.as_str(), t.secs),
            ("1", "Zen Drums", 397)
        );
        assert_eq!((t.artist.as_str(), t.clip.clone()), ("Bibio", None));
    }

    #[test]
    fn a_list_tile_stands_as_one_track() {
        let mut tile = release();
        tile["tracks"] = json!([]);
        tile["image"] = json!("509246-1.jpg");
        let r = &check(&send(vec![tile])).unwrap().records[0];
        assert_eq!(r.tracks.len(), 1);
        let t = &r.tracks[0];
        assert_eq!(
            (t.position.as_str(), t.title.as_str(), t.track.as_str()),
            ("", "Zen Drums/Dada Drums", "r100168")
        );
        assert!(r.cover.ends_with("/509246-1.jpg"));
    }

    #[test]
    fn a_bad_record_is_refused_alone() {
        let mut bad = release();
        bad["id"] = json!("100168/../x");
        let mut unknown = release();
        unknown["sample"] = json!("x");
        let mut long = release();
        long["tracks"] = json!([{ "id": "1", "n": 100, "title": "x" }]);
        let out = check(&send(vec![release(), bad, unknown, long])).unwrap();
        assert_eq!((out.records.len(), out.refused), (1, 3));
    }

    /// What the extension's reader (`bleep.js`) made of a release and a label page (2025
    /// copies): every record passes.
    #[test]
    fn the_extensions_reading_of_real_pages_passes() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/bleep");
        for (file, records, tracks) in [("release_page.json", 1, 2), ("label_page.json", 25, 25)] {
            let text = std::fs::read_to_string(format!("{dir}/{file}")).unwrap();
            let out = check(&serde_json::from_str(&text).unwrap()).unwrap();
            assert_eq!(out.refused, 0, "{file}");
            assert_eq!(out.records.len(), records, "{file}");
            let n: usize = out.records.iter().map(|r| r.tracks.len()).sum();
            assert_eq!(n, tracks, "{file}: a tile stands as one track");
        }
    }

    #[test]
    fn whole_sends_are_refused() {
        assert_eq!(check(&send(vec![release(); 501])), Err(ListError::TooMany));
        let elsewhere = json!({ "page": "https://evil.example/", "records": [release()] });
        assert_eq!(check(&elsewhere), Err(ListError::Unsupported));
        let mut bad = release();
        bad["artist"] = json!("");
        assert_eq!(
            check(&send(vec![bad])),
            Err(ListError::NothingUsable { refused: 1 })
        );
        assert!(looks_like_bleep("https://bleep.co.uk/release/1-x"));
        assert!(looks_like_bleep("bleep.com/label/4035-rubadub"));
        assert!(!looks_like_bleep("bleepingcomputer.com"));
    }
}
