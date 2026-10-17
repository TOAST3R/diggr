//! Records read from a store's page by the browser extension: Juno, Beatport and Traxsource.
//!
//! Each store checks its own send (`crate::juno`, `crate::beatport`, `crate::traxsource`)
//! and turns what passed into [`StoreRecord`]s, so the player merges, links to Discogs and
//! notes prices the same way for all three. What a store's sends taught (stock and price,
//! the Discogs link) is kept per store, in its own file in the cache folder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Where a record was sent from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Store {
    Juno,
    Beatport,
    Traxsource,
}

impl Store {
    pub const ALL: [Store; 3] = [Store::Juno, Store::Beatport, Store::Traxsource];

    pub fn name(self) -> &'static str {
        match self {
            Store::Juno => "Juno",
            Store::Beatport => "Beatport",
            Store::Traxsource => "Traxsource",
        }
    }

    fn file(self) -> &'static str {
        match self {
            Store::Juno => "juno.ron",
            Store::Beatport => "beatport.ron",
            Store::Traxsource => "traxsource.ron",
        }
    }
}

/// One track of a record, checked, with every address it needs already rebuilt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoreTrack {
    /// "1.2", "A1", or empty.
    pub position: String,
    pub artist: String,
    pub title: String,
    /// 0 when unknown.
    pub secs: u32,
    /// Its preview key (`jn.…`, `bp.…`); `None` when the store has no sample the app may
    /// fetch, so its preview is searched for.
    pub clip: Option<String>,
    /// The store's track id and its page (Beatport, Traxsource), to open it.
    pub track: String,
    pub page: String,
    pub bpm: Option<u16>,
    /// A Camelot code ("8A").
    pub key: Option<String>,
}

/// One record as a store's page showed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreRecord {
    pub store: Store,
    /// What the store's notes and Discogs links are kept under: a Juno product, a Beatport
    /// or Traxsource release (or the track, when the page doesn't name its release).
    pub id: String,
    /// The page to open for the record (`page` of the entries).
    pub page: String,
    /// The page name part kept with the note, to open it later.
    pub slug: String,
    pub artist: String,
    pub title: String,
    pub label: String,
    pub catno: String,
    pub format: String,
    pub stock: Stock,
    /// As shown ("€14.78"): only displayed, never read as an amount.
    pub price: String,
    pub cover: String,
    pub tracks: Vec<StoreTrack>,
}

/// What a send is called: "Artist - Title" for one record, else "Juno: 20 records".
pub fn send_name(records: &[StoreRecord]) -> String {
    match records {
        [r] => format!("{} - {}", r.artist, r.title),
        rs => {
            let store = rs.first().map_or("", |r| r.store.name());
            match rs.len() {
                1 => format!("{store}: 1 record"),
                n => format!("{store}: {n} records"),
            }
        }
    }
}

/// Text of `min..=max` characters with no control character.
pub(crate) fn text(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.chars().count()) && !s.chars().any(char::is_control)
}

/// A page name part: 1 to 200 of `a–z 0–9 -`.
pub(crate) fn valid_slug(s: &str) -> bool {
    (1..=200).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// 1 to 10 digits.
pub(crate) fn valid_id(s: &str) -> bool {
    (1..=10).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

// ---- Beatport and Traxsource sends ---------------------------------------------------------

/// The most tracks one Beatport or Traxsource send may hold.
pub const MAX_LISTED: usize = 500;
/// The longest a track may say it is, in seconds.
pub const MAX_SECS: u32 = 7_200;

/// One track as a Beatport or Traxsource page shows it. Beatport names a mix and has a
/// sample; Traxsource names a version and has none the app may fetch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ListedTrack {
    pub id: String,
    pub slug: String,
    pub artists: Vec<String>,
    pub title: String,
    /// Beatport's mix name, or Traxsource's version ("Extended Mix").
    pub mix: String,
    pub remixers: Vec<String>,
    /// Beatport's sample id (a UUID).
    pub sample: String,
    pub bpm: Option<u16>,
    /// As the page writes it: "8A" on Beatport, "A min" on Traxsource.
    pub key: String,
    pub secs: u32,
    pub release: String,
    pub release_name: String,
    pub release_slug: String,
    pub label: String,
    pub catno: String,
    pub price: String,
    /// Beatport's image id (a UUID).
    pub image: String,
}

/// A send's records that passed every check, and how many tracks didn't.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listed {
    pub records: Vec<StoreRecord>,
    pub refused: usize,
}

/// Why a whole Beatport or Traxsource send was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListError {
    Unsupported,
    TooMany,
    NothingUsable { refused: usize },
}

