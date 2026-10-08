//! Which clip is which track: Discogs clips are uploads with free-form titles ("Nightcraft -
//! Glasshouse (Original Mix) [HQ] 1994"), matched to the tracklist by the words they share.

use std::collections::HashSet;

use super::model::{Clip, Record, Role};

/// Minimum word overlap (Jaccard) for a clip to be a track.
const MIN_SCORE: f64 = 0.5;
/// Scores this close count as a tie, won by the clip whose duration matches.
const TIE: f64 = 0.1;
const DURATION_TOLERANCE: f64 = 10.0;

/// One entry to add for a clip.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipEntry {
    pub clip: String,
    pub artist: String,
    pub title: String,
    /// The side, e.g. "A1"; empty for an unmatched clip.
    pub position: String,
    /// The clip's duration, as a hint until the file is read.
    pub duration: Option<f64>,
}

/// One entry for a track that no clip of its record matches: its preview is searched for when
/// it is about to play.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackEntry {
    pub artist: String,
    pub title: String,
    /// The side, e.g. "A1".
    pub position: String,
    /// From the tracklist, when it gives one.
    pub duration: Option<f64>,
    /// What its search result is remembered by (see [`track_key`]).
    pub search_key: String,
}

/// What a record becomes in a crate: one item per track of its tracklist, in order (its clip
/// when one matches, else a track to search), then the clips that match no track.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RecordPlan {
    pub items: Vec<Planned>,
    /// A full-album upload held back while the record's tracks are searched for: added only
    /// when one of them is not found.
    pub full_album: Option<ClipEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Planned {
    Clip(ClipEntry),
    Search(TrackEntry),
}

impl RecordPlan {
    pub fn clips(clips: Vec<ClipEntry>) -> Self {
        Self {
            items: clips.into_iter().map(Planned::Clip).collect(),
            full_album: None,
        }
    }

    pub fn tracks(tracks: Vec<TrackEntry>) -> Self {
        Self {
            items: tracks.into_iter().map(Planned::Search).collect(),
            full_album: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Whether any item has a clip of its own (not counting a held-back full album).
    pub fn has_clip(&self) -> bool {
        self.items.iter().any(|p| matches!(p, Planned::Clip(_)))
    }

    pub fn has_search(&self) -> bool {
        self.items.iter().any(|p| matches!(p, Planned::Search(_)))
    }
}

/// What a track's search result is remembered by: its artist and title, folded, so every
/// release of the same track (vinyl A1, CD 1, a compilation) shares one search.
pub fn track_key(artist: &str, title: &str) -> String {
    let words = |s: &str| fold(s).split_whitespace().collect::<Vec<_>>().join(" ");
    format!("track/{}/{}", words(artist), words(title))
}

fn track_entry(record: &Record, i: usize) -> TrackEntry {
    let t = &record.tracks[i];
    let artist = if t.artist.is_empty() {
        record.artist.clone()
    } else {
        t.artist.clone()
    };
    let title = t.title.trim().to_owned();
    TrackEntry {
        search_key: track_key(&artist, &title),
        artist,
        title,
        position: t.position.trim().to_owned(),
        duration: t.duration,
    }
}

/// Words that say nothing about which track a clip is.
const NOISE: [&str; 14] = [
    "official",
    "video",
    "audio",
    "hq",
    "hd",
    "vinyl",
    "rip",
    "original",
    "mix",
    "full",
    "remaster",
    "remastered",
    "version",
    "lyrics",
];
/// Noise that is only noise as a phrase ("original mix" is noise, "the mix" may be a title).
const NOISE_PHRASES: [&str; 5] = [
    "official video",
    "official audio",
    "vinyl rip",
    "original mix",
    "full track",
];

/// Lowercase, accents folded; anything that isn't a letter or digit becomes a space.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => 'a',
            'ç' | 'ć' | 'č' => 'c',
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' | 'ń' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'ū' => 'u',
            'ý' | 'ÿ' => 'y',
            'ß' => 's',
            'ł' => 'l',
            'š' => 's',
            'ž' | 'ź' | 'ż' => 'z',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        };
        out.push(c);
    }
    out
}

