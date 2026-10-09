//! Where a preview's audio comes from: a YouTube video or a Bandcamp track.
//!
//! The preview worker and the crates know a clip by a key that is also its file name: a
//! YouTube clip by its 11-character id, a Bandcamp track by `bc.‹track id›`. The dot can't
//! be in a YouTube id, so the two never collide.

use serde::{Deserialize, Serialize};

use crate::bandcamp::valid_track_id;
use crate::discogs::model::valid_clip_id;

const BANDCAMP_PREFIX: &str = "bc.";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Source {
    #[default]
    YouTube,
    Bandcamp,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::YouTube => "YouTube",
            Source::Bandcamp => "Bandcamp",
        }
    }

    /// The row's badge: "YT", "BC".
    pub fn badge(self) -> &'static str {
        match self {
            Source::YouTube => "YT",
            Source::Bandcamp => "BC",
        }
    }
}

/// A clip to download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clip {
    YouTube(String),
    Bandcamp {
        track_id: String,
        /// The track's page, checked (see [`crate::bandcamp::parse`]).
        url: String,
    },
}

/// A Bandcamp track's key: `bc.3020153053`.
pub fn bandcamp_key(track_id: &str) -> String {
    format!("{BANDCAMP_PREFIX}{track_id}")
}

/// The source a key names.
pub fn source_of(key: &str) -> Source {
    if key.starts_with(BANDCAMP_PREFIX) {
        Source::Bandcamp
    } else {
        Source::YouTube
    }
}

/// A key the worker may fetch: a valid YouTube id or `bc.` and a valid track id.
pub fn valid_key(key: &str) -> bool {
    match key.strip_prefix(BANDCAMP_PREFIX) {
        Some(id) => valid_track_id(id),
        None => valid_clip_id(key),
    }
}

/// The track id in a Bandcamp key.
pub fn track_id_of(key: &str) -> Option<&str> {
    key.strip_prefix(BANDCAMP_PREFIX)
}

impl Clip {
    pub fn key(&self) -> String {
        match self {
            Clip::YouTube(id) => id.clone(),
            Clip::Bandcamp { track_id, .. } => bandcamp_key(track_id),
        }
    }

    pub fn source(&self) -> Source {
        match self {
            Clip::YouTube(_) => Source::YouTube,
            Clip::Bandcamp { .. } => Source::Bandcamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_never_collide() {
        assert_eq!(bandcamp_key("3020153053"), "bc.3020153053");
        assert_eq!(source_of("bc.3020153053"), Source::Bandcamp);
        // An 11-character YouTube id may well start with "bc" and digits.
        assert_eq!(source_of("bc123456789"), Source::YouTube);
        assert!(valid_key("bc123456789"));
        assert!(valid_key("bc.3020153053"));
        for bad in [
            "bc.",
            "bc.12a",
            "bc.../etc",
            "bc.123456789012345678901",
            "x;rm",
        ] {
            assert!(!valid_key(bad), "{bad}");
        }
    }
}
