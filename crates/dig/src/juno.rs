//! Juno Records pages, sent by the browser extension as data, never fetched by the app.
//!
//! Every juno.co.uk page sits behind a bot challenge, and getting around it is not ours to
//! do. The user's browser has passed it, so the extension reads the page the user has open
//! and sends the records it shows: ids, text and sample paths. Nothing here trusts that
//! data. Each field is checked against a strict pattern and length, a record that fails is
//! dropped alone, and every address (product page, sample, cover) is rebuilt from checked
//! ids. No address the extension sends is ever fetched.
//!
//! A sample is the one thing the app downloads from Juno: a 2-minute MP3 at a fixed path on
//! `mp3.juno.co.uk`, which answers without a challenge (see `preview::fetcher`).

use serde::Deserialize;
use serde_json::Value;

pub use crate::stores::{STALE_SECS, Seen, Stock, StoreMemory as JunoMemory};
use crate::stores::{Store, StoreRecord, StoreTrack, text, valid_slug};

/// The most records one send may hold (Juno's largest page size).
pub const MAX_RECORDS: usize = 500;
/// The most tracks one record may hold.
pub const MAX_TRACKS: usize = 60;
/// The most tracks one send may hold.
pub const MAX_SEND_TRACKS: usize = 5_000;
/// The longest a track may say it is, in seconds.
pub const MAX_SECS: u32 = 7_200;

const KEY_PREFIX: &str = "jn.";
const SAMPLE_HOST: &str = "https://mp3.juno.co.uk/MP3/";
const PAGE_HOST: &str = "https://www.juno.co.uk";
const COVER_HOST: &str = "https://imagescdn.juno.co.uk/full/";

/// One track as the page shows it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JunoTrack {
    /// "1", "2", "A"…: the side or disc heading it is listed under.
    #[serde(default)]
    pub side: String,
    /// Its place on that side, from 1.
    pub n: u8,
    /// Empty when the page names only the record (a listing showing one sample).
    #[serde(default)]
    pub title: String,
    /// Empty when it is the record's artist.
    #[serde(default)]
    pub artist: String,
    /// 0 when the page doesn't say.
    #[serde(default)]
    pub secs: u32,
    /// The sample's part after the product (`SF1178909-01-‹02-01›.mp3`): side and track.
    pub sample: String,
}

/// One record as the page shows it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JunoRecord {
    /// `1178909-01`.
    pub product: String,
    /// The product page's name part: `0-mika-vainio-rontgen-vinyl`.
    pub slug: String,
    pub artist: String,
    pub title: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub catno: String,
    /// `12"`, `LP`, `CD`…
    #[serde(default)]
    pub format: String,
    #[serde(default, deserialize_with = "stock")]
    pub stock: Stock,
    /// As shown ("€14.78"): only displayed, never read as an amount.
    #[serde(default)]
    pub price: String,
    /// The cover's file name: `CS1178909-01A-BIG.jpg`.
    #[serde(default)]
    pub image: String,
    pub tracks: Vec<JunoTrack>,
}

fn stock<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Stock, D::Error> {
    let s = String::deserialize(d)?;
    Stock::parse(&s).ok_or_else(|| serde::de::Error::custom("unknown stock"))
}

/// A send's records that passed every check, and how many didn't.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Checked {
    pub records: Vec<JunoRecord>,
    pub refused: usize,
}

/// Why a whole send was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    NotJuno,
    TooMany,
    NothingUsable { refused: usize },
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::NotJuno => f.write_str("not a Juno page"),
            SendError::TooMany => write!(
                f,
                "too many records (at most {MAX_RECORDS}, {MAX_SEND_TRACKS} tracks)"
            ),
            SendError::NothingUsable { refused } => {
                write!(f, "no record could be read ({refused} refused)")
            }
        }
    }
}

/// `1178909-01`: 1 to 9 digits, a dash, 2 digits.
pub fn valid_product(s: &str) -> bool {
    let Some((id, rel)) = s.split_once('-') else {
        return false;
    };
    (1..=9).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_digit())
        && rel.len() == 2
        && rel.bytes().all(|b| b.is_ascii_digit())
}

