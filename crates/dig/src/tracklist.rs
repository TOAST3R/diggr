//! A mix's tracklist: chapters or description lines (YouTube, SoundCloud) or an extension's
//! send (1001tracklists, Mixcloud), read into tracks to search for. The parser is
//! conservative: a line needs a spaced dash between artist and title, and a description
//! giving fewer than [`MIN_TRACKS`] tracks counts as having none.

use serde_json::Value;

use crate::discogs::model::clip_id;

/// Fewer tracks than this in a description means it holds no tracklist.
pub const MIN_TRACKS: usize = 3;

/// A mix's longest time: 12 hours.
pub const MAX_TIME: u32 = 43_200;

/// Where a mix is. YouTube and SoundCloud are read with yt-dlp; Mixcloud and 1001tracklists
/// only arrive from the browser extension, whose page address is kept to open it again.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MixSource {
    YouTube(String),
    SoundCloud { user: String, slug: String },
    Mixcloud(String),
    Tracklists1001(String),
}

/// Why an address isn't a mix the app reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotMix {
    /// Not a YouTube video, SoundCloud track, Mixcloud show or 1001tracklists page.
    Other,
    /// Mixcloud or 1001tracklists: sent from the browser extension only.
    ExtensionOnly(&'static str),
}

/// `[a-z0-9_-]{1,100}`: a SoundCloud user or track name, safe as part of an argument.
fn sc_part(s: &str) -> bool {
    (1..=100).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// A pasted address as a mix: a YouTube video, or a SoundCloud track (its address rebuilt from
/// user and name, query and fragment dropped). Mixcloud and 1001tracklists are refused as
/// extension-only.
pub fn parse_url(text: &str) -> Result<MixSource, NotMix> {
    let t = text.trim();
    if let Some(id) = clip_id(t) {
        return Ok(MixSource::YouTube(id));
    }
    let rest = t
        .strip_prefix("https://")
        .or_else(|| t.strip_prefix("http://"))
        .ok_or(NotMix::Other)?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.to_ascii_lowercase();
    let host = host
        .strip_prefix("www.")
        .or(host.strip_prefix("m."))
        .unwrap_or(&host);
    match host {
        "soundcloud.com" => {
            let path = path.split(['?', '#']).next().unwrap_or("");
            match path
                .trim_end_matches('/')
                .split('/')
                .collect::<Vec<_>>()
                .as_slice()
            {
                [user, slug] if sc_part(user) && sc_part(slug) => Ok(MixSource::SoundCloud {
                    user: (*user).to_owned(),
                    slug: (*slug).to_owned(),
                }),
                _ => Err(NotMix::Other),
            }
        }
        "mixcloud.com" => Err(NotMix::ExtensionOnly("Mixcloud")),
        "1001tracklists.com" => Err(NotMix::ExtensionOnly("1001tracklists")),
        _ => Err(NotMix::Other),
    }
}

impl MixSource {
    /// The mix's own address.
    pub fn url(&self) -> String {
        match self {
            MixSource::YouTube(id) => format!("https://www.youtube.com/watch?v={id}"),
            MixSource::SoundCloud { user, slug } => format!("https://soundcloud.com/{user}/{slug}"),
            MixSource::Mixcloud(u) | MixSource::Tracklists1001(u) => u.clone(),
        }
    }

    /// The mix at `secs` in: YouTube `&t=`, SoundCloud `#t=m:ss`; the page itself elsewhere.
    pub fn at(&self, secs: u32) -> String {
        match self {
            MixSource::YouTube(_) => format!("{}&t={secs}s", self.url()),
            MixSource::SoundCloud { .. } => {
                format!("{}#t={}:{:02}", self.url(), secs / 60, secs % 60)
            }
            _ => self.url(),
        }
    }

    /// Back from a kept address (an entry's origin), the way it was stored.
    pub fn from_url(url: &str) -> Option<MixSource> {
        match parse_url(url) {
            Ok(s) => Some(s),
            Err(NotMix::ExtensionOnly("Mixcloud")) => Some(MixSource::Mixcloud(url.to_owned())),
            Err(NotMix::ExtensionOnly(_)) => Some(MixSource::Tracklists1001(url.to_owned())),
            Err(NotMix::Other) => None,
        }
    }

    /// yt-dlp's arguments to read a YouTube or SoundCloud mix's metadata, downloading nothing;
    /// None for the extension-only sites, which are never given to yt-dlp.
    pub fn read_args(&self) -> Option<Vec<String>> {
        matches!(self, MixSource::YouTube(_) | MixSource::SoundCloud { .. }).then(|| {
            [
                "--ignore-config",
                "--no-warnings",
                "--dump-single-json",
                "--skip-download",
                "--no-playlist",
                "--",
            ]
            .into_iter()
            .map(str::to_owned)
            .chain([self.url()])
            .collect()
        })
    }
}

/// Why a mix gave no tracklist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixError {
    /// Read, but neither chapters nor description list tracks: the mix's title.
    NoTracklist(String),
    /// Not read: why.
    Failed(String),
}

