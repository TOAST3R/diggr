//! Records as the dig needs them, read from Discogs' JSON.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A record of a page: a release, or a master release (which carries its own clips).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecordKey {
    Release(u64),
    Master(u64),
}

/// How an artist is credited on a record of their listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Main,
    /// Only the clips whose title names the artist are kept.
    Remix,
}

/// A record as a page's listing knows it, before its details are fetched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Listed {
    pub key: RecordKey,
    pub artist: String,
    pub title: String,
    pub label: String,
    pub catno: String,
    pub year: Option<u16>,
    /// From the listing's formats, when it gives them.
    pub vinyl: Option<bool>,
    pub role: Role,
    /// A small cover image's address, when the listing gives one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cover: String,
}

impl Listed {
    pub fn new(key: RecordKey) -> Self {
        Self {
            key,
            artist: String::new(),
            title: String::new(),
            label: String::new(),
            catno: String::new(),
            year: None,
            vinyl: None,
            role: Role::Main,
            cover: String::new(),
        }
    }
}

/// Copies for sale on the Discogs marketplace, when they were counted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForSale {
    pub count: u32,
    pub lowest: Option<f64>,
    /// ISO code, e.g. "EUR".
    pub currency: String,
    /// Seconds since the Unix epoch.
    pub fetched_at: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub position: String,
    pub title: String,
    /// The track's own artists, joined; empty when they are the release's.
    pub artist: String,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// A validated 11-character video id.
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
}

/// A record's details.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub key: RecordKey,
    /// The release the marketplace numbers are for (a master's main release).
    pub release: Option<u64>,
    pub master: Option<u64>,
    pub artist: String,
    /// Every credited artist's name, for dropping from clip titles.
    pub artist_names: Vec<String>,
    pub title: String,
    pub label: String,
    pub catno: String,
    pub year: Option<u16>,
    pub vinyl: bool,
    pub tracks: Vec<Track>,
    pub clips: Vec<Clip>,
    pub for_sale: Option<ForSale>,
    /// The 150 px thumbnail's address (empty when the record has no image).
    pub cover: String,
    /// Its Discogs styles ("Deep House", "Minimal"), else its genres.
    pub styles: Vec<String>,
}

/// Formats that are records: the listing strings (`12", EP`) and format names.
const VINYL_MARKERS: [&str; 8] = [
    "Vinyl",
    "LP",
    "7\"",
    "10\"",
    "12\"",
    "Flexi-disc",
    "Lathe Cut",
    "Acetate",
];

/// From a listing's format string, such as `12", EP` or `CD, Album`.
pub fn vinyl_from_format(format: &str) -> bool {
    format
        .split([',', ' '])
        .map(str::trim)
        .any(|part| VINYL_MARKERS.iter().any(|m| part.eq_ignore_ascii_case(m)))
        || format.contains("Lathe Cut")
}

/// From a `formats` array (`[{name: "Vinyl", descriptions: [...]}]`).
pub fn vinyl_from_formats(formats: &Value) -> Option<bool> {
    let formats = formats.as_array()?;
    Some(formats.iter().any(|f| {
        f["name"]
            .as_str()
            .is_some_and(|n| VINYL_MARKERS.iter().any(|m| n.eq_ignore_ascii_case(m)))
    }))
}

/// `Nightcraft (2)` → `Nightcraft`.
pub fn clean_artist(name: &str) -> String {
    let n = name.trim();
    if let Some(open) = n.rfind(" (")
        && n.ends_with(')')
        && n[open + 2..n.len() - 1].bytes().all(|b| b.is_ascii_digit())
        && open + 3 < n.len()
    {
        return n[..open].to_owned();
    }
    n.to_owned()
}

/// Joins an `artists` array as Discogs shows it, using each entry's `join` ("&", "Feat.", ",").
pub fn join_artists(artists: &Value) -> String {
    let Some(list) = artists.as_array() else {
        return String::new();
    };
    let mut out = String::new();
    for (i, a) in list.iter().enumerate() {
        out.push_str(&clean_artist(a["name"].as_str().unwrap_or("")));
        if i + 1 < list.len() {
            match a["join"].as_str().map(str::trim).unwrap_or("") {
                "" | "," => out.push_str(", "),
                j => {
                    out.push(' ');
                    out.push_str(j);
                    out.push(' ');
                }
            }
        }
    }
    out
}