impl std::fmt::Display for ListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListError::Unsupported => write!(f, "unsupported"),
            ListError::TooMany => write!(f, "too many tracks (at most {MAX_LISTED})"),
            ListError::NothingUsable { refused } => {
                write!(f, "no track could be read ({refused} refused)")
            }
        }
    }
}

/// `0f1e2d3c-4b5a-6978-8a9b-0c1d2e3f4a5b`: lowercase hex in 8-4-4-4-12.
pub(crate) fn valid_uuid(s: &str) -> bool {
    let groups: Vec<&str> = s.split('-').collect();
    groups.len() == 5
        && groups.iter().zip([8, 4, 4, 4, 12]).all(|(g, n)| {
            g.len() == n
                && g.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}

fn valid_listed(store: Store, t: &ListedTrack) -> bool {
    let names = |v: &[String], min: usize| {
        (min..=20).contains(&v.len()) && v.iter().all(|a| text(a, 1, 200))
    };
    valid_id(&t.id)
        && valid_slug(&t.slug)
        && names(&t.artists, 1)
        && names(&t.remixers, 0)
        && text(&t.title, 1, 200)
        && text(&t.mix, 0, 200)
        && t.bpm.is_none_or(|b| (40..=250).contains(&b))
        && text(&t.key, 0, 20)
        && t.secs <= MAX_SECS
        && (t.release.is_empty() || valid_id(&t.release))
        && text(&t.release_name, 0, 200)
        && (t.release_slug.is_empty() || valid_slug(&t.release_slug))
        && text(&t.label, 0, 200)
        && text(&t.catno, 0, 40)
        && text(&t.price, 0, 20)
        && match store {
            // Beatport's keys are Camelot codes; Traxsource's are names, kept when they fold.
            Store::Beatport => {
                valid_uuid(&t.sample)
                    && (t.image.is_empty() || valid_uuid(&t.image))
                    && (t.key.is_empty() || key_name(&t.key).is_some())
            }
            _ => t.sample.is_empty() && t.image.is_empty(),
        }
}

/// "Title (Mix)", leaving out a mix named "Original Mix".
fn titled(t: &ListedTrack) -> String {
    match t.mix.trim() {
        "" => t.title.clone(),
        m if m.eq_ignore_ascii_case("original mix") => t.title.clone(),
        m => format!("{} ({m})", t.title),
    }
}

/// Checks a Beatport or Traxsource send (`{ "page": …, "tracks": [ … ] }`, `page` checked by
/// `page_ok`): each track on its own, one that fails (an unknown field included) counted and
/// dropped. What passed is grouped into records by release, in the page's order.
pub(crate) fn check_listed(
    store: Store,
    value: &Value,
    page_ok: fn(&str) -> bool,
    to_record: fn(&ListedTrack) -> (String, String, String, String),
    to_track: fn(&ListedTrack) -> (Option<String>, String),
) -> Result<Listed, ListError> {
    let obj = value.as_object().ok_or(ListError::Unsupported)?;
    if obj.keys().any(|k| k != "page" && k != "tracks") {
        return Err(ListError::Unsupported);
    }
    let page = obj.get("page").and_then(Value::as_str).unwrap_or_default();
    if page.len() > 2048 || !page_ok(page) {
        return Err(ListError::Unsupported);
    }
    let raw = obj
        .get("tracks")
        .and_then(Value::as_array)
        .ok_or(ListError::Unsupported)?;
    if raw.len() > MAX_LISTED {
        return Err(ListError::TooMany);
    }
    let mut out = Listed::default();
    for v in raw {
        let t = match serde_json::from_value::<ListedTrack>(v.clone()) {
            Ok(t) if valid_listed(store, &t) => t,
            _ => {
                out.refused += 1;
                continue;
            }
        };
        let (id, page, slug, cover) = to_record(&t);
        let (clip, track_page) = to_track(&t);
        let key = match store {
            Store::Beatport => key_name(&t.key).map(|_| t.key.clone()),
            _ => camelot(&t.key).map(str::to_owned),
        };
        let track = StoreTrack {
            position: String::new(),
            artist: t.artists.join(", "),
            title: titled(&t),
            secs: t.secs,
            clip,
            track: t.id.clone(),
            page: track_page,
            bpm: t.bpm,
            key,
        };
        match out.records.iter_mut().find(|r| r.id == id) {
            Some(r) => r.tracks.push(track),
            None => out.records.push(StoreRecord {
                store,
                id,
                page,
                slug,
                artist: t.artists.join(", "),
                title: match t.release_name.is_empty() {
                    true => t.title.clone(),
                    false => t.release_name.clone(),
                },
                label: t.label.clone(),
                catno: t.catno.clone(),
                format: String::new(),
                stock: Stock::Unknown,
                price: t.price.clone(),
                cover,
                tracks: vec![track],
            }),
        }
    }
    if out.records.is_empty() {
        return Err(ListError::NothingUsable {
            refused: out.refused,
        });
    }
    Ok(out)
}

/// Whether `text` names an address on `host` or one of its subdomains (pasted addresses of
/// stores read only from the browser are refused with a hint).
pub fn looks_like(text: &str, host: &str) -> bool {
    let t = text.trim().to_ascii_lowercase();
    let t = t
        .strip_prefix("https://")
        .or_else(|| t.strip_prefix("http://"))
        .unwrap_or(&t);
    let h = t.split(['/', '?', '#']).next().unwrap_or_default();
    h == host || h.ends_with(&format!(".{host}"))
}

/// Whether `page` is an https address on `www.host` (only kept to say where a send came
/// from).
pub(crate) fn page_on(page: &str, host: &str) -> bool {
    page.strip_prefix("https://www.")
        .and_then(|rest| rest.strip_prefix(host))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// Camelot codes in wheel order with their key names: `1A` A♭ minor … `12B` E major.
const CAMELOT: [(&str, &str); 24] = [
    ("1A", "A♭ minor"),
    ("2A", "E♭ minor"),
    ("3A", "B♭ minor"),
    ("4A", "F minor"),
    ("5A", "C minor"),
    ("6A", "G minor"),
    ("7A", "D minor"),
    ("8A", "A minor"),
    ("9A", "E minor"),
    ("10A", "B minor"),
    ("11A", "F♯ minor"),
    ("12A", "D♭ minor"),
    ("1B", "B major"),
    ("2B", "F♯ major"),
    ("3B", "D♭ major"),
    ("4B", "A♭ major"),
    ("5B", "E♭ major"),
    ("6B", "B♭ major"),
    ("7B", "F major"),
    ("8B", "C major"),
    ("9B", "G major"),
    ("10B", "D major"),
    ("11B", "A major"),
    ("12B", "E major"),
];

/// "A minor" for "8A"; `None` for anything else.
pub fn key_name(code: &str) -> Option<&'static str> {
    CAMELOT.iter().find(|(c, _)| *c == code).map(|(_, n)| *n)
}

/// The Camelot code of a key as a store writes it: "8A", "A min", "Amin", "F#maj",
/// "Db major", "G♭ minor" (enharmonic spellings fold together); `None` when it isn't one.
pub fn camelot(key: &str) -> Option<&'static str> {
    let k = key.trim();
    if let Some((c, _)) = CAMELOT.iter().find(|(c, _)| c.eq_ignore_ascii_case(k)) {
        return Some(c);
    }
    let k = k.replace('♯', "#").replace('♭', "b");
    let mut chars = k.chars();
    let letter = chars.next()?.to_ascii_uppercase();
    let rest: String = chars.collect();
    let (accidental, mode) = match rest.chars().next() {
        Some('#') => (1, &rest[1..]),
        Some('b') if !rest[1..].trim().is_empty() || rest.len() == 1 => (-1, &rest[1..]),
        _ => (0, rest.as_str()),
    };
    let mode = mode.trim().to_ascii_lowercase();
    let minor = match mode.as_str() {
        "m" | "min" | "minor" => true,
        "" | "maj" | "major" => false,
        _ => return None,
    };
    let base = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let pitch = (base + accidental + 12) % 12;
    // Pitch class (C = 0) of each Camelot number's key, minor (A) and major (B).
    const MINOR: [i32; 12] = [8, 3, 10, 5, 0, 7, 2, 9, 4, 11, 6, 1];
    const MAJOR: [i32; 12] = [11, 6, 1, 8, 3, 10, 5, 0, 7, 2, 9, 4];
    let wheel = if minor { MINOR } else { MAJOR };
    let n = wheel.iter().position(|&p| p == pitch)?;
    Some(CAMELOT[n + if minor { 0 } else { 12 }].0)
}

/// What the page said about stock when it was sent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stock {
    #[default]
    Unknown,
    In,
    Out,
    Preorder,
}