/// A mix's tracks, in mix order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracklist {
    pub source: MixSource,
    pub title: String,
    pub tracks: Vec<MixTrack>,
}

/// yt-dlp's `--dump-single-json` answer as a tracklist: its chapters when at least
/// [`MIN_TRACKS`] read as tracks, else its description's lines. Err (the mix's title) when
/// neither gives one.
pub fn from_ytdlp(source: MixSource, json: &str) -> Result<Tracklist, String> {
    let v: Value = serde_json::from_str(json).map_err(|_| String::new())?;
    let title = clean(v["title"].as_str().unwrap_or_default(), 200);
    let chapters: Vec<(f64, String)> = v["chapters"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|c| Some((c["start_time"].as_f64()?, c["title"].as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default();
    let mut tracks = from_chapters(&chapters);
    if tracks.is_empty() {
        tracks = from_description(v["description"].as_str().unwrap_or_default());
    }
    if tracks.is_empty() {
        return Err(title);
    }
    Ok(Tracklist {
        source,
        title,
        tracks,
    })
}

/// The most tracks a tracklist send may hold.
pub const MAX_SEND_TRACKS: usize = 300;
/// The longest a tracklist send's body may be.
pub const MAX_SEND_BODY: usize = 256 * 1024;

/// A 1001tracklists or Mixcloud page address, rebuilt: `https://www.1001tracklists.com/
/// tracklist/‹id›/‹name›.html` or `https://www.mixcloud.com/‹user›/‹show›/`.
fn page_source(url: &str) -> Option<MixSource> {
    let ok = |s: &str| {
        (1..=100).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    };
    let rest = url.strip_prefix("https://www.")?;
    let (host, path) = rest.split_once('/')?;
    let parts: Vec<&str> = path
        .split(['?', '#'])
        .next()?
        .trim_end_matches('/')
        .split('/')
        .collect();
    match (host, parts.as_slice()) {
        ("1001tracklists.com", ["tracklist", id, name]) if ok(id) && ok(name) => {
            Some(MixSource::Tracklists1001(format!(
                "https://www.1001tracklists.com/tracklist/{id}/{name}"
            )))
        }
        ("mixcloud.com", [user, show]) if ok(user) && ok(show) => Some(MixSource::Mixcloud(
            format!("https://www.mixcloud.com/{user}/{show}/"),
        )),
        _ => None,
    }
}

/// A tracklist send from the browser extension (see `browser-bridge`): its page, title and
/// tracks, each checked on its own; Err for a send that can't be taken at all. Returns the
/// tracklist and how many tracks were refused.
pub fn check(v: &Value) -> Result<(Tracklist, usize), &'static str> {
    let source = page_source(v["url"].as_str().ok_or("invalid tracklist")?).ok_or("unsupported")?;
    let text = |x: &Value| -> Option<String> {
        match x {
            Value::Null => Some(String::new()),
            Value::String(s) if s.chars().count() <= 200 && !s.chars().any(char::is_control) => {
                Some(s.trim().to_owned())
            }
            _ => None,
        }
    };
    let title = text(&v["title"])
        .filter(|t| !t.is_empty())
        .ok_or("invalid tracklist")?;
    let items = v["tracks"].as_array().ok_or("invalid tracklist")?;
    if items.is_empty() || items.len() > MAX_SEND_TRACKS {
        return Err("invalid tracklist");
    }
    let mut refused = 0;
    let tracks: Vec<MixTrack> = items
        .iter()
        .filter_map(|t| {
            let time = match &t["time"] {
                Value::Null => Some(None),
                Value::Number(n) => n
                    .as_u64()
                    .filter(|s| *s <= MAX_TIME as u64)
                    .map(|s| Some(s as u32)),
                _ => None,
            };
            let is_id = t["id"].as_bool().unwrap_or(false);
            let track = (|| {
                let m = MixTrack {
                    time: time?,
                    artist: text(&t["artist"])?,
                    title: text(&t["title"])?,
                    version: text(&t["version"])?,
                    label: text(&t["label"])?,
                    is_id,
                };
                (is_id || (!m.artist.is_empty() && !m.title.is_empty())).then_some(m)
            })();
            if track.is_none() {
                refused += 1;
            }
            track
        })
        .collect();
    if tracks.is_empty() {
        return Err("invalid tracklist");
    }
    Ok((
        Tracklist {
            source,
            title,
            tracks,
        },
        refused,
    ))
}

/// Text cut to `max` characters, without control characters.
pub fn clean(s: &str, max: usize) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect::<String>()
        .trim()
        .to_owned()
}

/// One track of a mix.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MixTrack {
    /// Seconds into the mix, when the line said.
    pub time: Option<u32>,
    pub artist: String,
    pub title: String,
    /// "Dennis Ferrer Remix".
    pub version: String,
    pub label: String,
    /// "ID", "ID - ID", "Unknown": unidentified in the mix.
    pub is_id: bool,
}

