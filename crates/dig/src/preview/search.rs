//! Finding a preview for a track that has no clip on Discogs: the user's yt-dlp lists a few
//! YouTube results for "artist title" (nothing is downloaded), and a result is used only when
//! its title holds the track's title, it names the artist, and its length matches the
//! tracklist's. A missing preview is better than the wrong music under a record's name.
//!
//! The query comes from Discogs' tracklist text, so it is cleaned and passed as one argument
//! after `--`, never through a shell; only an 11-character video id from the results is kept.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::discogs::matching::{fold, names_artist, normalize};
use crate::discogs::model::valid_clip_id;

/// Results asked for per search.
pub const RESULTS: usize = 5;
/// The longest query passed to yt-dlp.
pub const MAX_QUERY: usize = 120;
/// A search that hasn't answered by then is stopped.
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
/// A track not found is searched again after this long (seconds).
pub const RETRY_NOT_FOUND_SECS: u64 = 7 * 24 * 3600;

/// One track to find, as the tracklist knows it.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchRequest {
    /// What the result is remembered by (`release/<id>/<position>`).
    pub key: String,
    /// The track's artist (its own credit, else the record's).
    pub artist: String,
    /// The record's artist, also accepted as the uploader's credit.
    pub record_artist: String,
    pub title: String,
    pub duration: Option<f64>,
}

/// One search result, as yt-dlp lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: String,
    pub duration: Option<f64>,
    pub channel: String,
    pub title: String,
}

/// "artist title", without control characters, at most [`MAX_QUERY`] characters.
pub fn query(artist: &str, title: &str) -> String {
    let joined = format!("{artist} {title}");
    let clean: String = joined
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let words: Vec<&str> = clean.split_whitespace().collect();
    words.join(" ").chars().take(MAX_QUERY).collect()
}

/// The whole argument list for one search: listing only, the query as one argument after
/// `--`.
pub fn args(query: &str) -> Vec<String> {
    vec![
        "--ignore-config".into(),
        "--no-warnings".into(),
        "--flat-playlist".into(),
        "--skip-download".into(),
        "--print".into(),
        "%(id)s\t%(duration)s\t%(channel)s\t%(title)s".into(),
        "--".into(),
        format!("ytsearch{RESULTS}:{query}"),
    ]
}

/// yt-dlp's lines back to results; lines without a valid id are left out.
pub fn parse(output: &str) -> Vec<SearchResult> {
    output
        .lines()
        .filter_map(|line| {
            let mut f = line.splitn(4, '\t');
            let id = f.next()?.trim();
            if !valid_clip_id(id) {
                return None;
            }
            let duration = f.next()?.trim().parse::<f64>().ok().filter(|d| *d > 0.0);
            let channel = f.next()?.trim();
            let title = f.next()?.trim();
            Some(SearchResult {
                id: id.to_owned(),
                duration,
                channel: if channel == "NA" { "" } else { channel }.to_owned(),
                title: title.to_owned(),
            })
        })
        .collect()
}

/// Whether `words` holds `part` as consecutive words.
fn contains_words(words: &[String], part: &[String]) -> bool {
    !part.is_empty() && words.windows(part.len()).any(|w| w == part)
}

/// The result to use for `req`, if any passes the rule (see the module doc): the one closest
/// in length, or the first when the tracklist gives no length.
pub fn best<'a>(req: &SearchRequest, results: &'a [SearchResult]) -> Option<&'a SearchResult> {
    // Titles are compared by their significant words; a track title made only of words the
    // matcher drops ("A1", "Untitled") is compared word for word instead.
    let plain =
        |t: &str| -> Vec<String> { fold(t).split_whitespace().map(str::to_owned).collect() };
    let significant = !normalize(&req.title, &[]).is_empty();
    let words = |t: &str| {
        if significant {
            normalize(t, &[])
        } else {
            plain(t)
        }
    };
    let track = words(&req.title);
    let artists: Vec<&str> = [req.artist.as_str(), req.record_artist.as_str()]
        .into_iter()
        .filter(|a| !fold(a).trim().is_empty())
        .collect();
    let fits = |r: &&SearchResult| {
        let title = words(&r.title);
        let named = artists
            .iter()
            .any(|a| names_artist(&r.title, a) || names_artist(&r.channel, a));
        let length = match (req.duration, r.duration) {
            (Some(want), Some(got)) => (want - got).abs() <= (want * 0.05).max(10.0),
            (Some(_), None) => false,
            (None, _) => true,
        };
        contains_words(&title, &track) && named && length
    };
    let off = |r: &SearchResult| match (req.duration, r.duration) {
        (Some(want), Some(got)) => (want - got).abs(),
        _ => 0.0,
    };
    results
        .iter()
        .filter(fits)
        .min_by(|a, b| off(a).total_cmp(&off(b)))
}

/// What a search found for a track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Remembered {
    Found {
        clip: String,
        title: String,
    },
    /// Nothing usable, at this time (seconds since the epoch).
    NotFound {
        at: u64,
    },
}

/// Search results kept in the cache folder (`searches.ron`), so a track is searched once.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Searches {
    pub results: BTreeMap<String, Remembered>,
}

pub const FILE: &str = "searches.ron";

impl Searches {
    pub fn path(cache: &Path) -> PathBuf {
        cache.join(FILE)
    }