impl Stock {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "" => Stock::Unknown,
            "in" => Stock::In,
            "out" => Stock::Out,
            "preorder" => Stock::Preorder,
            _ => return None,
        })
    }

    /// "in stock", "out of stock", "pre-order"; empty when unknown.
    pub fn text(self) -> &'static str {
        match self {
            Stock::Unknown => "",
            Stock::In => "in stock",
            Stock::Out => "out of stock",
            Stock::Preorder => "pre-order",
        }
    }
}

/// One record as last sent, and its Discogs release once looked up.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Seen {
    /// The record's (or, on Beatport and Traxsource, the track's) page name part, to open it.
    pub slug: String,
    pub stock: Stock,
    pub price: String,
    /// When a send last carried it (Unix seconds).
    pub seen: u64,
    /// Its Discogs release, when exactly one fits.
    pub release: Option<u64>,
    /// Looked up on Discogs already (found or not): never again.
    pub searched: bool,
}

/// What a store's sends taught: per record, the stock and price note and the Discogs link
/// (`juno.ron`, `beatport.ron` or `traxsource.ron` in the cache folder).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StoreMemory {
    pub products: BTreeMap<String, Seen>,
}

/// A note older than this "may have changed".
pub const STALE_SECS: u64 = 30 * 24 * 60 * 60;