fn is_noise_word(w: &str) -> bool {
    NOISE.contains(&w)
        // Years and record sizes (12" folds to "12").
        || (w.len() == 4 && w.starts_with(['1', '2']) && w.bytes().all(|b| b.is_ascii_digit()))
        || matches!(w, "7" | "10" | "12" | "7inch" | "10inch" | "12inch")
        // Side positions: a1, b2.
        || (w.len() == 2 && w.as_bytes()[0].is_ascii_lowercase() && w.as_bytes()[1].is_ascii_digit())
}

/// Removes `[...]` and `(...)` groups that hold nothing but noise ("(Official Video)",
/// "[HQ]", "(1994)", `(12")`); groups with words that matter ("(Nightcraft Remix)") stay.
fn drop_bracketed_noise(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(open) = rest.find(['(', '[']) {
        let close_char = if rest.as_bytes()[open] == b'(' {
            ')'
        } else {
            ']'
        };
        let Some(len) = rest[open + 1..].find(close_char) else {
            break;
        };
        let inner = &rest[open + 1..open + 1 + len];
        out.push_str(&rest[..open]);
        let mut folded = format!(" {} ", fold(inner));
        for p in NOISE_PHRASES {
            folded = folded.replace(&format!(" {p} "), " ");
        }
        if !folded.split_whitespace().all(is_noise_word) {
            out.push(' ');
            out.push_str(inner);
            out.push(' ');
        }
        rest = &rest[open + 2 + len..];
    }
    out.push_str(rest);
    out
}

/// A title's significant words: accents folded, bracketed noise and noise words gone, and the
/// record's artist names dropped (a clip is usually titled "Artist - Title").
pub fn normalize(title: &str, artists: &[String]) -> Vec<String> {
    let mut text = format!(" {} ", fold(&drop_bracketed_noise(title)));
    for p in NOISE_PHRASES {
        text = text.replace(&format!(" {p} "), " ");
    }
    for a in artists {
        let a = fold(a).split_whitespace().collect::<Vec<_>>().join(" ");
        if !a.is_empty() {
            text = text.replace(&format!(" {a} "), " ");
        }
    }
    text.split_whitespace()
        .filter(|w| !is_noise_word(w))
        .map(str::to_owned)
        .collect()
}

pub fn jaccard(a: &[String], b: &[String]) -> f64 {
    let a: HashSet<&String> = a.iter().collect();
    let b: HashSet<&String> = b.iter().collect();
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(&b).count() as f64 / union as f64
}

/// For a remix credit: whether the clip's title names the artist.
pub fn names_artist(clip_title: &str, artist: &str) -> bool {
    let a = fold(artist)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let t = fold(clip_title)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    !a.is_empty() && format!(" {t} ").contains(&format!(" {a} "))
}

/// Whether a clip's title says it is the whole record.
fn says_full_album(title: &str) -> bool {
    format!(
        " {} ",
        fold(title).split_whitespace().collect::<Vec<_>>().join(" ")
    )
    .contains(" full album ")
}