    pub fn load(cache: &Path) -> Self {
        std::fs::read_to_string(Self::path(cache))
            .ok()
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Written atomically: a crash never leaves half a file.
    pub fn save(&self, cache: &Path) -> std::io::Result<()> {
        let text = ron::ser::to_string(self).map_err(std::io::Error::other)?;
        crate::config::write_atomic(&Self::path(cache), text.as_bytes(), false)
    }

    /// A result still good at `now`: found, or not found less than 7 days ago.
    pub fn get(&self, key: &str, now: u64) -> Option<&Remembered> {
        self.results.get(key).filter(|r| match r {
            Remembered::Found { .. } => true,
            Remembered::NotFound { at } => now.saturating_sub(*at) < RETRY_NOT_FOUND_SECS,
        })
    }

    pub fn put(&mut self, key: &str, r: Remembered) {
        self.results.insert(key.to_owned(), r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(id: &str, secs: Option<f64>, channel: &str, title: &str) -> SearchResult {
        SearchResult {
            id: id.into(),
            duration: secs,
            channel: channel.into(),
            title: title.into(),
        }
    }

    fn paper_wings(secs: Option<f64>) -> SearchRequest {
        SearchRequest {
            key: "release/38583846/A1".into(),
            artist: "The 89th Passenger".into(),
            record_artist: "The 89th Passenger".into(),
            title: "Paper Wings".into(),
            duration: secs,
        }
    }

    #[test]
    fn the_query_is_one_clean_argument_after_the_double_dash() {
        let q = query("The 89th Passenger", "--exec rm -rf ~\n\u{7}Paper");
        assert_eq!(q, "The 89th Passenger --exec rm -rf ~ Paper");
        let a = args(&q);
        let dash = a.iter().position(|x| x == "--").unwrap();
        assert_eq!(a.len(), dash + 2, "nothing after the query");
        assert_eq!(a[dash + 1], format!("ytsearch5:{q}"));
        assert!(a.contains(&"--ignore-config".to_owned()));
        assert!(a.contains(&"--skip-download".to_owned()));
        assert_eq!(query("a", &"x".repeat(500)).chars().count(), MAX_QUERY);
    }

    #[test]
    fn only_lines_with_a_valid_id_are_read() {
        let out = "dQw4w9WgXcQ\t291\tAnalogical Force\tThe 89th Passenger - Paper Wings\n\
                   not-an-id\t200\tX\tY\n\
                   abcdefghijk\tNA\tNA\tUntimed\n\
                   garbage\n";
        let r = parse(out);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].duration, Some(291.0));
        assert_eq!(r[0].channel, "Analogical Force");
        assert_eq!((r[1].duration, r[1].channel.as_str()), (None, ""));
    }

    #[test]
    fn a_result_is_used_only_when_title_artist_and_length_agree() {
        let results = [
            // A live set: right words, wrong length.
            result(
                "live0000001",
                Some(570.0),
                "Somebody",
                "The 89th Passenger Paper Wings live",
            ),
            // Another artist's "Paper Wings".
            result(
                "other000001",
                Some(291.0),
                "Other Band",
                "Other Band - Paper Wings",
            ),
            // The upload, a second off, with label noise.
            result(
                "right000001",
                Some(292.0),
                "Analogical Force",
                "The 89th Passenger – Paper Wings (Original Mix) [AF069]",
            ),
        ];
        let req = paper_wings(Some(291.0));
        assert_eq!(best(&req, &results).unwrap().id, "right000001");
        // Without it: nothing is good enough.
        assert_eq!(best(&req, &results[..2]), None);
        // The channel can name the artist ("‹artist› - Topic").
        let topic = [result(
            "topic000001",
            Some(291.0),
            "The 89th Passenger - Topic",
            "Paper Wings",
        )];
        assert!(best(&req, &topic).is_some());
        // No length in the tracklist: the first that names the title and the artist.
        let req = paper_wings(None);
        assert_eq!(best(&req, &results).unwrap().id, "live0000001");
        // A result with no length can't be checked against a known one.
        let untimed = [result(
            "untimed0001",
            None,
            "",
            "The 89th Passenger - Paper Wings",
        )];
        assert_eq!(best(&paper_wings(Some(291.0)), &untimed), None);
    }

    #[test]
    fn a_title_of_dropped_words_is_compared_word_for_word() {
        let req = SearchRequest {
            title: "A1".into(),
            ..paper_wings(Some(291.0))
        };
        let results = [
            result("wrongside01", Some(291.0), "", "The 89th Passenger - B2"),
            result("rightside01", Some(291.0), "", "The 89th Passenger - A1"),
        ];
        assert_eq!(best(&req, &results).unwrap().id, "rightside01");
    }

    #[test]
    fn the_closest_length_wins() {
        let results = [
            result(
                "near0000001",
                Some(300.0),
                "",
                "The 89th Passenger - Paper Wings",
            ),
            result(
                "nearer00001",
                Some(290.0),
                "",
                "The 89th Passenger - Paper Wings",
            ),
        ];
        assert_eq!(
            best(&paper_wings(Some(291.0)), &results).unwrap().id,
            "nearer00001"
        );
    }

    #[test]
    fn results_are_remembered_and_not_found_ages_out() {
        let d = crate::test_dir("searches");
        let mut s = Searches::load(&d);
        s.put(
            "release/1/A1",
            Remembered::Found {
                clip: "right000001".into(),
                title: "T".into(),
            },
        );
        s.put("release/1/A2", Remembered::NotFound { at: 1_000 });
        s.save(&d).unwrap();
        let s = Searches::load(&d);
        assert!(s.get("release/1/A1", u64::MAX).is_some(), "found stays");
        assert!(s.get("release/1/A2", 1_000 + 3600).is_some());
        assert!(
            s.get("release/1/A2", 1_000 + RETRY_NOT_FOUND_SECS)
                .is_none(),
            "retried"
        );
        assert!(s.get("release/1/B1", 0).is_none());
    }
}
