//! What the user decided while digging, kept in `<config>/dig/memory.ron`: the releases they
//! want, the clips they passed on, and wantlist changes still to be sent.
//!
//! Wanting is per record (a release): the Discogs wantlist holds records. Passing is per clip
//! (a clip id, or the file path for a local entry), so one track of a record can be passed.
//!
//! A wantlist change, or a copy to take out of the collection, that Discogs can't take waits
//! here. While Discogs is offline it waits as
//! long as it takes; after server errors it is tried again 1, 2, 5, 15 and 60 minutes later,
//! then it stops and says why, until the user asks to retry.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};

pub const FILE: &str = "memory.ron";
/// After the first try and each failed retry, the next retry waits this many minutes; after
/// the last one the change has failed.
pub const RETRY_MINUTES: [u64; 5] = [1, 2, 5, 15, 60];
/// While Discogs doesn't answer, a waiting change is tried again this often (seconds).
pub const OFFLINE_RETRY_SECS: u64 = 60;

/// A clip kept before wanting was per record (read from older memory files only).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Kept {
    release: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WantOp {
    Add,
    Remove,
}

impl WantOp {
    fn opposite(self) -> Self {
        match self {
            WantOp::Add => WantOp::Remove,
            WantOp::Remove => WantOp::Add,
        }
    }
}

/// A wantlist change waiting to be sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub release: u64,
    pub op: WantOp,
    /// Server errors so far (offline tries don't count).
    #[serde(default)]
    pub attempts: u32,
    /// Not before this (seconds since the Unix epoch).
    #[serde(default)]
    pub next_at: u64,
    /// Given up after the last retry, with the error: waits for Retry wantlist.
    #[serde(default)]
    pub failed: Option<String>,
}

/// A collection removal waiting to be sent: one copy of `release`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDiscard {
    pub release: u64,
    /// Server errors so far (offline tries don't count).
    #[serde(default)]
    pub attempts: u32,
    /// Not before this (seconds since the Unix epoch).
    #[serde(default)]
    pub next_at: u64,
    /// Given up after the last retry, with the error: waits for Retry remove from collection.
    #[serde(default)]
    pub failed: Option<String>,
}

/// After a try that failed at `now`: offline, wait a minute without counting; otherwise wait
/// for the next retry, or fail after the last one. Returns the error when it has just failed.
fn back_off(
    attempts: &mut u32,
    next_at: &mut u64,
    failed: &mut Option<String>,
    now: u64,
    offline: bool,
    error: &str,
) -> Option<String> {
    if offline {
        *next_at = now + OFFLINE_RETRY_SECS;
        return None;
    }
    *attempts += 1;
    match RETRY_MINUTES.get(*attempts as usize - 1) {
        Some(m) => {
            *next_at = now + m * 60;
            None
        }
        None => {
            *failed = Some(error.to_owned());
            Some(error.to_owned())
        }
    }
}

/// `(release, op)` pairs from older files, or full entries.
#[derive(Deserialize)]
#[serde(untagged)]
enum PendingRepr {
    Old((u64, WantOp)),
    New(Pending),
}