/// A record's plan: each track once, with its clip when one matches it, else to be searched
/// for; then the clips that match no track, titled after themselves (whole sides, mixes,
/// mislabelled uploads). A video listed twice counts once. With a remix credit for `artist`,
/// only the clips and the tracks naming the artist are kept (a clip naming the artist still
/// takes the track it matches). A full-album upload (its title says so, or it is the record's
/// only clip and matches no track) is held back while there are tracks to search.
pub fn plan(record: &Record, role: Role, artist: &str) -> RecordPlan {
    let mut seen = HashSet::new();
    let clips: Vec<&Clip> = record
        .clips
        .iter()
        .filter(|c| seen.insert(c.id.as_str()))
        .filter(|c| role == Role::Main || names_artist(&c.title, artist))
        .collect();
    let clip_words: Vec<Vec<String>> = clips
        .iter()
        .map(|c| normalize(&c.title, &record.artist_names))
        .collect();
    let track_words: Vec<Vec<String>> = record
        .tracks
        .iter()
        .map(|t| normalize(&t.title, &record.artist_names))
        .collect();

    // Every pair over the threshold, best first; a duration match lifts a pair over any other
    // within the tie margin.
    let mut pairs = Vec::new();
    for (ci, cw) in clip_words.iter().enumerate() {
        for (ti, tw) in track_words.iter().enumerate() {
            let score = jaccard(cw, tw);
            if score >= MIN_SCORE {
                let fits = match (clips[ci].duration, record.tracks[ti].duration) {
                    (Some(c), Some(t)) => (c - t).abs() <= DURATION_TOLERANCE,
                    _ => false,
                };
                let rank = score + if fits { TIE } else { 0.0 };
                pairs.push((rank, fits, ci, ti));
            }
        }
    }
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
    let mut clip_of_track = vec![None; record.tracks.len()];
    let mut used = vec![false; clips.len()];
    for (_, _, ci, ti) in pairs {
        if !used[ci] && clip_of_track[ti].is_none() {
            used[ci] = true;
            clip_of_track[ti] = Some(ci);
        }
    }

    let mut out = RecordPlan::default();
    for (ti, ci) in clip_of_track.iter().enumerate() {
        let t = &record.tracks[ti];
        match *ci {
            Some(ci) => out.items.push(Planned::Clip(ClipEntry {
                clip: clips[ci].id.clone(),
                artist: if t.artist.is_empty() {
                    record.artist.clone()
                } else {
                    t.artist.clone()
                },
                title: t.title.clone(),
                position: t.position.clone(),
                duration: clips[ci].duration,
            })),
            None if t.title.trim().is_empty() => {}
            None if role == Role::Main || names_artist(&t.title, artist) => {
                out.items.push(Planned::Search(track_entry(record, ti)));
            }
            None => {}
        }
    }
    let only_clip = clips.len() == 1;
    let mut extras = Vec::new();
    for (ci, c) in clips.iter().enumerate() {
        if used[ci] {
            continue;
        }
        let title = c
            .title
            .split_once(" - ")
            .map_or(c.title.as_str(), |(_, t)| t)
            .trim();
        let entry = ClipEntry {
            clip: c.id.clone(),
            artist: record.artist.clone(),
            title: if title.is_empty() {
                record.title.clone()
            } else {
                title.to_owned()
            },
            position: String::new(),
            duration: c.duration,
        };
        let full_album = only_clip || says_full_album(&c.title);
        if full_album && out.full_album.is_none() && out.has_search() {
            out.full_album = Some(entry);
        } else {
            extras.push(entry);
        }
    }
    out.items.extend(extras.into_iter().map(Planned::Clip));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discogs::model::{RecordKey, Track};

    fn track(pos: &str, title: &str, secs: Option<f64>) -> Track {
        Track {
            position: pos.into(),
            title: title.into(),
            artist: String::new(),
            duration: secs,
        }
    }

    fn clip(id: &str, title: &str, secs: Option<f64>) -> Clip {
        Clip {
            id: format!("{id:x<11}"),
            title: title.into(),
            duration: secs,
        }
    }

    fn record(tracks: Vec<Track>, clips: Vec<Clip>) -> Record {
        Record {
            key: RecordKey::Release(123456),
            release: Some(123456),
            master: None,
            artist: "Nightcraft".into(),
            artist_names: vec!["Nightcraft".into()],
            title: "Glasshouse EP".into(),
            label: "Lowtide Tapes".into(),
            catno: "LT-012".into(),
            year: Some(1994),
            formats: vec![crate::discogs::model::Format::Vinyl],
            tracks,
            clips,
            for_sale: None,
            styles: Vec::new(),
            cover: String::new(),
        }
    }

    /// The plan's clip entries, in order.
    fn entries(r: &Record, role: Role, artist: &str) -> Vec<ClipEntry> {
        plan(r, role, artist)
            .items
            .into_iter()
            .filter_map(|p| match p {
                Planned::Clip(c) => Some(c),
                Planned::Search(_) => None,
            })
            .collect()
    }

    /// (clip id's first letter, or "?" to search; position; title) per item.
    fn shape(p: &RecordPlan) -> Vec<(String, String, String)> {
        p.items
            .iter()
            .map(|p| match p {
                Planned::Clip(c) => (c.clip[..1].to_owned(), c.position.clone(), c.title.clone()),
                Planned::Search(t) => ("?".into(), t.position.clone(), t.title.clone()),
            })
            .collect()
    }

    fn summary(e: &[ClipEntry]) -> Vec<(String, String, String)> {
        e.iter()
            .map(|e| (e.clip[..1].to_owned(), e.position.clone(), e.title.clone()))
            .collect()
    }

    #[test]
    fn normalizing_drops_noise_artists_and_accents() {
        let a = vec!["Nightcraft".to_string()];
        assert_eq!(
            normalize("NIGHTCRAFT - Glasshouse (Original Mix) [HQ] 1994", &a),
            ["glasshouse"]
        );
        assert_eq!(
            normalize("Nightcraft – Glasshouse (Official Video)", &a),
            ["glasshouse"]
        );
        assert_eq!(
            normalize("Nightcraft - Café Noir 12\" Vinyl Rip", &a),
            ["cafe", "noir"]
        );
        assert_eq!(
            normalize("Glasshouse (Lumen Remix)", &a),
            ["glasshouse", "lumen", "remix"],
            "a remix credit is not noise"
        );
        assert_eq!(normalize("A1. Glasshouse", &a), ["glasshouse"]);
    }

    #[test]
    fn a_release_with_four_tracks_and_three_clips() {
        let r = record(
            vec![
                track("A1", "Glasshouse", Some(372.0)),
                track("A2", "Tidal Pull", None),
                track("B1", "Glasshouse (Lumen Remix)", None),
                track("B2", "Last Light", None),
            ],
            vec![
                clip("b", "Nightcraft - Glasshouse (Lumen Remix) HD", None),
                clip("a", "NIGHTCRAFT - GLASSHOUSE [1994]", Some(371.0)),
                clip("c", "Nightcraft – Last Light (Vinyl Rip)", None),
            ],
        );
        assert_eq!(
            shape(&plan(&r, Role::Main, "")),
            [
                ("a".into(), "A1".into(), "Glasshouse".into()),
                ("?".into(), "A2".into(), "Tidal Pull".into()),
                ("b".into(), "B1".into(), "Glasshouse (Lumen Remix)".into()),
                ("c".into(), "B2".into(), "Last Light".into()),
            ],
            "every track once, the one without a clip to search"
        );
        let e = entries(&r, Role::Main, "");
        assert!(e.iter().all(|e| e.artist == "Nightcraft"));
    }

    #[test]
    fn duration_breaks_a_tie() {
        // Two uploads of the same title: the one whose length fits the track wins it; the other
        // is kept as its own entry.
        let r = record(
            vec![track("A1", "Glasshouse", Some(372.0))],
            vec![
                clip("x", "Nightcraft - Glasshouse", Some(200.0)),
                clip("y", "Nightcraft - Glasshouse [HQ]", Some(368.0)),
            ],
        );
        let e = summary(&entries(&r, Role::Main, ""));
        assert_eq!(e[0], ("y".into(), "A1".into(), "Glasshouse".into()));
        assert_eq!(e[1], ("x".into(), "".into(), "Glasshouse".into()));
    }

    #[test]
    fn unmatched_and_long_clips_keep_their_own_title() {
        let r = record(
            vec![track("A1", "Glasshouse", None)],
            vec![
                clip("s", "Nightcraft - Glasshouse EP (Side A)", Some(1_500.0)),
                clip("m", "Something unrelated", None),
            ],
        );
        let e = entries(&r, Role::Main, "");
        assert_eq!(
            summary(&e),
            [
                ("s".into(), "".into(), "Glasshouse EP (Side A)".into()),
                ("m".into(), "".into(), "Something unrelated".into()),
            ]
        );
        assert_eq!(e[0].artist, "Nightcraft");
    }

    #[test]
    fn a_remix_credit_keeps_only_clips_naming_the_artist() {
        let mut r = record(
            vec![
                track("A1", "Glasshouse", None),
                track("A2", "Glasshouse (Lumen Remix)", None),
                track("B1", "Tidal", None),
                track("B2", "Tidal (Dub)", None),
            ],
            vec![
                clip("a", "Nightcraft - Glasshouse", None),
                clip("b", "Nightcraft - Glasshouse (Lümen Remix)", None),
                clip("c", "Nightcraft - Tidal", None),
                clip("d", "Nightcraft - Tidal (Dub)", None),
            ],
        );
        r.artist_names.push("Unrelated".into());
        let e = entries(&r, Role::Remix, "Lumen");
        assert_eq!(
            summary(&e),
            [("b".into(), "A2".into(), "Glasshouse (Lumen Remix)".into())]
        );
        assert!(!names_artist("Illuminate", "Lumen"), "whole words only");
    }

    #[test]
    fn no_clips_every_track_to_search() {
        let r = record(
            vec![
                track("A1", "Glasshouse", Some(372.0)),
                track("", "", None),
                track("", "Tidal", None),
            ],
            vec![],
        );
        let p = plan(&r, Role::Main, "");
        assert_eq!(
            shape(&p),
            [
                ("?".into(), "A1".into(), "Glasshouse".into()),
                ("?".into(), "".into(), "Tidal".into()),
            ],
            "untitled tracks are skipped"
        );
        let Planned::Search(t) = &p.items[0] else {
            unreachable!()
        };
        assert_eq!(t.artist, "Nightcraft");
        assert_eq!(t.duration, Some(372.0));
        assert_eq!(t.search_key, "track/nightcraft/glasshouse");
    }

    /// Massive Attack, Mezzanine (release 5077187): eleven tracks, one video listed twice.
    #[test]
    fn one_video_for_eleven_tracks() {
        let titles = [
            ("A1", "Angel"),
            ("A2", "Risingson"),
            ("A3", "Teardrop"),
            ("B1", "Inertia Creeps"),
            ("B2", "Exchange"),
            ("B3", "Dissolved Girl"),
            ("C1", "Man Next Door"),
            ("C2", "Black Milk"),
            ("C3", "Mezzanine"),
            ("D1", "Group Four"),
            ("D2", "(Exchange)"),
        ];
        let mut r = record(
            titles.iter().map(|(p, t)| track(p, t, None)).collect(),
            vec![
                clip(
                    "t",
                    "Massive Attack - Teardrop (Official Video)",
                    Some(285.0),
                ),
                clip(
                    "t",
                    "Massive Attack - Teardrop (Official Video)",
                    Some(285.0),
                ),
            ],
        );
        r.artist = "Massive Attack".into();
        r.artist_names = vec!["Massive Attack".into()];
        r.title = "Mezzanine".into();
        let p = plan(&r, Role::Main, "");
        assert_eq!(
            p.items.len(),
            11,
            "one entry per track, the repeat counted once"
        );
        assert_eq!(p.full_album, None, "the only clip matches a track");
        for (item, (pos, title)) in p.items.iter().zip(titles) {
            match item {
                Planned::Clip(c) => assert_eq!((pos, title), ("A3", "Teardrop"), "{c:?}"),
                Planned::Search(t) => {
                    assert_eq!((t.position.as_str(), t.title.as_str()), (pos, title))
                }
            }
        }
        let keys: Vec<&str> = p
            .items
            .iter()
            .filter_map(|p| match p {
                Planned::Search(t) => Some(t.search_key.as_str()),
                Planned::Clip(_) => None,
            })
            .collect();
        assert_eq!(keys[3], "track/massive attack/exchange");
        assert_eq!(
            keys[3], keys[9],
            "Exchange and (Exchange) are the same track"
        );
    }

    #[test]
    fn a_full_album_upload_is_held_back() {
        let tracks = vec![
            track("A1", "Glasshouse", None),
            track("A2", "Tidal", None),
            track("B1", "Last Light", None),
        ];
        // Said in its title, among other clips.
        let r = record(
            tracks.clone(),
            vec![
                clip("a", "Nightcraft - Glasshouse", None),
                clip(
                    "f",
                    "Nightcraft – Glasshouse EP [FULL ALBUM]",
                    Some(1_500.0),
                ),
                clip("s", "Nightcraft - Glasshouse EP (Side B)", Some(700.0)),
            ],
        );
        let p = plan(&r, Role::Main, "");
        assert_eq!(
            shape(&p),
            [
                ("a".into(), "A1".into(), "Glasshouse".into()),
                ("?".into(), "A2".into(), "Tidal".into()),
                ("?".into(), "B1".into(), "Last Light".into()),
                ("s".into(), "".into(), "Glasshouse EP (Side B)".into()),
            ],
            "a side rip among other clips stays an extra"
        );
        assert_eq!(p.full_album.as_ref().map(|c| &c.clip[..1]), Some("f"));

        // The record's only clip, matching no track.
        let r = record(
            tracks.clone(),
            vec![clip(
                "o",
                "Nightcraft - The Lowtide Sessions",
                Some(1_500.0),
            )],
        );
        let p = plan(&r, Role::Main, "");
        assert_eq!(p.items.len(), 3);
        assert!(!p.has_clip());
        assert_eq!(p.full_album.as_ref().map(|c| &c.clip[..1]), Some("o"));

        // Nothing to search: it is an ordinary clip.
        let r = record(
            vec![],
            vec![clip("o", "Nightcraft - Glasshouse EP (Full Album)", None)],
        );
        let p = plan(&r, Role::Main, "");
        assert_eq!(p.full_album, None);
        assert_eq!(
            shape(&p),
            [("o".into(), "".into(), "Glasshouse EP (Full Album)".into())]
        );
        assert!(
            !says_full_album("Nightcraft - Full Albumen"),
            "whole words only"
        );
    }

    #[test]
    fn a_remix_credit_searches_its_track_without_a_clip() {
        let r = record(
            vec![
                track("A1", "Glasshouse", None),
                track("A2", "Glasshouse (Lumen Remix)", Some(400.0)),
                track("B1", "Tidal", None),
                track("B2", "Last Light", None),
            ],
            vec![
                clip("a", "Nightcraft - Glasshouse", None),
                clip("c", "Nightcraft - Tidal", None),
                clip("d", "Nightcraft - Last Light", None),
                clip("e", "Nightcraft - Glasshouse (Dub)", None),
            ],
        );
        let p = plan(&r, Role::Remix, "Lumen");
        assert_eq!(
            shape(&p),
            [("?".into(), "A2".into(), "Glasshouse (Lumen Remix)".into())]
        );
        assert_eq!(p.full_album, None);
    }

    #[test]
    fn the_track_key_ignores_case_accents_and_punctuation() {
        assert_eq!(
            track_key("Nightcraft", "Café Noir!"),
            track_key("NIGHTCRAFT", "  cafe   noir ")
        );
        assert_ne!(
            track_key("Nightcraft", "Glasshouse"),
            track_key("Lumen", "Glasshouse")
        );
    }
}