pub fn artist_names(artists: &Value) -> Vec<String> {
    artists
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|a| a["name"].as_str().map(clean_artist))
                .collect()
        })
        .unwrap_or_default()
}

/// `6:12` or `1:02:03` in seconds.
pub fn parse_duration(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    s.split(':').try_fold(0.0, |acc, p| {
        Some(acc * 60.0 + p.trim().parse::<f64>().ok()?)
    })
}

/// A tracklist, flattening index tracks' sub-tracks and skipping headings.
pub fn tracks(tracklist: &Value) -> Vec<Track> {
    let mut out = Vec::new();
    for t in tracklist.as_array().into_iter().flatten() {
        match t["type_"].as_str().unwrap_or("track") {
            "heading" => {}
            "index" => out.extend(tracks(&t["sub_tracks"])),
            _ => out.push(Track {
                position: t["position"].as_str().unwrap_or("").trim().to_owned(),
                title: t["title"].as_str().unwrap_or("").trim().to_owned(),
                artist: join_artists(&t["artists"]),
                duration: t["duration"].as_str().and_then(parse_duration),
            }),
        }
    }
    out
}

/// The usable clips of a `videos` array: YouTube ones with a valid id, each once.
pub fn clips(videos: &Value) -> Vec<Clip> {
    let mut out: Vec<Clip> = Vec::new();
    for v in videos.as_array().into_iter().flatten() {
        let Some(id) = v["uri"].as_str().and_then(clip_id) else {
            continue;
        };
        if out.iter().any(|c| c.id == id) {
            continue;
        }
        out.push(Clip {
            id,
            title: v["title"].as_str().unwrap_or("").trim().to_owned(),
            duration: v["duration"].as_f64().filter(|d| *d > 0.0),
        });
    }
    out
}

/// The video id of a YouTube address: `youtube.com/watch?v=ID`, `m.youtube.com/watch?v=ID`,
/// `youtu.be/ID` or `youtube.com/embed/ID`. Anything that isn't exactly 11 letters, digits,
/// `-` or `_` is refused: the id ends up as an argument to yt-dlp.
pub fn clip_id(uri: &str) -> Option<String> {
    let rest = uri
        .trim()
        .strip_prefix("https://")
        .or_else(|| uri.trim().strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/')?;
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let id = match host {
        "youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            if let Some(q) = path.strip_prefix("watch?") {
                q.split(['&', '#']).find_map(|kv| kv.strip_prefix("v="))?
            } else {
                path.strip_prefix("embed/")?
                    .split(['?', '&', '#', '/'])
                    .next()?
            }
        }
        "youtu.be" => path.split(['?', '&', '#', '/']).next()?,
        _ => return None,
    };
    valid_clip_id(id).then(|| id.to_owned())
}

/// Exactly 11 of `A–Z a–z 0–9 - _`.
pub fn valid_clip_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn first_label(labels: &Value) -> (String, String) {
    let l = &labels[0];
    (
        l["name"].as_str().map(clean_artist).unwrap_or_default(),
        l["catno"].as_str().unwrap_or("").trim().to_owned(),
    )
}

/// The primary image's 150 px thumbnail from an `images` array, else the first image's.
pub fn cover(images: &Value) -> String {
    let list = images.as_array().map(Vec::as_slice).unwrap_or_default();
    list.iter()
        .find(|i| i["type"].as_str() == Some("primary"))
        .or_else(|| list.first())
        .and_then(|i| i["uri150"].as_str())
        .map(thumb)
        .unwrap_or_default()
}

/// A thumbnail address, or nothing for an empty or non-https one (Discogs' "spacer" images
/// for records without a picture are left out too).
fn thumb(uri: &str) -> String {
    let uri = uri.trim();
    if uri.starts_with("https://") && !uri.contains("spacer.gif") {
        uri.to_owned()
    } else {
        String::new()
    }
}

/// A record's styles, or its genres when it lists no style.
fn styles(v: &Value) -> Vec<String> {
    let names = |a: &Value| -> Vec<String> {
        a.as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s.as_str())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let s = names(&v["styles"]);
    if s.is_empty() { names(&v["genres"]) } else { s }
}

fn year(v: &Value) -> Option<u16> {
    v.as_u64()
        .or_else(|| v.as_str().and_then(|s| s.get(..4)?.parse().ok()))
        .filter(|&y| y > 0)
        .and_then(|y| u16::try_from(y).ok())
}

