//! What the user decided while digging, kept in `<config>/dig/memory.ron`: kept and passed
//! clips, which releases this app added to the wantlist (only those are ever removed), and
//! wantlist changes still to be sent.
//!
//! Keys are clip ids, or the file path for local entries, so keep and pass work on any entry.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const FILE: &str = "memory.ron";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kept {
    pub release: Option<u64>,
    /// This app put the release on the wantlist (so un-keeping may take it off).
    pub added_to_wantlist: bool,
    /// Seconds since the Unix epoch.
    pub at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WantOp {
    Add,
    Remove,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DigMemory {
    pub kept: BTreeMap<String, Kept>,
    pub passed: BTreeMap<String, u64>,
    /// Wantlist changes that couldn't be sent yet, oldest first.
    pub wantlist_pending: Vec<(u64, WantOp)>,
}

impl DigMemory {
    pub fn path(config: &Path) -> PathBuf {
        crate::config::dir(config).join(FILE)
    }

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

    pub fn is_kept(&self, key: &str) -> bool {
        self.kept.contains_key(key)
    }

    pub fn is_passed(&self, key: &str) -> bool {
        self.passed.contains_key(key)
    }

    /// Whether other kept clips come from `release` (un-keeping one of several leaves the
    /// release on the wantlist).
    pub fn release_kept_elsewhere(&self, release: u64, except: &str) -> bool {
        self.kept
            .iter()
            .any(|(k, v)| k != except && v.release == Some(release))
    }

    /// Whether the app put `release` on the wantlist for any kept clip.
    pub fn added_by_app(&self, release: u64) -> bool {
        self.kept
            .values()
            .any(|v| v.release == Some(release) && v.added_to_wantlist)
    }

    /// Queues a wantlist change; an Add followed by a Remove of the same release cancel out.
    pub fn queue_want(&mut self, release: u64, op: WantOp) {
        let opposite = match op {
            WantOp::Add => WantOp::Remove,
            WantOp::Remove => WantOp::Add,
        };
        if let Some(i) = self
            .wantlist_pending
            .iter()
            .position(|&(r, o)| r == release && o == opposite)
        {
            self.wantlist_pending.remove(i);
        } else if !self.wantlist_pending.contains(&(release, op)) {
            self.wantlist_pending.push((release, op));
        }
    }

    pub fn is_want_pending(&self, release: u64) -> bool {
        self.wantlist_pending.iter().any(|&(r, _)| r == release)
    }

    /// A pending change went through.
    pub fn want_done(&mut self, release: u64, op: WantOp) {
        self.wantlist_pending.retain(|&p| p != (release, op));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_round_trips() {
        let d = crate::test_dir("memory");
        assert_eq!(DigMemory::load(&d), DigMemory::default());
        let mut m = DigMemory::default();
        m.kept.insert(
            "GLASShouse1".into(),
            Kept {
                release: Some(123456),
                added_to_wantlist: true,
                at: 1_790_000_000,
            },
        );
        m.kept.insert(
            "/music/local.mp3".into(),
            Kept {
                release: None,
                added_to_wantlist: false,
                at: 1_790_000_001,
            },
        );
        m.passed.insert("LASTlight01".into(), 1_790_000_002);
        m.queue_want(1004, WantOp::Add);
        m.save(&d).unwrap();
        let back = DigMemory::load(&d);
        assert_eq!(back, m);
        assert!(back.is_kept("GLASShouse1") && back.is_passed("LASTlight01"));
        assert!(back.added_by_app(123456));
        assert!(!back.release_kept_elsewhere(123456, "GLASShouse1"));
        assert!(back.is_want_pending(1004));
    }

    #[test]
    fn opposite_pending_changes_cancel() {
        let mut m = DigMemory::default();
        m.queue_want(1, WantOp::Add);
        m.queue_want(1, WantOp::Add);
        assert_eq!(m.wantlist_pending, [(1, WantOp::Add)]);
        m.queue_want(1, WantOp::Remove);
        assert!(m.wantlist_pending.is_empty());
        m.queue_want(2, WantOp::Remove);
        m.want_done(2, WantOp::Remove);
        assert!(!m.is_want_pending(2));
    }
}