const VERSION_WORDS: [&str; 6] = ["remix", "mix", "edit", "dub", "rework", "version"];

/// `h:mm:ss`, `mm:ss` or either in brackets, at the start of `s`: its seconds and the rest.
fn leading_time(s: &str) -> Option<(u32, &str)> {
    let (inner, rest) = match s.strip_prefix('[').or_else(|| s.strip_prefix('(')) {
        Some(r) => {
            let end = r.find([']', ')'])?;
            (&r[..end], &r[end + 1..])
        }
        None => {
            let end = s
                .find(|c: char| !(c.is_ascii_digit() || c == ':'))
                .unwrap_or(s.len());
            (&s[..end], &s[end..])
        }
    };
    let parts: Vec<&str> = inner.split(':').collect();
    if !(2..=3).contains(&parts.len()) || parts.iter().any(|p| p.is_empty() || p.len() > 2) {
        return None;
    }
    let n: Vec<u32> = parts
        .iter()
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    let secs = n.iter().fold(0, |acc, v| acc * 60 + v);
    (n[1..].iter().all(|v| *v < 60) && secs <= MAX_TIME).then_some((secs, rest))
}

/// Takes off a track number (`01.`, `1)`, `#1`) and a bullet.
fn strip_number(s: &str) -> &str {
    let s = s
        .trim_start_matches(['•', '·', '*', '-', '–', '—', '>'])
        .trim_start();
    let t = s.strip_prefix('#').unwrap_or(s);
    let digits = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
    if digits > 0 && digits <= 3 {
        let rest = &t[digits..];
        if let Some(r) = rest
            .strip_prefix(['.', ')', ':'])
            .or((s.starts_with('#')).then_some(rest))
        {
            return r.trim_start();
        }
    }
    s
}