impl Record {
    /// From `GET /releases/{id}`; prices are in `currency`.
    pub fn from_release(v: &Value, currency: &str, fetched_at: u64) -> Option<Self> {
        let id = v["id"].as_u64()?;
        let (label, catno) = first_label(&v["labels"]);
        Some(Record {
            key: RecordKey::Release(id),
            release: Some(id),
            master: v["master_id"].as_u64().filter(|&m| m > 0),
            artist: join_artists(&v["artists"]),
            artist_names: artist_names(&v["artists"]),
            title: v["title"].as_str().unwrap_or("").trim().to_owned(),
            label,
            catno,
            year: year(&v["year"]).or_else(|| year(&v["released"])),
            vinyl: vinyl_from_formats(&v["formats"]).unwrap_or(false),
            tracks: tracks(&v["tracklist"]),
            clips: clips(&v["videos"]),
            for_sale: Some(ForSale {
                count: v["num_for_sale"].as_u64().unwrap_or(0) as u32,
                lowest: v["lowest_price"].as_f64(),
                currency: currency.to_owned(),
                fetched_at,
            }),
            cover: cover(&v["images"]),
            styles: styles(v),
        })
    }

    /// From `GET /masters/{id}`, with its main release for the label, catalog number and
    /// marketplace numbers.
    pub fn from_master(v: &Value, main: Option<&Record>) -> Option<Self> {
        let id = v["id"].as_u64()?;
        Some(Record {
            key: RecordKey::Master(id),
            release: main.and_then(|m| m.release),
            master: Some(id),
            artist: join_artists(&v["artists"]),
            artist_names: artist_names(&v["artists"]),
            title: v["title"].as_str().unwrap_or("").trim().to_owned(),
            label: main.map(|m| m.label.clone()).unwrap_or_default(),
            catno: main.map(|m| m.catno.clone()).unwrap_or_default(),
            year: year(&v["year"]).or_else(|| main.and_then(|m| m.year)),
            vinyl: main.is_some_and(|m| m.vinyl),
            tracks: tracks(&v["tracklist"]),
            clips: clips(&v["videos"]),
            for_sale: main.and_then(|m| m.for_sale.clone()),
            cover: Some(cover(&v["images"]))
                .filter(|c| !c.is_empty())
                .or_else(|| main.map(|m| m.cover.clone()))
                .unwrap_or_default(),
            styles: Some(styles(v))
                .filter(|s| !s.is_empty())
                .or_else(|| main.map(|m| m.styles.clone()))
                .unwrap_or_default(),
        })
    }
}

/// A label or artist listing item (`/labels/{id}/releases`, `/artists/{id}/releases`).
pub fn listed_from_listing(item: &Value) -> Option<Listed> {
    let id = item["id"].as_u64()?;
    let key = match item["type"].as_str() {
        Some("master") => RecordKey::Master(id),
        _ => RecordKey::Release(id),
    };
    Some(Listed {
        key,
        artist: clean_artist(item["artist"].as_str().unwrap_or("")),
        title: item["title"].as_str().unwrap_or("").trim().to_owned(),
        label: item["label"].as_str().unwrap_or("").trim().to_owned(),
        catno: item["catno"].as_str().unwrap_or("").trim().to_owned(),
        year: year(&item["year"]),
        vinyl: item["format"].as_str().map(vinyl_from_format),
        role: match item["role"].as_str() {
            Some("Remix") => Role::Remix,
            _ => Role::Main,
        },
        cover: thumb(item["thumb"].as_str().unwrap_or("")),
    })
}

/// A wantlist item (`/users/{name}/wants`).
pub fn listed_from_want(item: &Value) -> Option<Listed> {
    let b = &item["basic_information"];
    let id = item["id"].as_u64().or_else(|| b["id"].as_u64())?;
    let (label, catno) = first_label(&b["labels"]);
    Some(Listed {
        key: RecordKey::Release(id),
        artist: join_artists(&b["artists"]),
        title: b["title"].as_str().unwrap_or("").trim().to_owned(),
        label,
        catno,
        year: year(&b["year"]),
        vinyl: vinyl_from_formats(&b["formats"]),
        role: Role::Main,
        cover: thumb(b["thumb"].as_str().unwrap_or("")),
    })
}

