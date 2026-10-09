//! Bandcamp pages: a label's (or artist's), an album or a track, sent like a Discogs page.
//!
//! Bandcamp is read through the user's yt-dlp (it keeps up with Bandcamp's pages, and the
//! app already needs it), never with a scraper of our own. The address is checked strictly
//! and rebuilt from its parts before yt-dlp sees it, after `--`, so nothing the user pastes
//! or a page links to reaches it unchanged; results are kept only when their own addresses
//! and ids pass the same checks.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::discogs::url::Refused;

/// What `‹name›.bandcamp.com` holds at a path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BandcampKind {
    /// The front page or `/music`: every album and track.
    Label,
    Album(String),
    Track(String),
}

/// A checked Bandcamp page.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BandcampPage {
    /// The subdomain: `analogicalforce` for `analogicalforce.bandcamp.com`.
    pub name: String,
    pub kind: BandcampKind,
}

pub const SUPPORTED: &str =
    "Supported Bandcamp pages: a label or artist (‹name›.bandcamp.com), an album or a track";

/// The longest page title used to name a label.
pub const MAX_TITLE: usize = 200;

fn valid_name(s: &str) -> bool {
    (1..=63).contains(&s.len())
        && s.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn valid_slug(s: &str) -> bool {
    (1..=200).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A track id as Bandcamp gives it: 1 to 20 digits.
pub fn valid_track_id(id: &str) -> bool {
    (1..=20).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit())
}

/// Checks a Bandcamp address. Text that isn't a `bandcamp.com` address at all is
/// [`Refused::NotDiscogs`] (a paste of other text is ignored); a `bandcamp.com` address that
/// isn't a label, album or track is [`Refused::Unsupported`].
pub fn parse(text: &str) -> Result<BandcampPage, Refused> {
    let text = text.trim();
    if text.contains(char::is_whitespace) {
        return Err(Refused::NotDiscogs);
    }
    let rest = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))
        .ok_or(Refused::NotDiscogs)?;
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.to_ascii_lowercase();
    let Some(name) = host.strip_suffix(".bandcamp.com") else {
        return Err(Refused::NotDiscogs);
    };
    if !valid_name(name) {
        return Err(Refused::Unsupported);
    }
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let kind = match parts.as_slice() {
        [] | ["music"] => BandcampKind::Label,
        ["album", slug] if valid_slug(slug) => BandcampKind::Album((*slug).to_owned()),
        ["track", slug] if valid_slug(slug) => BandcampKind::Track((*slug).to_owned()),
        _ => return Err(Refused::Unsupported),
    };
    Ok(BandcampPage {
        name: name.to_owned(),
        kind,
    })
}

impl BandcampPage {
    /// The label's front page.
    pub fn root(&self) -> String {
        format!("https://{}.bandcamp.com", self.name)
    }

    /// The address rebuilt from its checked parts: the only form yt-dlp is given.
    pub fn url(&self) -> String {
        match &self.kind {
            BandcampKind::Label => format!("{}/music", self.root()),
            BandcampKind::Album(s) => format!("{}/album/{s}", self.root()),
            BandcampKind::Track(s) => format!("{}/track/{s}", self.root()),
        }
    }

    /// A name before Bandcamp gives one: the slug with its dashes as spaces.
    pub fn provisional_name(&self) -> String {
        match &self.kind {
            BandcampKind::Label => format!("Label: {}", self.name),
            BandcampKind::Album(s) | BandcampKind::Track(s) => s.replace('-', " "),
        }
    }
}

/// The label's name: from the page's title the extension sent ("Music | Analogical Force"),
/// the part after the last " | ", else the subdomain.
pub fn label_name(title: Option<&str>, page: &BandcampPage) -> String {
    title
        .map(|t| {
            let t: String = t
                .chars()
                .filter(|c| !c.is_control())
                .take(MAX_TITLE)
                .collect();
            t.rsplit(" | ").next().unwrap_or_default().trim().to_owned()
        })
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| page.name.clone())
}

/// Bracketed words that only say the format: "[Vinyl]", "(EP)".
const FORMAT_WORDS: [&str; 8] = [
    "vinyl", "ep", "lp", "12\"", "digital", "cd", "cassette", "single",
];

