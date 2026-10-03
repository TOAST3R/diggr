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

/// The entries for a record's clips: matched clips in tracklist order, titled after their
/// track, then unmatched clips (whole sides, mixes, mislabelled uploads) titled after
/// themselves. With a remix credit for `artist`, only clips naming the artist are kept.
pub fn entries(record: &Record, role: Role, artist: &str) -> Vec<ClipEntry> {
    let clips: Vec<&Clip> = record
        .clips
        .iter()
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

    let mut out = Vec::new();
    for (ti, ci) in clip_of_track.iter().enumerate() {
        let Some(ci) = *ci else { continue };
        let t = &record.tracks[ti];
        out.push(ClipEntry {
            clip: clips[ci].id.clone(),
            artist: if t.artist.is_empty() {
                record.artist.clone()
            } else {
                t.artist.clone()
            },
            title: t.title.clone(),
            position: t.position.clone(),
            duration: clips[ci].duration,
        });
    }
    for (ci, c) in clips.iter().enumerate() {
        if used[ci] {
            continue;
        }
        let title = c
            .title
            .split_once(" - ")
            .map_or(c.title.as_str(), |(_, t)| t)
            .trim();
        out.push(ClipEntry {
            clip: c.id.clone(),
            artist: record.artist.clone(),
            title: if title.is_empty() {
                record.title.clone()
            } else {
                title.to_owned()
            },
            position: String::new(),
            duration: c.duration,
        });
    }
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
            vinyl: true,
            tracks,
            clips,
            for_sale: None,
            styles: Vec::new(),
            cover: String::new(),
        }
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
            summary(&entries(&r, Role::Main, "")),
            [
                ("a".into(), "A1".into(), "Glasshouse".into()),
                ("b".into(), "B1".into(), "Glasshouse (Lumen Remix)".into()),
                ("c".into(), "B2".into(), "Last Light".into()),
            ]
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
    fn no_clips_no_entries() {
        let r = record(vec![track("A1", "Glasshouse", None)], vec![]);
        assert!(entries(&r, Role::Main, "").is_empty());
    }
}