/// One line of a tracklist, or None for a line that isn't a track.
pub fn parse_line(line: &str) -> Option<MixTrack> {
    let mut s = line.trim();
    let mut time = None;
    if let Some((t, rest)) = leading_time(s) {
        time = Some(t);
        s = rest.trim_start_matches([' ', '-', '–', '—', '|']).trim();
    }
    s = strip_number(s);
    let unknown = |x: &str| matches!(x.trim().to_lowercase().as_str(), "id" | "unknown" | "?");
    if unknown(s) {
        return Some(MixTrack {
            time,
            is_id: true,
            ..MixTrack::default()
        });
    }
    let at = [" - ", " – ", " — "]
        .iter()
        .filter_map(|d| s.find(d).map(|i| (i, d.len())))
        .min()?;
    let artist = s[..at.0].trim();
    let mut title = s[at.0 + at.1..].trim();
    let mut label = String::new();
    if title.ends_with(']')
        && let Some(open) = title.rfind('[')
    {
        label = title[open + 1..title.len() - 1].trim().to_owned();
        title = title[..open].trim_end();
    }
    let mut version = String::new();
    if title.ends_with(')')
        && let Some(open) = title.rfind('(')
    {
        let inner = title[open + 1..title.len() - 1].trim();
        let last = inner.rsplit(' ').next().unwrap_or("").to_lowercase();
        if VERSION_WORDS.contains(&last.as_str()) {
            version = inner.to_owned();
            title = title[..open].trim_end();
        }
    }
    // A sentence with a dash in it ("was on the phone so … - also messed up …") is chatter.
    let words = |x: &str| x.split_whitespace().count();
    if artist.is_empty() || title.is_empty() || words(artist) > 8 || words(title) > 15 {
        return None;
    }
    if unknown(artist) && unknown(title) {
        return Some(MixTrack {
            time,
            is_id: true,
            ..MixTrack::default()
        });
    }
    Some(MixTrack {
        time,
        is_id: unknown(artist) && unknown(title),
        artist: artist.to_owned(),
        title: title.to_owned(),
        version,
        label,
    })
}

/// A description's tracks, or none when it gives fewer than [`MIN_TRACKS`].
pub fn from_description(text: &str) -> Vec<MixTrack> {
    let tracks: Vec<MixTrack> = text.lines().filter_map(parse_line).collect();
    if tracks.len() < MIN_TRACKS {
        Vec::new()
    } else {
        tracks
    }
}

