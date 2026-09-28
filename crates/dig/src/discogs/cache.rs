//! Discogs responses on disk, under `<cache>/discogs/`, so a page sent again costs nothing.
//!
//! Record data (releases, masters, artists, labels) is reused indefinitely; only its
//! marketplace numbers go stale, after 24 h, and are refreshed when a track from it starts.
//! Listing pages are kept for 24 h. Wantlist pages are never cached: they change with every
//! keep. There is no size limit: a release is 5–20 KB and the folder is safe to delete.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DIR: &str = "discogs";
/// Listing pages, and a release's for-sale numbers, count as stale after this.
pub const FRESH_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Release,
    Master,
    Artist,
    Label,
    Listing,
}

impl Kind {
    fn folder(self) -> &'static str {
        match self {
            Kind::Release => "release",
            Kind::Master => "master",
            Kind::Artist => "artist",
            Kind::Label => "label",
            Kind::Listing => "listing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cached {
    /// Seconds since the Unix epoch.
    pub fetched_at: u64,
    /// The raw response.
    pub body: String,
    /// The currency its prices were asked for in (releases).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

impl Cached {
    pub fn is_fresh(&self, now: u64) -> bool {
        now.saturating_sub(self.fetched_at) < FRESH_SECS
    }
}

/// `None` keeps nothing (no cache folder).
#[derive(Debug, Clone, Default)]
pub struct DiskCache {
    dir: Option<PathBuf>,
}

impl DiskCache {
    /// `root` is the app's cache folder; responses go in its `discogs/`.
    pub fn new(root: Option<&Path>) -> Self {
        Self {
            dir: root.map(|r| r.join(DIR)),
        }
    }

    fn path(&self, kind: Kind, key: &str) -> Option<PathBuf> {
        let safe: String = key
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        Some(self.dir.as_ref()?.join(kind.folder()).join(safe + ".json"))
    }

    pub fn get(&self, kind: Kind, key: &str) -> Option<Cached> {
        let text = std::fs::read_to_string(self.path(kind, key)?).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// A listing page, if fetched less than 24 h ago.
    pub fn listing(&self, key: &str, now: u64) -> Option<String> {
        self.get(Kind::Listing, key)
            .filter(|c| c.is_fresh(now))
            .map(|c| c.body)
    }

    /// Writes atomically; failures are ignored (the cache is only an optimisation).
    pub fn put(&self, kind: Kind, key: &str, body: &str, fetched_at: u64) {
        self.put_priced(kind, key, body, fetched_at, None);
    }

    /// A response whose prices are in `currency`.
    pub fn put_priced(
        &self,
        kind: Kind,
        key: &str,
        body: &str,
        fetched_at: u64,
        currency: Option<&str>,
    ) {
        let Some(path) = self.path(kind, key) else {
            return;
        };
        let text = serde_json::to_string(&Cached {
            fetched_at,
            body: body.to_owned(),
            currency: currency.map(str::to_owned),
        })
        .expect("serializable");
        let tmp = path.with_extension("tmp");
        let _ = std::fs::create_dir_all(path.parent().expect("has parent"))
            .and_then(|()| std::fs::write(&tmp, text))
            .and_then(|()| std::fs::rename(&tmp, &path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_are_kept_and_listings_expire_after_a_day() {
        let root = crate::test_dir("cache");
        let c = DiskCache::new(Some(&root));
        let t0 = 1_700_000_000;
        c.put(Kind::Release, "123456", r#"{"id":123456}"#, t0);
        c.put(Kind::Listing, "labels/1/releases?page=2", "[1]", t0);
        assert!(root.join("discogs/release/123456.json").exists());

        let later = t0 + 400 * FRESH_SECS;
        let r = c
            .get(Kind::Release, "123456")
            .expect("records are reused indefinitely");
        assert_eq!(r.body, r#"{"id":123456}"#);
        assert!(!r.is_fresh(later), "but their for-sale numbers are stale");
        assert!(r.is_fresh(t0 + FRESH_SECS - 1));

        let key = "labels/1/releases?page=2";
        assert_eq!(c.listing(key, t0 + FRESH_SECS - 1).as_deref(), Some("[1]"));
        assert_eq!(
            c.listing(key, t0 + FRESH_SECS),
            None,
            "24 h on, fetched again"
        );
        assert!(c.get(Kind::Master, "123456").is_none());
    }

    #[test]
    fn no_folder_keeps_nothing() {
        let c = DiskCache::new(None);
        c.put(Kind::Release, "1", "{}", 0);
        assert!(c.get(Kind::Release, "1").is_none());
    }
}