impl StoreMemory {
    pub fn path(cache: &Path, store: Store) -> PathBuf {
        cache.join(store.file())
    }

    /// Missing or broken: empty (a broken file never blocks launch).
    pub fn load(cache: &Path, store: Store) -> Self {
        std::fs::read_to_string(Self::path(cache, store))
            .ok()
            .and_then(|t| ron::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, cache: &Path, store: Store) -> std::io::Result<()> {
        let text = ron::ser::to_string(self).map_err(std::io::Error::other)?;
        crate::config::write_atomic(&Self::path(cache, store), text.as_bytes(), false)
    }

    /// A send's records, seen `now`: their stock notes replace the old ones; links stay.
    pub fn saw(&mut self, records: &[StoreRecord], now: u64) {
        for r in records {
            let s = self.products.entry(r.id.clone()).or_default();
            s.slug = r.slug.clone();
            s.stock = r.stock;
            s.price = r.price.clone();
            s.seen = now;
        }
    }

    /// The product linked to `release`, if any (the most recently seen).
    pub fn product_of(&self, release: u64) -> Option<(&str, &Seen)> {
        self.products
            .iter()
            .filter(|(_, s)| s.release == Some(release))
            .max_by_key(|(_, s)| s.seen)
            .map(|(p, s)| (p.as_str(), s))
    }
}

impl Seen {
    /// "in stock, €14.78 (seen 14 Oct)", with "may have changed" once stale (the tooltip
    /// labels it with the store); `None` when nothing about stock or price was seen.
    pub fn note(&self, now: u64, date: impl Fn(u64) -> String) -> Option<String> {
        let stock = self.stock.text();
        if stock.is_empty() && self.price.is_empty() {
            return None;
        }
        let what = match (stock.is_empty(), self.price.is_empty()) {
            (false, false) => format!("{stock}, {}", self.price),
            (false, true) => stock.to_owned(),
            _ => self.price.clone(),
        };
        let stale = match now.saturating_sub(self.seen) > STALE_SECS {
            true => ", may have changed",
            false => "",
        };
        Some(format!("{what} (seen {}{stale})", date(self.seen)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_fold_into_camelot_codes() {
        for (k, c) in [
            ("8A", "8A"),
            ("8a", "8A"),
            ("A min", "8A"),
            ("Amin", "8A"),
            ("A minor", "8A"),
            ("C maj", "8B"),
            ("F#min", "11A"),
            ("Gb minor", "11A"),
            ("G♭ min", "11A"),
            ("Db major", "3B"),
            ("C#maj", "3B"),
            ("Eb min", "2A"),
            ("Bb", "6B"),
            ("E major", "12B"),
        ] {
            assert_eq!(camelot(k), Some(c), "{k}");
        }
        for bad in ["", "H min", "13A", "A dorian", "zz"] {
            assert_eq!(camelot(bad), None, "{bad}");
        }
        assert_eq!(key_name("5A"), Some("C minor"));
        assert_eq!(key_name("8B"), Some("C major"));
        assert_eq!(key_name("x"), None);
    }

    #[test]
    fn each_store_keeps_its_own_file() {
        let dir = crate::test_dir("store-memory");
        let mut m = StoreMemory::default();
        m.products.insert(
            "1".into(),
            Seen {
                price: "$1.69".into(),
                ..Seen::default()
            },
        );
        m.save(&dir, Store::Beatport).unwrap();
        assert_eq!(StoreMemory::load(&dir, Store::Beatport), m);
        assert_eq!(StoreMemory::load(&dir, Store::Juno), StoreMemory::default());
        assert!(dir.join("beatport.ron").exists());
    }
}