fn pending_list<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Pending>, D::Error> {
    Ok(Vec::<PendingRepr>::deserialize(d)?
        .into_iter()
        .map(|p| match p {
            PendingRepr::Old((release, op)) => Pending {
                release,
                op,
                attempts: 0,
                next_at: 0,
                failed: None,
            },
            PendingRepr::New(p) => p,
        })
        .collect())
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DigMemory {
    /// Releases on the wantlist (the ✓), as far as this app knows.
    pub wanted: BTreeSet<u64>,
    pub passed: BTreeMap<String, u64>,
    /// Wantlist changes not sent yet, oldest first.
    #[serde(deserialize_with = "pending_list")]
    pub wantlist_pending: Vec<Pending>,
    /// Copies to take out of the collection, oldest first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub discards: Vec<PendingDiscard>,
    /// The account whose wantlist the wanted releases were last matched with. Another (or
    /// none) means the local ones are pushed to Discogs; the same means Discogs is followed.
    pub synced_user: Option<String>,
    /// Kept clips from older files: their releases become wanted on load.
    #[serde(skip_serializing)]
    kept: BTreeMap<String, Kept>,
}

impl DigMemory {
    pub fn path(config: &Path) -> PathBuf {
        crate::config::dir(config).join(FILE)
    }

    pub fn load(config: &Path) -> Self {
        let mut m: Self = std::fs::read_to_string(Self::path(config))
            .ok()
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default();
        for k in std::mem::take(&mut m.kept).into_values() {
            m.wanted.extend(k.release);
        }
        m
    }

    pub fn save(&self, config: &Path) -> std::io::Result<()> {
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(std::io::Error::other)?;
        crate::config::write_atomic(&Self::path(config), text.as_bytes(), false)
    }

    pub fn is_wanted(&self, release: u64) -> bool {
        self.wanted.contains(&release)
    }

    pub fn is_passed(&self, key: &str) -> bool {
        self.passed.contains_key(key)
    }

    /// Queues a wantlist change to send now; an Add and a Remove of the same release cancel
    /// out, and a change already waiting starts over.
    pub fn queue_want(&mut self, release: u64, op: WantOp) {
        let opposite = op.opposite();
        if let Some(i) = self
            .wantlist_pending
            .iter()
            .position(|p| p.release == release && p.op == opposite)
        {
            self.wantlist_pending.remove(i);
            return;
        }
        self.wantlist_pending
            .retain(|p| !(p.release == release && p.op == op));
        self.wantlist_pending.push(Pending {
            release,
            op,
            attempts: 0,
            next_at: 0,
            failed: None,
        });
    }

    /// Changes due at `now` (seconds since the epoch), oldest first; failed ones wait.
    pub fn due(&self, now: u64) -> Vec<(u64, WantOp)> {
        self.wantlist_pending
            .iter()
            .filter(|p| p.failed.is_none() && p.next_at <= now)
            .map(|p| (p.release, p.op))
            .collect()
    }

    /// A change is waiting to be sent (not failed).
    pub fn is_want_pending(&self, release: u64) -> bool {
        self.wantlist_pending
            .iter()
            .any(|p| p.release == release && p.failed.is_none())
    }

    /// Why a change for `release` gave up, if one did.
    pub fn want_failure(&self, release: u64) -> Option<&str> {
        self.wantlist_pending
            .iter()
            .find(|p| p.release == release)
            .and_then(|p| p.failed.as_deref())
    }

    /// A change went through (or can't ever: a rejected token, an unknown release).
    pub fn want_done(&mut self, release: u64, op: WantOp) {
        self.wantlist_pending
            .retain(|p| !(p.release == release && p.op == op));
    }

    /// A change couldn't be sent at `now`. Offline, it waits a minute without counting;
    /// otherwise it waits for the next retry, or fails after the last one. Returns the error
    /// when it has just failed.
    pub fn want_failed(
        &mut self,
        release: u64,
        op: WantOp,
        now: u64,
        offline: bool,
        error: &str,
    ) -> Option<String> {
        let p = self
            .wantlist_pending
            .iter_mut()
            .find(|p| p.release == release && p.op == op)?;
        back_off(
            &mut p.attempts,
            &mut p.next_at,
            &mut p.failed,
            now,
            offline,
            error,
        )
    }

    /// Retry wantlist: a failed change for `release` is tried again at once, from the start of
    /// the schedule.
    pub fn retry_want(&mut self, release: u64) {
        for p in self
            .wantlist_pending
            .iter_mut()
            .filter(|p| p.release == release)
        {
            p.attempts = 0;
            p.next_at = 0;
            p.failed = None;
        }
    }

    // ---- collection removals ------------------------------------------------------------

    /// Queues taking one copy of `release` out of the collection; one already waiting for
    /// it stays the only one (a record is removed one copy at a time).
    pub fn queue_discard(&mut self, release: u64) {
        if !self.discards.iter().any(|p| p.release == release) {
            self.discards.push(PendingDiscard {
                release,
                attempts: 0,
                next_at: 0,
                failed: None,
            });
        }
    }

    /// Removals due at `now`, oldest first; failed ones wait.
    pub fn discards_due(&self, now: u64) -> Vec<u64> {
        self.discards
            .iter()
            .filter(|p| p.failed.is_none() && p.next_at <= now)
            .map(|p| p.release)
            .collect()
    }

    /// A removal of `release` is waiting to be sent (not failed).
    pub fn is_discard_pending(&self, release: u64) -> bool {
        self.discards
            .iter()
            .any(|p| p.release == release && p.failed.is_none())
    }

    /// Why a removal of `release` gave up, if one did.
    pub fn discard_failure(&self, release: u64) -> Option<&str> {
        self.discards
            .iter()
            .find(|p| p.release == release)
            .and_then(|p| p.failed.as_deref())
    }

    /// A removal went through (or can't ever).
    pub fn discard_done(&mut self, release: u64) {
        self.discards.retain(|p| p.release != release);
    }

    /// A removal couldn't be sent at `now`, as [`Self::want_failed`] for wantlist changes.
    pub fn discard_failed(
        &mut self,
        release: u64,
        now: u64,
        offline: bool,
        error: &str,
    ) -> Option<String> {
        let p = self.discards.iter_mut().find(|p| p.release == release)?;
        back_off(
            &mut p.attempts,
            &mut p.next_at,
            &mut p.failed,
            now,
            offline,
            error,
        )
    }

    /// Retry remove from collection: tried again at once, from the start of the schedule.
    pub fn retry_discard(&mut self, release: u64) {
        for p in self.discards.iter_mut().filter(|p| p.release == release) {
            p.attempts = 0;
            p.next_at = 0;
            p.failed = None;
        }
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
        m.wanted.insert(123456);
        m.passed.insert("LASTlight01".into(), 1_790_000_002);
        m.queue_want(1004, WantOp::Add);
        m.synced_user = Some("digger".into());
        m.save(&d).unwrap();
        let back = DigMemory::load(&d);
        assert_eq!(back, m);
        assert!(back.is_wanted(123456) && back.is_passed("LASTlight01"));
        assert!(back.is_want_pending(1004));
    }

    #[test]
    fn an_older_file_loads_with_kept_releases_wanted() {
        let d = crate::test_dir("memory-old");
        let path = DigMemory::path(&d);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"(
                kept: {
                    "GLASShouse1": (release: Some(123456), added_to_wantlist: true, at: 1),
                    "/music/local.mp3": (release: None, added_to_wantlist: false, at: 2),
                },
                passed: {"LASTlight01": 3},
                wantlist_pending: [(1004, Add)],
            )"#,
        )
        .unwrap();
        let m = DigMemory::load(&d);
        assert_eq!(m.wanted.iter().copied().collect::<Vec<_>>(), [123456]);
        assert!(m.is_passed("LASTlight01"));
        assert_eq!(m.due(0), [(1004, WantOp::Add)]);
        // Saved again, the old field is gone.
        m.save(&d).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("kept"), "{text}");
        assert_eq!(DigMemory::load(&d), m);
    }

    #[test]
    fn opposite_changes_cancel() {
        let mut m = DigMemory::default();
        m.queue_want(1, WantOp::Add);
        m.queue_want(1, WantOp::Add);
        assert_eq!(m.due(0), [(1, WantOp::Add)]);
        m.queue_want(1, WantOp::Remove);
        assert!(m.wantlist_pending.is_empty());
        m.queue_want(2, WantOp::Remove);
        m.want_done(2, WantOp::Remove);
        assert!(!m.is_want_pending(2));
    }

    #[test]
    fn server_errors_back_off_then_fail() {
        let mut m = DigMemory::default();
        m.queue_want(7, WantOp::Add);
        let mut now = 1_000;
        let mut tries = 0;
        let mut failed = None;
        while failed.is_none() {
            let due = m.due(now);
            if due.is_empty() {
                now += 1;
                continue;
            }
            tries += 1;
            failed = m.want_failed(7, WantOp::Add, now, false, "HTTP 503");
        }
        assert_eq!(tries, 6, "the first try and 5 retries");
        assert_eq!(now - 1_000, (1 + 2 + 5 + 15 + 60) * 60, "about 83 minutes");
        assert_eq!(failed.as_deref(), Some("HTTP 503"));
        assert_eq!(m.want_failure(7), Some("HTTP 503"));
        assert!(m.due(u64::MAX).is_empty(), "nothing more is sent");
        assert!(!m.is_want_pending(7));
        m.retry_want(7);
        assert_eq!(
            m.due(now),
            [(7, WantOp::Add)],
            "Retry wantlist sends it at once"
        );
        assert_eq!(m.want_failure(7), None);
    }

    #[test]
    fn a_collection_removal_waits_backs_off_and_survives_a_restart() {
        let d = crate::test_dir("memory-discard");
        let mut m = DigMemory::default();
        m.queue_discard(1001);
        m.queue_discard(1001);
        assert_eq!(m.discards_due(0), [1001], "one copy at a time");
        assert_eq!(m.discard_failed(1001, 0, true, "offline"), None);
        assert!(m.discards_due(59).is_empty() && m.is_discard_pending(1001));
        m.save(&d).unwrap();
        let mut m = DigMemory::load(&d);
        assert_eq!(m.discards_due(60), [1001], "kept across a restart");
        let mut now = 60;
        let mut failed = None;
        while failed.is_none() {
            if m.discards_due(now).is_empty() {
                now += 60;
                continue;
            }
            failed = m.discard_failed(1001, now, false, "HTTP 503");
        }
        assert_eq!(m.discard_failure(1001), Some("HTTP 503"));
        assert!(!m.is_discard_pending(1001));
        m.retry_discard(1001);
        assert_eq!(m.discards_due(now), [1001]);
        m.discard_done(1001);
        assert!(m.discards.is_empty());
        // An older file, without removals, loads; none are written while there are none.
        let text = ron::to_string(&DigMemory::default()).unwrap();
        assert!(!text.contains("discards"), "{text}");
    }

    #[test]
    fn offline_waits_without_counting() {
        let mut m = DigMemory::default();
        m.queue_want(7, WantOp::Add);
        for i in 0..100 {
            assert_eq!(m.want_failed(7, WantOp::Add, i * 60, true, "offline"), None);
        }
        assert_eq!(m.wantlist_pending[0].attempts, 0);
        assert!(m.due(100 * 60).contains(&(7, WantOp::Add)));
    }
}