/// `02-01` (side 2, track 1): two groups of 2 digits.
pub fn valid_sample(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 2
        && parts
            .iter()
            .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_digit()))
}

fn valid_image(s: &str) -> bool {
    s.strip_suffix(".jpg").is_some_and(|stem| {
        (1..=40).contains(&stem.len())
            && stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

fn valid_side(s: &str) -> bool {
    s.len() <= 4 && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn valid_track(t: &JunoTrack) -> bool {
    valid_side(&t.side)
        && (1..=99).contains(&t.n)
        && text(&t.title, 0, 200)
        && text(&t.artist, 0, 200)
        && t.secs <= MAX_SECS
        && valid_sample(&t.sample)
}

fn valid_record(r: &JunoRecord) -> bool {
    valid_product(&r.product)
        && valid_slug(&r.slug)
        && text(&r.artist, 1, 200)
        && text(&r.title, 1, 200)
        && text(&r.label, 0, 200)
        && text(&r.catno, 0, 40)
        && text(&r.format, 0, 20)
        && text(&r.price, 0, 20)
        && (r.image.is_empty() || valid_image(&r.image))
        && (1..=MAX_TRACKS).contains(&r.tracks.len())
        && r.tracks.iter().all(valid_track)
}

/// Whether `page` is a juno.co.uk address (only kept to say where a send came from).
/// `www.` or a regional site (`es.`, `de.`…), which use the same product ids.
fn juno_page(page: &str) -> bool {
    let Some(rest) = page.strip_prefix("https://") else {
        return false;
    };
    let (host, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let region = host.strip_suffix(".juno.co.uk").unwrap_or_default();
    page.len() <= 2048
        && (region == "www"
            || (region.len() == 2 && region.bytes().all(|b| b.is_ascii_lowercase())))
        && (path.is_empty() || path.starts_with('/'))
}

/// Checks a send's `juno` object: `{ "page": …, "records": [ … ] }`. Each record is checked
/// on its own; one that fails (an unknown field included) is counted and dropped.
pub fn check(juno: &Value) -> Result<Checked, SendError> {
    let obj = juno.as_object().ok_or(SendError::NotJuno)?;
    if obj.keys().any(|k| k != "page" && k != "records") {
        return Err(SendError::NotJuno);
    }
    let page = obj.get("page").and_then(Value::as_str).unwrap_or_default();
    if !juno_page(page) {
        return Err(SendError::NotJuno);
    }
    let raw = obj
        .get("records")
        .and_then(Value::as_array)
        .ok_or(SendError::NotJuno)?;
    let tracks: usize = raw
        .iter()
        .filter_map(|r| r.get("tracks").and_then(Value::as_array))
        .map(Vec::len)
        .sum();
    if raw.len() > MAX_RECORDS || tracks > MAX_SEND_TRACKS {
        return Err(SendError::TooMany);
    }
    let mut out = Checked::default();
    for r in raw {
        match serde_json::from_value::<JunoRecord>(r.clone()) {
            Ok(rec) if valid_record(&rec) => out.records.push(rec),
            _ => out.refused += 1,
        }
    }
    if out.records.is_empty() {
        return Err(SendError::NothingUsable {
            refused: out.refused,
        });
    }
    Ok(out)
}

impl JunoRecord {
    /// The record in the shape every store shares: its page, cover and sample keys rebuilt.
    pub fn to_store(&self) -> StoreRecord {
        StoreRecord {
            store: Store::Juno,
            id: self.product.clone(),
            page: product_url(&self.slug, &self.product).unwrap_or_default(),
            slug: self.slug.clone(),
            artist: self.artist.clone(),
            title: self.title.clone(),
            label: self.label.clone(),
            catno: self.catno.clone(),
            format: self.format.clone(),
            stock: self.stock,
            price: self.price.clone(),
            cover: cover_url(&self.image).unwrap_or_default(),
            tracks: self
                .tracks
                .iter()
                .map(|t| StoreTrack {
                    position: match t.side.is_empty() {
                        true => t.n.to_string(),
                        false => format!("{}.{}", t.side, t.n),
                    },
                    artist: match t.artist.is_empty() {
                        true => self.artist.clone(),
                        false => t.artist.clone(),
                    },
                    title: match t.title.is_empty() {
                        true => self.title.clone(),
                        false => t.title.clone(),
                    },
                    secs: t.secs,
                    clip: Some(key(&self.product, &t.sample)),
                    ..StoreTrack::default()
                })
                .collect(),
        }
    }
}

// ---- keys and addresses ----------------------------------------------------------------

/// A sample's preview key: `jn.1178909-01-02-01`.
pub fn key(product: &str, sample: &str) -> String {
    format!("{KEY_PREFIX}{product}-{sample}")
}

/// The product and sample a key names, when it is a valid Juno key.
pub fn parse_key(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix(KEY_PREFIX)?;
    // The product is `digits-2digits`: the sample starts after its second dash.
    let first = rest.find('-')?;
    let second = first + 3;
    let (product, sample) = (rest.get(..second)?, rest.get(second + 1..)?);
    (rest.as_bytes().get(second) == Some(&b'-') && valid_product(product) && valid_sample(sample))
        .then_some((product, sample))
}

/// Whether a key is a Juno sample's (valid or not): `jn.` never starts a YouTube id.
pub fn is_key(key: &str) -> bool {
    key.starts_with(KEY_PREFIX)
}

/// The sample's address, rebuilt from a valid key.
pub fn sample_url(key: &str) -> Option<String> {
    let (product, sample) = parse_key(key)?;
    Some(format!("{SAMPLE_HOST}SF{product}-{sample}.mp3"))
}

/// The product's page, rebuilt from checked parts.
pub fn product_url(slug: &str, product: &str) -> Option<String> {
    (valid_slug(slug) && valid_product(product))
        .then(|| format!("{PAGE_HOST}/products/{slug}/{product}/"))
}

/// The cover's address, rebuilt from a checked file name.
pub fn cover_url(image: &str) -> Option<String> {
    valid_image(image).then(|| format!("{COVER_HOST}{image}"))
}

/// Whether `host` may serve a sample (the request and any redirect).
pub fn sample_host(host: &str) -> bool {
    matches!(host, "mp3.juno.co.uk" | "www.juno.co.uk")
}

/// Whether `text` looks like a Juno address (pasted ones are refused with a hint).
pub fn looks_like_juno(text: &str) -> bool {
    let t = text.trim().to_ascii_lowercase();
    let t = t
        .strip_prefix("https://")
        .or_else(|| t.strip_prefix("http://"))
        .unwrap_or(&t);
    let host = t.split(['/', '?', '#']).next().unwrap_or_default();
    host == "juno.co.uk" || host.ends_with(".juno.co.uk")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record() -> Value {
        json!({
            "product": "1178909-01",
            "slug": "0-mika-vainio-rontgen-vinyl",
            "artist": "0 aka Mika Vainio",
            "title": "Rontgen (reissue)",
            "label": "Sahko Finland",
            "catno": "SAHKO 00012",
            "format": "12\"",
            "stock": "in",
            "price": "€14.78",
            "image": "CS1178909-01A-BIG.jpg",
            "tracks": [
                { "side": "1", "n": 1, "title": "Rontgen", "secs": 301, "sample": "01-01" },
                { "side": "2", "n": 1, "title": "Track 2", "secs": 450, "sample": "02-01" }
            ]
        })
    }

    fn send(records: Vec<Value>) -> Value {
        json!({ "page": "https://www.juno.co.uk/products/0-mika-vainio-rontgen-vinyl/1178909-01/", "records": records })
    }

    #[test]
    fn a_product_page_is_accepted() {
        let c = check(&send(vec![record()])).unwrap();
        assert_eq!(c.refused, 0);
        let r = &c.records[0];
        assert_eq!(r.catno, "SAHKO 00012");
        assert_eq!(r.stock, Stock::In);
        assert_eq!(r.tracks.len(), 2);
        assert_eq!(r.tracks[1].secs, 450);
    }

    #[test]
    fn a_bad_record_is_dropped_alone() {
        let mut bad = record();
        bad["product"] = json!("1178909-01/../x");
        let mut extra = record();
        extra["evil"] = json!(1);
        let mut records = vec![record(); 18];
        records.push(bad);
        records.push(extra);
        let c = check(&send(records)).unwrap();
        assert_eq!((c.records.len(), c.refused), (18, 2));
    }

    #[test]
    fn each_field_is_checked() {
        let cases: Vec<(&str, Value)> = vec![
            ("product", json!("1178909")),
            ("product", json!("1234567890-01")),
            ("product", json!("1178909-1")),
            ("slug", json!("Has-Upper")),
            ("slug", json!("a/b")),
            ("artist", json!("")),
            ("title", json!("x".repeat(201))),
            ("title", json!("line\nbreak")),
            ("catno", json!("C".repeat(41))),
            ("format", json!("F".repeat(21))),
            ("stock", json!("maybe")),
            ("price", json!("€".repeat(21))),
            ("image", json!("../x.jpg")),
            ("image", json!("x.png")),
            ("tracks", json!([])),
        ];
        for (field, value) in cases {
            let mut r = record();
            r[field] = value.clone();
            assert!(
                check(&send(vec![r])).is_err(),
                "{field} = {value} was accepted"
            );
        }
        let track_cases: Vec<(&str, Value)> = vec![
            ("n", json!(0)),
            ("n", json!(100)),
            ("secs", json!(7201)),
            ("sample", json!("01")),
            ("sample", json!("01-01-01")),
            ("sample", json!("01-1a")),
            ("sample", json!("01-01.mp3")),
            ("side", json!("Side 1")),
            ("title", json!("\u{7}")),
        ];
        for (field, value) in track_cases {
            let mut r = record();
            r["tracks"][0][field] = value.clone();
            assert!(
                check(&send(vec![r])).is_err(),
                "track {field} = {value} was accepted"
            );
        }
        // Optional fields may be missing or empty.
        let mut r = record();
        for k in ["label", "catno", "format", "stock", "price", "image"] {
            r.as_object_mut().unwrap().remove(k);
        }
        r["tracks"][0]["title"] = json!("");
        assert!(check(&send(vec![r])).is_ok());
    }

    #[test]
    fn limits() {
        assert_eq!(
            check(&send(vec![record(); MAX_RECORDS + 1])),
            Err(SendError::TooMany)
        );
        assert!(check(&send(vec![record(); MAX_RECORDS])).is_ok());
        let mut r = record();
        r["tracks"] = json!(vec![r["tracks"][0].clone(); MAX_TRACKS + 1]);
        assert!(check(&send(vec![r])).is_err());
        let mut r = record();
        r["tracks"] = json!(vec![r["tracks"][0].clone(); 50]);
        assert_eq!(
            check(&send(vec![r; 101])),
            Err(SendError::TooMany),
            "5,050 tracks"
        );
    }

    #[test]
    fn the_send_itself_is_checked() {
        assert_eq!(check(&json!([])), Err(SendError::NotJuno));
        assert_eq!(
            check(&json!({ "page": "https://evil.net/", "records": [record()] })),
            Err(SendError::NotJuno)
        );
        assert_eq!(
            check(&json!({ "page": "https://www.juno.co.uk.evil.net/", "records": [record()] })),
            Err(SendError::NotJuno)
        );
        // Regional sites share product ids; other Juno hosts aren't pages.
        let es = "https://es.juno.co.uk/products/x/1-01/";
        assert!(check(&json!({ "page": es, "records": [record()] })).is_ok());
        assert_eq!(
            check(&json!({ "page": "https://mp3.juno.co.uk/", "records": [record()] })),
            Err(SendError::NotJuno)
        );
        assert_eq!(
            check(&json!({ "page": "https://www.juno.co.uk/", "records": [record()], "x": 1 })),
            Err(SendError::NotJuno)
        );
        assert_eq!(
            check(&send(vec![json!({})])),
            Err(SendError::NothingUsable { refused: 1 })
        );
    }

    #[test]
    fn keys_round_trip() {
        let k = key("1178909-01", "02-01");
        assert_eq!(k, "jn.1178909-01-02-01");
        assert_eq!(parse_key(&k), Some(("1178909-01", "02-01")));
        assert_eq!(
            sample_url(&k).as_deref(),
            Some("https://mp3.juno.co.uk/MP3/SF1178909-01-02-01.mp3")
        );
        for bad in [
            "jn.",
            "jn.1178909-01",
            "jn.1178909-01-01",
            "jn.1178909-01-01-01-01",
            "jn.1178909-01-01-0a",
            "jn.x/../1-01-01-01",
            "1178909-01-01-01",
        ] {
            assert_eq!(parse_key(bad), None, "{bad}");
            assert_eq!(sample_url(bad), None, "{bad}");
        }
        assert!(is_key("jn.anything"));
        assert!(!is_key("bc.3020153053"));
        assert!(!is_key("dQw4w9WgXcQ"));
    }

    #[test]
    fn addresses_are_rebuilt_from_checked_parts() {
        assert_eq!(
            product_url("0-mika-vainio-rontgen-vinyl", "1178909-01").as_deref(),
            Some("https://www.juno.co.uk/products/0-mika-vainio-rontgen-vinyl/1178909-01/")
        );
        assert_eq!(product_url("../admin", "1178909-01"), None);
        assert_eq!(
            cover_url("CS1178909-01A-BIG.jpg").as_deref(),
            Some("https://imagescdn.juno.co.uk/full/CS1178909-01A-BIG.jpg")
        );
        assert_eq!(cover_url("../../x.jpg"), None);
        assert!(sample_host("mp3.juno.co.uk"));
        assert!(!sample_host("mp3.juno.co.uk.evil.net"));
    }

    #[test]
    fn pasted_juno_addresses_are_recognised() {
        for t in [
            "https://www.juno.co.uk/products/x/1-01/",
            "http://juno.co.uk",
            " es.juno.co.uk/labels/x ",
        ] {
            assert!(looks_like_juno(t), "{t}");
        }
        for t in [
            "https://www.discogs.com/release/1",
            "https://notjuno.co.uk/",
        ] {
            assert!(!looks_like_juno(t), "{t}");
        }
    }

    #[test]
    fn memory_keeps_links_and_replaces_notes() {
        let dir = crate::test_dir("juno-memory");
        let rec: JunoRecord = serde_json::from_value(record()).unwrap();
        let mut m = JunoMemory::default();
        m.saw(&[rec.to_store()], 1_000);
        m.products.get_mut("1178909-01").unwrap().release = Some(123456);
        m.save(&dir, Store::Juno).unwrap();
        let mut m = JunoMemory::load(&dir, Store::Juno);
        assert_eq!(m.product_of(123456).map(|(p, _)| p), Some("1178909-01"));
        let mut out = rec.clone();
        out.stock = Stock::Out;
        out.price = String::new();
        m.saw(&[out.to_store()], 2_000);
        let s = &m.products["1178909-01"];
        assert_eq!(
            (s.stock, s.release, s.seen),
            (Stock::Out, Some(123456), 2_000)
        );
        assert_eq!(
            JunoMemory::load(&dir.join("nowhere"), Store::Juno),
            JunoMemory::default()
        );
        std::fs::write(JunoMemory::path(&dir, Store::Juno), "not ron").unwrap();
        assert_eq!(JunoMemory::load(&dir, Store::Juno), JunoMemory::default());
    }

    #[test]
    fn the_stock_note() {
        let date = |_| "14 Oct".to_owned();
        let s = Seen {
            stock: Stock::In,
            price: "€14.78".into(),
            seen: 1_000,
            ..Seen::default()
        };
        assert_eq!(
            s.note(1_000, date).as_deref(),
            Some("in stock, €14.78 (seen 14 Oct)")
        );
        assert_eq!(
            s.note(1_000 + STALE_SECS + 1, date).as_deref(),
            Some("in stock, €14.78 (seen 14 Oct, may have changed)")
        );
        assert_eq!(Seen::default().note(0, date), None);
    }
}