/// A catalogue number in brackets, leading or trailing: "[AF070] The Ooze EP" → ("AF070",
/// "The Ooze EP"), "Advance [TOBAS 006]" → ("TOBAS 006", "Advance"). A bracket counts when it
/// holds a digit; one that only says the format ("[Vinyl]") is dropped. Without one, no
/// catalogue number and the whole title.
pub fn catno_and_album(title: &str) -> (String, String) {
    let is_catno = |c: &str| {
        let c = c.trim();
        !c.is_empty() && c.len() <= 30 && c.chars().any(|ch| ch.is_ascii_digit())
    };
    let is_format = |c: &str| FORMAT_WORDS.contains(&c.trim().to_lowercase().as_str());
    let mut t = title.trim().to_owned();
    let mut catno = String::new();
    if let Some(rest) = t.strip_prefix('[')
        && let Some((c, album)) = rest.split_once(']')
        && is_catno(c)
    {
        catno = c.trim().to_owned();
        t = album.trim().to_owned();
    }
    // Trailing brackets: a catalogue number (when none led), or format words.
    while let Some(open) = t.rfind(['[', '('])
        && (t.ends_with(']') || t.ends_with(')'))
    {
        let inner = &t[open + 1..t.len() - 1];
        if catno.is_empty() && t.ends_with(']') && is_catno(inner) {
            catno = inner.trim().to_owned();
        } else if !is_format(inner) {
            break;
        }
        t = t[..open].trim_end().to_owned();
    }
    (catno, t)
}

/// One track as Bandcamp lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct BandcampTrack {
    /// Digits only (see [`valid_track_id`]).
    pub track_id: String,
    /// The track's page, checked and rebuilt.
    pub url: String,
    pub artist: String,
    pub title: String,
    pub duration: Option<f64>,
    /// Bandcamp streams it (a pre-order track often isn't).
    pub streamable: bool,
}

/// An album (or a single track page, as an album of one).
#[derive(Debug, Clone, PartialEq)]
pub struct BandcampAlbum {
    /// The album's page (a track page's own address for a single track).
    pub url: String,
    /// Without its catalogue number.
    pub title: String,
    pub catno: String,
    pub artist: String,
    pub cover: String,
    pub year: Option<u16>,
    pub tracks: Vec<BandcampTrack>,
}

/// What reading a page gave.
#[derive(Debug, Clone, PartialEq)]
pub enum Listing {
    /// A label's albums and tracks, as checked addresses, in the page's order.
    Albums(Vec<String>),
    Album(BandcampAlbum),
}

/// The argument list that reads `page` without downloading anything: a label's albums are
/// only listed (`--flat-playlist`), an album or track gives every track's details.
pub fn read_args(page: &BandcampPage) -> Vec<String> {
    let mut args: Vec<String> = vec!["--ignore-config".into(), "--no-warnings".into()];
    if page.kind == BandcampKind::Label {
        args.push("--flat-playlist".into());
    }
    args.extend(["-J".into(), "--".into(), page.url()]);
    args
}

fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().trim().to_owned()
}

/// A checked page address from yt-dlp's answer, rebuilt.
fn checked(url: &str) -> Option<BandcampPage> {
    parse(url).ok()
}

fn track(v: &Value) -> Option<BandcampTrack> {
    let id = match &v["track_id"] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => text(v, "id"),
    };
    let page = checked(v["webpage_url"].as_str()?)?;
    if !valid_track_id(&id) || !matches!(page.kind, BandcampKind::Track(_)) {
        return None;
    }
    let artist = text(v, "artist");
    let mut title = text(v, "track");
    if title.is_empty() {
        title = text(v, "title");
        if let Some(t) = title.strip_prefix(&format!("{artist} - ")) {
            title = t.to_owned();
        }
    }
    let streamable = v["url"].as_str().is_some_and(|u| !u.is_empty())
        || v["formats"].as_array().is_some_and(|f| !f.is_empty());
    Some(BandcampTrack {
        track_id: id,
        url: page.url(),
        artist,
        title,
        duration: v["duration"].as_f64().filter(|d| *d > 0.0),
        streamable,
    })
}