/// Chapters `(start seconds, title)` as tracks, when at least [`MIN_TRACKS`] read as tracks.
pub fn from_chapters(chapters: &[(f64, String)]) -> Vec<MixTrack> {
    let tracks: Vec<MixTrack> = chapters
        .iter()
        .filter_map(|(start, title)| {
            let mut t = parse_line(title)?;
            t.time = t.time.or(Some(start.clamp(0.0, MAX_TIME as f64) as u32));
            Some(t)
        })
        .collect();
    if tracks.len() < MIN_TRACKS {
        Vec::new()
    } else {
        tracks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(time: Option<u32>, artist: &str, title: &str, version: &str, label: &str) -> MixTrack {
        MixTrack {
            time,
            artist: artist.into(),
            title: title.into(),
            version: version.into(),
            label: label.into(),
            is_id: false,
        }
    }

    #[test]
    fn real_line_shapes() {
        let id = |time| {
            Some(MixTrack {
                time,
                is_id: true,
                ..MixTrack::default()
            })
        };
        let cases: Vec<(&str, Option<MixTrack>)> = vec![
            (
                "12:34 Kerri Chandler – Rain (Dennis Ferrer Remix) [Madhouse]",
                Some(t(
                    Some(754),
                    "Kerri Chandler",
                    "Rain",
                    "Dennis Ferrer Remix",
                    "Madhouse",
                )),
            ),
            (
                "01. Moodymann - Shades of Jae",
                Some(t(None, "Moodymann", "Shades of Jae", "", "")),
            ),
            (
                "1) Moodymann - Shades of Jae",
                Some(t(None, "Moodymann", "Shades of Jae", "", "")),
            ),
            (
                "#7 Moodymann - Shades of Jae",
                Some(t(None, "Moodymann", "Shades of Jae", "", "")),
            ),
            (
                "[01:02:03] Theo Parrish - Falling Up (Carl Craig Remix)",
                Some(t(
                    Some(3723),
                    "Theo Parrish",
                    "Falling Up",
                    "Carl Craig Remix",
                    "",
                )),
            ),
            (
                "00:00 - Larry Heard — Can You Feel It",
                Some(t(Some(0), "Larry Heard", "Can You Feel It", "", "")),
            ),
            (
                "1:05:00 Robert Hood - Minus",
                Some(t(Some(3900), "Robert Hood", "Minus", "", "")),
            ),
            (
                "(4:20) DJ Sprinkles - Grand Central, Pt. 1",
                Some(t(Some(260), "DJ Sprinkles", "Grand Central, Pt. 1", "", "")),
            ),
            (
                "• Floating Points - Silhouettes (I, II & III)",
                Some(t(
                    None,
                    "Floating Points",
                    "Silhouettes (I, II & III)",
                    "",
                    "",
                )),
            ),
            (
                "Artist - Title (Original Mix)",
                Some(t(None, "Artist", "Title", "Original Mix", "")),
            ),
            (
                "Artist - Title (Dub)",
                Some(t(None, "Artist", "Title", "Dub", "")),
            ),
            (
                "Artist - Title (Extended Edit) [Label Records]",
                Some(t(None, "Artist", "Title", "Extended Edit", "Label Records")),
            ),
            (
                "Artist - Title (feat. Singer)",
                Some(t(None, "Artist", "Title (feat. Singer)", "", "")),
            ),
            ("Jay-Z - Song", Some(t(None, "Jay-Z", "Song", "", ""))),
            (
                "Artist - Title - Part 2",
                Some(t(None, "Artist", "Title - Part 2", "", "")),
            ),
            ("10. ID - ID", id(None)),
            ("23:10 ID", id(Some(1390))),
            ("Unknown", id(None)),
            ("Follow me on Instagram", None),
            ("https://example.com/my-mix", None),
            ("Tracklist:", None),
            ("", None),
            ("12:34", None),
            ("Artist -Title", None),
            (
                "was on the phone so songs played on too long - also messed up some transitions",
                None,
            ),
            ("- Title", None),
        ];
        for (line, want) in cases {
            assert_eq!(parse_line(line), want, "{line:?}");
        }
    }

    #[test]
    fn a_description_needs_three_tracks() {
        assert!(from_description("Hello\nA - B\nC - D\nThanks").is_empty());
        let three = from_description("Mix!\n00:00 A - B\n05:00 C - D\n10:00 E - F (Dub)\nBye");
        assert_eq!(three.len(), 3);
        assert_eq!(three[2].version, "Dub");
    }

    #[test]
    fn mix_addresses() {
        assert_eq!(
            parse_url("https://youtu.be/dQw4w9WgXcQ?t=5"),
            Ok(MixSource::YouTube("dQw4w9WgXcQ".into()))
        );
        let sc = parse_url("https://soundcloud.com/some-dj/summer-mix-2026?si=abc#x").unwrap();
        assert_eq!(sc.url(), "https://soundcloud.com/some-dj/summer-mix-2026");
        assert_eq!(
            sc.at(754),
            "https://soundcloud.com/some-dj/summer-mix-2026#t=12:34"
        );
        assert_eq!(
            MixSource::YouTube("dQw4w9WgXcQ".into()).at(754),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=754s"
        );
        for refused in [
            "https://soundcloud.com/some-dj",
            "https://soundcloud.com/some-dj/sets/summer",
            "https://soundcloud.com/Some-DJ/x",
            "https://soundcloud.com/a/b;rm",
            "https://soundcloud.com.evil.net/a/b",
            "https://example.com/a/b",
        ] {
            assert_eq!(parse_url(refused), Err(NotMix::Other), "{refused}");
        }
        assert_eq!(
            parse_url("https://www.mixcloud.com/dj/show/"),
            Err(NotMix::ExtensionOnly("Mixcloud"))
        );
        assert_eq!(
            parse_url("https://www.1001tracklists.com/tracklist/abc/x.html"),
            Err(NotMix::ExtensionOnly("1001tracklists"))
        );
        // Only YouTube and SoundCloud reach yt-dlp, after `--`, never downloading.
        let args = sc.read_args().unwrap();
        assert_eq!(
            args[args.len() - 2..],
            ["--", "https://soundcloud.com/some-dj/summer-mix-2026"]
        );
        assert!(args.contains(&"--skip-download".to_owned()));
        assert!(
            MixSource::Mixcloud("https://www.mixcloud.com/a/b/".into())
                .read_args()
                .is_none()
        );
        assert_eq!(
            MixSource::from_url("https://www.mixcloud.com/a/b/"),
            Some(MixSource::Mixcloud("https://www.mixcloud.com/a/b/".into()))
        );
    }

    /// Shapes of yt-dlp's `--dump-single-json` (trimmed to the fields read).
    const CHAPTERED: &str = r#"{"title": "Boiler Room: Kerri Chandler", "duration": 3600,
        "description": "Thanks for watching",
        "chapters": [{"start_time": 0.0, "title": "Intro"},
                     {"start_time": 95.5, "title": "Kerri Chandler - Rain"},
                     {"start_time": 412.0, "title": "Moodymann - Shades of Jae"},
                     {"start_time": 800.0, "title": "Larry Heard - Can You Feel It (Dub)"}]}"#;
    const DESCRIBED: &str = r#"{"title": "Deep House Mix 2026", "chapters": null,
        "description": "Tracklist:\n00:00 Kerri Chandler - Rain [Madhouse]\n05:10 ID - ID\n11:02 Moodymann - Shades of Jae\nFollow me!"}"#;
    const SOUNDCLOUD: &str = r#"{"title": "Summer Mix 2026", "uploader": "some-dj",
        "description": "1. A - B\n2. C - D (Edit)\n3. E - F"}"#;
    const NOTHING: &str = r#"{"title": "My vlog", "description": "hello\nworld", "chapters": []}"#;

    #[test]
    fn chapters_first_then_the_description() {
        let yt = || MixSource::YouTube("dQw4w9WgXcQ".into());
        let c = from_ytdlp(yt(), CHAPTERED).unwrap();
        assert_eq!(c.title, "Boiler Room: Kerri Chandler");
        assert_eq!(c.tracks.len(), 3, "Intro isn't a track");
        assert_eq!(
            (c.tracks[0].time, c.tracks[2].version.as_str()),
            (Some(95), "Dub")
        );
        let d = from_ytdlp(yt(), DESCRIBED).unwrap();
        assert_eq!(d.tracks.len(), 3);
        assert!(d.tracks[1].is_id);
        assert_eq!(
            (d.tracks[0].label.as_str(), d.tracks[2].time),
            ("Madhouse", Some(662))
        );
        let sc = MixSource::SoundCloud {
            user: "some-dj".into(),
            slug: "summer-mix-2026".into(),
        };
        assert_eq!(
            from_ytdlp(sc, SOUNDCLOUD).unwrap().tracks[1].version,
            "Edit"
        );
        assert_eq!(from_ytdlp(yt(), NOTHING), Err("My vlog".to_owned()));
    }

    /// Real answers of yt-dlp, trimmed to the fields read (`tests/fixtures/mix-*.json`).
    fn fixture(name: &str) -> String {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
        std::fs::read_to_string(format!("{dir}{name}")).unwrap()
    }

    #[test]
    fn real_mixes() {
        // A YouTube mix of 37 chapters: 18 "Artist - Title", 19 "Title (Somebody Cover)"
        // with no artist, which are left out.
        let yt = MixSource::YouTube("1nQzXzBvcWk".into());
        let c = from_ytdlp(yt.clone(), &fixture("mix-youtube-chapters.json")).unwrap();
        assert_eq!(c.tracks.len(), 18);
        assert_eq!(
            (c.tracks[0].artist.as_str(), c.tracks[0].title.as_str()),
            ("ORGAN & DELLA", "Bad Generation")
        );
        assert!(
            c.tracks.windows(2).all(|w| w[0].time < w[1].time),
            "in mix order"
        );
        // Chapters naming titles only ("1. Take The Chance"): no tracklist, not a guess.
        let plain = from_ytdlp(yt, &fixture("mix-youtube-plain-chapters.json"));
        assert!(plain.is_err(), "{plain:?}");
        // A SoundCloud mix listing its tracks in the description.
        let sc =
            parse_url("https://soundcloud.com/benedictquinn/discodeeplofielectrohousemix").unwrap();
        let d = from_ytdlp(sc, &fixture("mix-soundcloud.json")).unwrap();
        assert!(d.tracks.len() >= 30, "{}", d.tracks.len());
        assert_eq!(
            (d.tracks[0].artist.as_str(), d.tracks[0].title.as_str()),
            ("The Whispers", "Contagious")
        );
        let logg = d.tracks.iter().find(|t| t.artist == "Logg").unwrap();
        assert_eq!(
            (logg.title.as_str(), logg.version.as_str()),
            ("Dancing Into the Stars", "John Morales remix")
        );
    }

    #[test]
    fn a_tracklist_send_is_checked_track_by_track() {
        let v: Value = serde_json::from_str(
            r#"{"url": "https://www.1001tracklists.com/tracklist/2p7bd3vt/duarte-sterium.html?x=1",
                "title": "Duarte b2b Sterium @ Boiler Room",
                "tracks": [
                  {"time": 0, "artist": "Kerri Chandler", "title": "Rain", "label": "Madhouse"},
                  {"time": 300, "id": true},
                  {"time": 99999, "artist": "A", "title": "B"},
                  {"artist": "C", "title": "D\u0007"},
                  {"artist": "", "title": "E"},
                  {"time": null, "artist": "F", "title": "G", "version": "Dub"}]}"#,
        )
        .unwrap();
        let (list, refused) = check(&v).unwrap();
        assert_eq!(
            refused, 3,
            "a time past 12 h, a control character, no artist"
        );
        assert_eq!(list.tracks.len(), 3);
        assert!(list.tracks[1].is_id);
        assert_eq!(
            list.source.url(),
            "https://www.1001tracklists.com/tracklist/2p7bd3vt/duarte-sterium.html"
        );
        let mc: Value = serde_json::from_str(
            r#"{"url": "https://www.mixcloud.com/dj/show/", "title": "Show", "tracks": [{"artist": "A", "title": "B"}]}"#,
        )
        .unwrap();
        assert!(matches!(
            check(&mc).unwrap().0.source,
            MixSource::Mixcloud(_)
        ));
        for bad in [
            r#"{"url": "https://www.mixcloud.com.evil.net/a/b/", "title": "x", "tracks": [{"artist": "A", "title": "B"}]}"#,
            r#"{"url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ", "title": "x", "tracks": [{"artist": "A", "title": "B"}]}"#,
            r#"{"url": "https://www.mixcloud.com/a/b/", "title": "x", "tracks": []}"#,
        ] {
            assert!(check(&serde_json::from_str(bad).unwrap()).is_err(), "{bad}");
        }
    }

    #[test]
    fn chapters_take_their_start() {
        let ch = |s: f64, t: &str| (s, t.to_owned());
        let tracks = from_chapters(&[
            ch(0.0, "Intro"),
            ch(60.0, "A - B"),
            ch(300.0, "C - D"),
            ch(600.5, "E - F"),
        ]);
        assert_eq!(
            tracks.iter().map(|t| t.time).collect::<Vec<_>>(),
            [Some(60), Some(300), Some(600)]
        );
        assert!(from_chapters(&[ch(0.0, "A - B"), ch(1.0, "Intro")]).is_empty());
    }
}