/// A list item (`/lists/{id}`): releases and masters only.
pub fn listed_from_list_item(item: &Value) -> Option<Listed> {
    let id = item["id"].as_u64()?;
    let key = match item["type"].as_str()? {
        "release" => RecordKey::Release(id),
        "master" => RecordKey::Master(id),
        _ => return None,
    };
    let mut l = Listed::new(key);
    let display = item["display_title"].as_str().unwrap_or("");
    match display.split_once(" - ") {
        Some((a, t)) => {
            l.artist = clean_artist(a);
            l.title = t.trim().to_owned();
        }
        None => l.title = display.trim().to_owned(),
    }
    l.cover = thumb(item["image_url"].as_str().unwrap_or(""));
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn clip_ids_from_every_address_form() {
        let ok = [
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://youtube.com/watch?feature=share&v=dQw4w9WgXcQ",
            "https://m.youtube.com/watch?v=dQw4w9WgXcQ&t=30",
            "https://youtu.be/dQw4w9WgXcQ?t=5",
            "http://www.youtube.com/embed/dQw4w9WgXcQ",
        ];
        for uri in ok {
            assert_eq!(clip_id(uri).as_deref(), Some("dQw4w9WgXcQ"), "{uri}");
        }
        let bad = [
            "https://www.youtube.com/watch?v=abc;rm -rf",
            "https://www.youtube.com/watch?v=dQw4w9WgXc",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQQ",
            "https://www.youtube.com/watch?v=--rm-rf-%2F",
            "https://vimeo.com/123456",
            "https://evil.com/youtube.com/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.evil.com/watch?v=dQw4w9WgXcQ",
            "https://www.youtube.com/playlist?list=PL123",
            "javascript:alert(1)",
        ];
        for uri in bad {
            assert_eq!(clip_id(uri), None, "{uri}");
        }
        assert!(valid_clip_id("a-b_c1234XY"));
        assert!(!valid_clip_id("abc;rm -rf "));
    }

    #[test]
    fn vinyl_from_listings_and_details() {
        for f in [
            "12\", EP",
            "LP, Album",
            "Vinyl, 7\", Single",
            "Lathe Cut, 10\"",
        ] {
            assert!(vinyl_from_format(f), "{f}");
        }
        for f in ["CD, Album", "File, MP3", "Cass, Album", ""] {
            assert!(!vinyl_from_format(f), "{f}");
        }
        assert_eq!(
            vinyl_from_formats(&json!([{"name":"CD"},{"name":"Vinyl"}])),
            Some(true)
        );
        assert_eq!(vinyl_from_formats(&json!([{"name":"CD"}])), Some(false));
        assert_eq!(vinyl_from_formats(&Value::Null), None);
    }

    #[test]
    fn artists_are_joined_as_discogs_shows_them() {
        let a = json!([
            {"name": "Nightcraft (2)", "join": "&"},
            {"name": "Lumen", "join": ","},
            {"name": "Ohm (12)", "join": ""}
        ]);
        assert_eq!(join_artists(&a), "Nightcraft & Lumen, Ohm");
        assert_eq!(artist_names(&a), ["Nightcraft", "Lumen", "Ohm"]);
        assert_eq!(clean_artist("Kraftwerk"), "Kraftwerk");
        assert_eq!(clean_artist("Room (4)"), "Room");
        assert_eq!(clean_artist("Phase (III)"), "Phase (III)");
    }

    #[test]
    fn a_release_reads_its_tracks_clips_and_for_sale_numbers() {
        let v = json!({
            "id": 123456, "master_id": 777, "title": "Glasshouse", "year": 1994,
            "artists": [{"name": "Nightcraft", "join": ""}],
            "labels": [{"name": "Lowtide Tapes", "catno": "LT-012"}],
            "formats": [{"name": "Vinyl", "descriptions": ["12\""]}],
            "tracklist": [
                {"position": "", "type_": "heading", "title": "Side A"},
                {"position": "A1", "type_": "track", "title": "Glasshouse", "duration": "6:12"},
                {"position": "A2", "type_": "track", "title": "Tidal",
                 "artists": [{"name": "Lumen (3)", "join": ""}]},
                {"type_": "index", "title": "Suite", "sub_tracks": [
                    {"position": "B1a", "type_": "track", "title": "Part I"}
                ]}
            ],
            "videos": [
                {"uri": "https://www.youtube.com/watch?v=aaaaaaaaaaa", "title": "Nightcraft - Glasshouse", "duration": 373},
                {"uri": "https://www.youtube.com/watch?v=aaaaaaaaaaa", "title": "dupe"},
                {"uri": "https://www.youtube.com/watch?v=bad", "title": "bad id"}
            ],
            "num_for_sale": 6, "lowest_price": 9.0
        });
        let r = Record::from_release(&v, "EUR", 42).unwrap();
        assert_eq!(
            (r.release, r.master, r.year),
            (Some(123456), Some(777), Some(1994))
        );
        assert_eq!(
            (r.label.as_str(), r.catno.as_str()),
            ("Lowtide Tapes", "LT-012")
        );
        assert!(r.vinyl);
        let pos: Vec<&str> = r.tracks.iter().map(|t| t.position.as_str()).collect();
        assert_eq!(pos, ["A1", "A2", "B1a"]);
        assert_eq!(r.tracks[0].duration, Some(372.0));
        assert_eq!(r.tracks[1].artist, "Lumen");
        assert_eq!(r.clips.len(), 1);
        assert_eq!(r.clips[0].duration, Some(373.0));
        let fs = r.for_sale.unwrap();
        assert_eq!(
            (fs.count, fs.lowest, fs.currency.as_str(), fs.fetched_at),
            (6, Some(9.0), "EUR", 42)
        );
    }

    #[test]
    fn covers_prefer_the_primary_image_and_fall_back() {
        let images = json!([
            {"type": "secondary", "uri150": "https://i.discogs.com/back.jpg"},
            {"type": "primary", "uri150": "https://i.discogs.com/front.jpg"}
        ]);
        assert_eq!(cover(&images), "https://i.discogs.com/front.jpg");
        let only_secondary =
            json!([{"type": "secondary", "uri150": "https://i.discogs.com/b.jpg"}]);
        assert_eq!(cover(&only_secondary), "https://i.discogs.com/b.jpg");
        assert_eq!(cover(&Value::Null), "");
        assert_eq!(cover(&json!([{"uri150": "http://insecure/x.jpg"}])), "");

        // A master without images takes its main release's cover.
        let release = json!({"id": 1, "title": "EP", "images": images});
        let main = Record::from_release(&release, "EUR", 0).unwrap();
        let bare = json!({"id": 9, "title": "EP"});
        let m = Record::from_master(&bare, Some(&main)).unwrap();
        assert_eq!(m.cover, "https://i.discogs.com/front.jpg");
        let own =
            json!({"id": 9, "title": "EP", "images": [{"uri150": "https://i.discogs.com/m.jpg"}]});
        assert_eq!(
            Record::from_master(&own, Some(&main)).unwrap().cover,
            "https://i.discogs.com/m.jpg"
        );
    }

    #[test]
    fn listings_wants_and_lists_carry_thumbnails() {
        let item = json!({"id": 5, "title": "T", "thumb": "https://i.discogs.com/t.jpg"});
        assert_eq!(
            listed_from_listing(&item).unwrap().cover,
            "https://i.discogs.com/t.jpg"
        );
        let spacer = json!({"id": 5, "thumb": "https://st.discogs.com/images/spacer.gif"});
        assert_eq!(listed_from_listing(&spacer).unwrap().cover, "");
        let want = json!({"id": 6, "basic_information": {"title": "W", "thumb": "https://i.discogs.com/w.jpg"}});
        assert_eq!(
            listed_from_want(&want).unwrap().cover,
            "https://i.discogs.com/w.jpg"
        );
        let list = json!({"id": 7, "type": "release", "display_title": "A - B", "image_url": "https://i.discogs.com/l.jpg"});
        assert_eq!(
            listed_from_list_item(&list).unwrap().cover,
            "https://i.discogs.com/l.jpg"
        );
    }

    #[test]
    fn styles_come_from_the_record_else_its_genres() {
        let r = Record::from_release(
            &serde_json::json!({"id": 1, "styles": ["Deep House", " Minimal "], "genres": ["Electronic"]}),
            "EUR",
            0,
        )
        .unwrap();
        assert_eq!(r.styles, ["Deep House", "Minimal"]);
        let g = Record::from_release(
            &serde_json::json!({"id": 2, "genres": ["Electronic"]}),
            "EUR",
            0,
        )
        .unwrap();
        assert_eq!(g.styles, ["Electronic"], "no style: the genres");
        let m = Record::from_master(&serde_json::json!({"id": 9}), Some(&r)).unwrap();
        assert_eq!(
            m.styles, r.styles,
            "a master without styles takes its main release's"
        );
    }
}