/// yt-dlp's JSON for a page back to a listing; entries that fail the checks are left out.
pub fn parse_listing(page: &BandcampPage, json: &str) -> Result<Listing, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("unreadable answer: {e}"))?;
    if page.kind == BandcampKind::Label {
        let mut urls: Vec<String> = Vec::new();
        for e in v["entries"].as_array().into_iter().flatten() {
            if let Some(p) = e["url"].as_str().and_then(checked)
                && p.kind != BandcampKind::Label
                && p.name == page.name
                && !urls.contains(&p.url())
            {
                urls.push(p.url());
            }
        }
        return Ok(Listing::Albums(urls));
    }
    if !v.is_object() {
        return Err("yt-dlp gave nothing for this page".into());
    }
    let entries: Vec<&Value> = match v["entries"].as_array() {
        // An entry yt-dlp couldn't read is `null`.
        Some(list) => list.iter().filter(|e| e.is_object()).collect(),
        None => vec![&v],
    };
    let first = entries.first().copied().unwrap_or(&v);
    let album_title = match text(&v, "_type").as_str() {
        "playlist" => text(&v, "title"),
        _ => text(first, "album"),
    };
    let (catno, title) = catno_and_album(&album_title);
    // The account's own name: what yt-dlp credits every track to when the account (a label's)
    // doesn't name each release's artist.
    let account = [text(first, "uploader"), text(first, "album_artist")]
        .into_iter()
        .find(|a| !a.is_empty())
        .unwrap_or_default();
    let mut artist = text(first, "album_artist");
    if artist.is_empty() {
        artist = text(first, "artist");
    }
    // A label's account names the artist in the album's title: "Gioele Menoni - Mental Roots".
    // yt-dlp can't tell a label's account from an artist's, so a title credited to the account
    // itself is read that way whenever it has a dash.
    let (title, named) = match title.split_once(" - ") {
        Some((a, t)) if artist == account && !a.trim().is_empty() && !t.trim().is_empty() => {
            (t.trim().to_owned(), Some(a.trim().to_owned()))
        }
        _ => (title, None),
    };
    if let Some(a) = &named {
        artist = a.clone();
    }
    let tracks = entries
        .into_iter()
        .filter_map(track)
        .map(|mut t| {
            if t.artist.is_empty() || t.artist == account {
                // A compilation: "Artist - Track"; else the album's artist.
                match t.title.split_once(" - ") {
                    Some((a, rest)) if named.is_some() || t.artist.is_empty() => {
                        t.artist = a.trim().to_owned();
                        t.title = rest.trim().to_owned();
                    }
                    _ => {
                        if let Some(a) = &named {
                            t.artist = a.clone();
                        }
                    }
                }
            }
            t
        })
        .collect();
    Ok(Listing::Album(BandcampAlbum {
        url: page.url(),
        title,
        catno,
        artist,
        cover: text(first, "thumbnail"),
        year: first["release_year"]
            .as_u64()
            .and_then(|y| u16::try_from(y).ok()),
        tracks,
    }))
}

/// A label's name for comparing: folded, without trailing "records", "music"… (however
/// many), and without spaces, so the subdomain "analogicalforce" is "Analogical Force".
pub fn name_key(name: &str) -> String {
    const TRAILING: [&str; 6] = ["records", "recordings", "music", "label", "rec", "ltd"];
    let name = name.strip_prefix("Label: ").unwrap_or(name);
    let mut words: Vec<String> = crate::discogs::matching::fold(name)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    while words.len() > 1 && words.last().is_some_and(|w| TRAILING.contains(&w.as_str())) {
        words.pop();
    }
    let mut joined = words.concat();
    // A subdomain has no spaces: its trailing words are cut from the end of the text.
    loop {
        let before = joined.len();
        for t in TRAILING {
            if let Some(rest) = joined.strip_suffix(t)
                && rest.len() >= 3
            {
                joined = rest.to_owned();
            }
        }
        if joined.len() == before {
            break;
        }
    }
    joined
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameMatch {
    Same,
    Close,
    Different,
}

/// Levenshtein distance, for short names.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = if ca == *cb {
                prev
            } else {
                1 + prev.min(row[j]).min(cur)
            };
            prev = cur;
        }
    }
    row[b.len()]
}

/// Whether two label names are the same label, close (ask), or different.
pub fn name_match(a: &str, b: &str) -> NameMatch {
    let (a, b) = (name_key(a), name_key(b));
    if a.is_empty() || b.is_empty() {
        return NameMatch::Different;
    }
    if a == b {
        NameMatch::Same
    } else if a.contains(&b) || b.contains(&a) || distance(&a, &b) <= 2 {
        NameMatch::Close
    } else {
        NameMatch::Different
    }
}

/// What the app remembers about Bandcamp (`dig/bandcamp.ron`): the albums each label crate
/// has read, so Refresh label reads only new ones, and the labels the user kept apart.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BandcampMemory {
    /// Crate id → the album and track pages read into it.
    pub albums: BTreeMap<u64, BTreeSet<String>>,
    /// (Bandcamp name, crate id): "Separate" was answered, so they are never asked again.
    pub separate: BTreeSet<(String, u64)>,
}

impl BandcampMemory {
    const FILE: &str = "bandcamp.ron";

    pub fn path(config: &Path) -> PathBuf {
        crate::config::dir(config).join(Self::FILE)
    }

    /// Missing or broken: empty (a broken file never blocks launch).
    pub fn load(config: &Path) -> Self {
        std::fs::read_to_string(Self::path(config))
            .ok()
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, config: &Path) -> std::io::Result<()> {
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(std::io::Error::other)?;
        crate::config::write_atomic(&Self::path(config), text.as_bytes(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALBUM: &str = include_str!("../tests/fixtures/bandcamp/album.json");
    const TRACK: &str = include_str!("../tests/fixtures/bandcamp/track.json");
    const LABEL: &str = include_str!("../tests/fixtures/bandcamp/label.json");

    fn page(name: &str, kind: BandcampKind) -> BandcampPage {
        BandcampPage {
            name: name.into(),
            kind,
        }
    }

    #[test]
    fn accepted_addresses() {
        let album = page(
            "analogicalforce",
            BandcampKind::Album("af070-the-ooze-ep".into()),
        );
        for text in [
            "https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep",
            "https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep?from=search#x",
            "https://AnalogicalForce.Bandcamp.com/album/af070-the-ooze-ep/",
            "  http://analogicalforce.bandcamp.com/album/af070-the-ooze-ep ",
        ] {
            assert_eq!(parse(text), Ok(album.clone()), "{text}");
        }
        let label = page("analogicalforce", BandcampKind::Label);
        assert_eq!(
            parse("https://analogicalforce.bandcamp.com"),
            Ok(label.clone())
        );
        assert_eq!(
            parse("https://analogicalforce.bandcamp.com/music"),
            Ok(label.clone())
        );
        assert_eq!(label.url(), "https://analogicalforce.bandcamp.com/music");
        assert_eq!(
            parse("https://analogicalforce.bandcamp.com/track/the-ooze").map(|p| p.url()),
            Ok("https://analogicalforce.bandcamp.com/track/the-ooze".into())
        );
    }

    #[test]
    fn refused_addresses() {
        for text in [
            "https://analogicalforce.bandcamp.com/merch",
            "https://analogicalforce.bandcamp.com/album/x;rm",
            "https://analogicalforce.bandcamp.com/album/X-Upper",
            "https://analogicalforce.bandcamp.com/album/a/b",
            "https://-bad.bandcamp.com/album/x",
            "https://a_b.bandcamp.com/album/x",
        ] {
            assert_eq!(parse(text), Err(Refused::Unsupported), "{text}");
        }
        for text in [
            "https://bandcamp.com.evil.net/album/x",
            "https://evil.bandcamp.com.attacker.net/album/x",
            "https://bandcamp.com/album/x",
            "analogicalforce.bandcamp.com/album/x",
            "ftp://analogicalforce.bandcamp.com/album/x",
            "hello there",
            "https://www.discogs.com/release/1",
        ] {
            assert_eq!(parse(text), Err(Refused::NotDiscogs), "{text}");
        }
    }

    #[test]
    fn catalogue_numbers_in_album_titles() {
        assert_eq!(
            catno_and_album("[AF070] The Ooze EP"),
            ("AF070".into(), "The Ooze EP".into())
        );
        assert_eq!(
            catno_and_album(" [ AF 070 ]  The Ooze EP "),
            ("AF 070".into(), "The Ooze EP".into())
        );
        assert_eq!(
            catno_and_album("The Ooze EP"),
            ("".into(), "The Ooze EP".into())
        );
        assert_eq!(catno_and_album("[] Ooze"), ("".into(), "[] Ooze".into()));
        assert_eq!(
            catno_and_album("Flits - Advance [TOBAS 006]"),
            ("TOBAS 006".into(), "Flits - Advance".into())
        );
        assert_eq!(
            catno_and_album("Hashashin - BASTION [Vinyl]"),
            ("".into(), "Hashashin - BASTION".into())
        );
        assert_eq!(
            catno_and_album("RVDMNTL - La Fuerza (Incl. remixes by Tensal)"),
            (
                "".into(),
                "RVDMNTL - La Fuerza (Incl. remixes by Tensal)".into()
            ),
            "words that matter stay"
        );
    }

    #[test]
    fn label_names() {
        let p = page("analogicalforce", BandcampKind::Label);
        assert_eq!(
            label_name(Some("Music | Analogical Force"), &p),
            "Analogical Force"
        );
        assert_eq!(label_name(Some("Analogical Force"), &p), "Analogical Force");
        assert_eq!(label_name(Some("  "), &p), "analogicalforce");
        assert_eq!(label_name(None, &p), "analogicalforce");
    }

    #[test]
    fn reading_is_listing_only_and_after_a_double_dash() {
        let args = read_args(&page("a", BandcampKind::Album("b".into())));
        assert_eq!(
            args,
            [
                "--ignore-config",
                "--no-warnings",
                "-J",
                "--",
                "https://a.bandcamp.com/album/b"
            ]
        );
        let args = read_args(&page("a", BandcampKind::Label));
        assert!(args.contains(&"--flat-playlist".to_owned()));
        assert_eq!(args.last().unwrap(), "https://a.bandcamp.com/music");
    }

    #[test]
    fn an_album_is_read() {
        let p = parse("https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep").unwrap();
        let Ok(Listing::Album(a)) = parse_listing(&p, ALBUM) else {
            panic!("an album");
        };
        assert_eq!(
            (a.catno.as_str(), a.title.as_str()),
            ("AF070", "The Ooze EP")
        );
        assert_eq!(a.artist, "Patricia");
        assert_eq!(a.year, Some(2026));
        assert!(a.cover.starts_with("https://f4.bcbits.com/"));
        let names: Vec<(&str, &str, bool)> = a
            .tracks
            .iter()
            .map(|t| (t.artist.as_str(), t.title.as_str(), t.streamable))
            .collect();
        assert_eq!(
            names,
            [
                ("Patricia", "The Ooze", true),
                ("Patricia", "Swamp", true),
                ("Patricia", "Bonus", false),
            ]
        );
        assert_eq!(a.tracks[0].track_id, "3020153053");
        assert_eq!(
            a.tracks[0].url,
            "https://analogicalforce.bandcamp.com/track/the-ooze"
        );
        assert_eq!(a.tracks[0].duration, Some(403.521));
    }

    #[test]
    fn a_track_page_is_an_album_of_one() {
        let p = parse("https://analogicalforce.bandcamp.com/track/the-ooze").unwrap();
        let Ok(Listing::Album(a)) = parse_listing(&p, TRACK) else {
            panic!("an album");
        };
        assert_eq!(
            (a.catno.as_str(), a.title.as_str()),
            ("AF070", "The Ooze EP")
        );
        assert_eq!(a.tracks.len(), 1);
        assert_eq!(a.tracks[0].title, "The Ooze");
    }

    #[test]
    fn a_label_lists_its_own_albums_and_tracks_only() {
        let p = parse("https://analogicalforce.bandcamp.com/").unwrap();
        let Ok(Listing::Albums(urls)) = parse_listing(&p, LABEL) else {
            panic!("albums");
        };
        assert_eq!(urls.len(), 5, "{urls:?}");
        assert_eq!(
            urls[0],
            "https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep"
        );
        assert!(
            urls.iter()
                .all(|u| u.starts_with("https://analogicalforce.bandcamp.com/"))
        );
    }

    #[test]
    fn memory_survives_a_restart() {
        let dir = crate::test_dir("bandcamp-memory");
        let mut m = BandcampMemory::default();
        m.albums
            .entry(4)
            .or_default()
            .insert("https://a.bandcamp.com/album/b".into());
        m.separate.insert(("lowtidetapes".into(), 9));
        m.save(&dir).unwrap();
        assert_eq!(BandcampMemory::load(&dir), m);
        std::fs::write(BandcampMemory::path(&dir), "garbage").unwrap();
        assert_eq!(BandcampMemory::load(&dir), BandcampMemory::default());
    }

    #[test]
    fn a_label_account_names_the_artist_in_the_album_title() {
        let p = parse("https://diffusereality.bandcamp.com/album/x").unwrap();
        let track = |id: u32, title: &str| {
            format!(
                r#"{{"track_id":{id},"webpage_url":"https://diffusereality.bandcamp.com/track/t{id}",
                "title":"Diffuse Reality Records - {title}","track":"{title}",
                "artist":"Diffuse Reality Records","album_artist":"Diffuse Reality Records",
                "uploader":"Diffuse Reality Records","url":"u"}}"#
            )
        };
        let json = format!(
            r#"{{"_type":"playlist","title":"Flits - Advance [TOBAS 006]","entries":[{},{},null]}}"#,
            track(1, "Go Get It"),
            track(2, "Other Artist - Guest Track")
        );
        let Ok(Listing::Album(a)) = parse_listing(&p, &json) else {
            panic!("an album");
        };
        assert_eq!(
            (a.artist.as_str(), a.title.as_str(), a.catno.as_str()),
            ("Flits", "Advance", "TOBAS 006")
        );
        let names: Vec<(&str, &str)> = a
            .tracks
            .iter()
            .map(|t| (t.artist.as_str(), t.title.as_str()))
            .collect();
        assert_eq!(
            names,
            [("Flits", "Go Get It"), ("Other Artist", "Guest Track")]
        );
        assert!(
            parse_listing(&p, "null").is_err(),
            "nothing read is a failure"
        );
    }

    /// yt-dlp can't tell a label's account from an artist's: a dash in a title credited to
    /// the account itself is read as "Artist - Album" either way (labels are what is followed).
    #[test]
    fn a_dash_in_a_title_credited_to_the_account_is_read_as_its_artist() {
        let p = parse("https://patricia.bandcamp.com/album/x").unwrap();
        let json = r#"{"_type":"playlist","title":"Swamp - Remixes","entries":[
            {"track_id":1,"webpage_url":"https://patricia.bandcamp.com/track/a","track":"A",
             "artist":"Patricia","album_artist":"Patricia","uploader":"Patricia","url":"u"}]}"#;
        let Ok(Listing::Album(a)) = parse_listing(&p, json) else {
            panic!("an album");
        };
        assert_eq!((a.artist.as_str(), a.title.as_str()), ("Swamp", "Remixes"));
    }

    #[test]
    fn a_real_label_account_album_is_credited_to_its_artist() {
        const JSON: &str = include_str!("../tests/fixtures/bandcamp/label-account-album.json");
        let p =
            parse("https://diffusereality.bandcamp.com/album/gioele-menoni-mental-roots").unwrap();
        let Ok(Listing::Album(a)) = parse_listing(&p, JSON) else {
            panic!("an album");
        };
        assert_eq!(
            (a.artist.as_str(), a.title.as_str()),
            ("Gioele Menoni", "Mental Roots")
        );
        assert_eq!(a.tracks.len(), 3, "the two `null` entries are left out");
        assert!(
            a.tracks.iter().all(|t| t.artist == "Gioele Menoni"),
            "{:?}",
            a.tracks
        );
        assert_eq!(a.tracks[0].title, "Intro");
    }

    #[test]
    fn results_failing_the_checks_are_dropped() {
        let p = parse("https://a.bandcamp.com/album/b").unwrap();
        let json = r#"{"_type":"playlist","title":"B","entries":[
            {"track_id":"12;rm","webpage_url":"https://a.bandcamp.com/track/x","track":"X"},
            {"track_id":"12","webpage_url":"https://evil.net/track/x","track":"Y"},
            {"track_id":12,"webpage_url":"https://a.bandcamp.com/track/z","track":"Z","url":"u"}]}"#;
        let Ok(Listing::Album(a)) = parse_listing(&p, json) else {
            panic!("an album");
        };
        assert_eq!(a.tracks.len(), 1);
        assert_eq!(
            (a.tracks[0].track_id.as_str(), a.tracks[0].title.as_str()),
            ("12", "Z")
        );
        assert!(parse_listing(&p, "not json").is_err());
    }

    #[test]
    fn label_names_match_across_spellings() {
        assert_eq!(name_key("Label: Analogical Force"), "analogicalforce");
        assert_eq!(name_key("analogicalforce"), "analogicalforce");
        assert_eq!(name_key("Siesta Records Ltd"), "siesta");
        assert_eq!(name_key("siestarecords"), "siesta");
        assert_eq!(
            name_match("Label: Siesta", "Siesta Records Ltd"),
            NameMatch::Same
        );
        assert_eq!(
            name_match("Label: Analogical Force", "analogicalforce"),
            NameMatch::Same
        );
        assert_eq!(
            name_match("Label: Lowtide", "Lowtide Tapes"),
            NameMatch::Close
        );
        assert_eq!(
            name_match("Label: Hardline Sounds", "Hardline Sound"),
            NameMatch::Close
        );
        assert_eq!(
            name_match("Label: Hardline Sounds", "Lowtide Tapes"),
            NameMatch::Different
        );
        assert_eq!(name_match("Label: Records", ""), NameMatch::Different);
    }
}
