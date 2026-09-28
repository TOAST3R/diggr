//! Preparing previews before they play: each downloaded preview's score (sections, drops) and
//! waveform overview are computed and cached, so it opens complete and can be navigated from
//! its first second.
//!
//! One preview at a time, in horizon order, on a low-priority thread. The work waits on a gate
//! the UI holds open only while the playing track's own analysis is well under way (32 bars) or
//! nothing plays, so preparing never competes with a track start. Previews whose score and
//! overview are already cached (the same file downloaded again) are skipped. Each prepared
//! preview is reported with its tempo, so its entry can show it before it plays.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

use analysis::cache::{ScoreCache, content_hash};
use platform::{FileSource, Priority, Spawner, TrackRef};

const GATE_POLL: Duration = Duration::from_millis(100);

/// A preview whose score and overview are ready.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub track: TrackRef,
    /// The analysed tempo of its longest steady stretch, if it has a beat.
    pub bpm: Option<f64>,
}

pub struct PrepareHandle {
    requests: Sender<Vec<TrackRef>>,
    done: Receiver<Prepared>,
    gate: Arc<AtomicBool>,
}

impl PrepareHandle {
    /// `cache_dir` is the app's cache folder (scores and overviews live under it).
    pub fn start(
        spawner: &dyn Spawner,
        files: Arc<dyn FileSource>,
        cache_dir: PathBuf,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, platform::PlatformError> {
        let (req_tx, req_rx) = channel::<Vec<TrackRef>>();
        let (done_tx, done_rx) = channel();
        let gate = Arc::new(AtomicBool::new(false));
        let g = gate.clone();
        spawner.spawn(
            "dig-prepare",
            Priority::Low,
            Box::new(move || {
                let scores = ScoreCache::new(&cache_dir);
                let mut prepared: HashSet<TrackRef> = HashSet::new();
                let mut queue: Vec<TrackRef> = Vec::new();
                loop {
                    // The latest horizon replaces the queue.
                    let next = queue.iter().find(|t| !prepared.contains(*t)).cloned();
                    let msg = match next {
                        Some(_) => req_rx.try_recv().map_err(|e| match e {
                            std::sync::mpsc::TryRecvError::Empty => RecvTimeoutError::Timeout,
                            std::sync::mpsc::TryRecvError::Disconnected => {
                                RecvTimeoutError::Disconnected
                            }
                        }),
                        None => req_rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                    };
                    match msg {
                        Ok(q) => {
                            queue = q;
                            continue;
                        }
                        Err(RecvTimeoutError::Disconnected) => return,
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                    let Some(track) = next else { continue };
                    if !wait_for(&g, &req_rx, &mut queue) {
                        continue;
                    }
                    let cached = content_hash(&*files, &track)
                        .and_then(|h| scores.load(h))
                        .filter(|s| s.complete);
                    let score = cached.or_else(|| {
                        let score = analysis::eval::analyze_file(&*files, &track)?;
                        let _ = scores.save(&score);
                        Some(score)
                    });
                    let bpm = score.as_ref().and_then(|s| s.dominant_bpm());
                    if !wait_for(&g, &req_rx, &mut queue) {
                        continue;
                    }
                    if !analysis::overview::is_cached(&*files, &cache_dir, &track) {
                        analysis::overview::precompute(&*files, &cache_dir, &track);
                    }
                    prepared.insert(track.clone());
                    if done_tx.send(Prepared { track, bpm }).is_err() {
                        return;
                    }
                    wake();
                }
            }),
        )?;
        Ok(Self {
            requests: req_tx,
            done: done_rx,
            gate,
        })
    }

    /// The previews to prepare, in horizon order (replaces the previous list).
    pub fn prepare(&self, tracks: Vec<TrackRef>) {
        let _ = self.requests.send(tracks);
    }

    /// Open while the playing track has 32 bars analyzed, or nothing plays.
    pub fn set_gate(&self, open: bool) {
        self.gate.store(open, Ordering::Release);
    }

    pub fn gate(&self) -> Arc<AtomicBool> {
        self.gate.clone()
    }

    /// Previews prepared since the last call.
    pub fn poll(&self) -> Vec<Prepared> {
        self.done.try_iter().collect()
    }
}

/// Waits for the gate. False if a new queue arrived meanwhile (start over with it).
fn wait_for(gate: &AtomicBool, rx: &Receiver<Vec<TrackRef>>, queue: &mut Vec<TrackRef>) -> bool {
    while !gate.load(Ordering::Acquire) {
        match rx.recv_timeout(GATE_POLL) {
            Ok(q) => {
                *queue = q;
                return false;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform::native::{NativeFileSource, NativeSpawner};
    use std::time::Instant;

    #[test]
    fn a_prepared_preview_has_its_score_and_overview_cached_and_waits_for_the_gate() {
        let cache = crate::test_dir("prepare");
        let src = format!(
            "{}/../audio/tests/fixtures/tone.m4a",
            env!("CARGO_MANIFEST_DIR")
        );
        let preview = cache.join("previews/aaaaaaaaaaa.m4a");
        std::fs::create_dir_all(preview.parent().unwrap()).unwrap();
        std::fs::copy(src, &preview).unwrap();
        let track = TrackRef::new(preview.to_string_lossy());
        let files: Arc<dyn FileSource> = Arc::new(NativeFileSource);
        let h = PrepareHandle::start(&NativeSpawner, files.clone(), cache.to_path_buf(), || {})
            .unwrap();

        h.prepare(vec![track.clone()]);
        std::thread::sleep(Duration::from_millis(400));
        assert!(h.poll().is_empty());
        let hash = content_hash(&*files, &track).unwrap();
        assert!(
            ScoreCache::new(cache.path()).load(hash).is_none(),
            "nothing before the gate opens"
        );
        assert!(!analysis::overview::is_cached(&*files, &cache, &track));

        h.set_gate(true);
        let t = Instant::now();
        let done = loop {
            let done = h.poll();
            if !done.is_empty() {
                break done;
            }
            assert!(t.elapsed() < Duration::from_secs(20), "prepared in time");
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(done[0].track, track);
        let score = ScoreCache::new(cache.path())
            .load(hash)
            .expect("score cached");
        assert!(score.complete, "the whole file");
        assert_eq!(done[0].bpm, score.dominant_bpm(), "reported with its tempo");
        assert!(analysis::overview::is_cached(&*files, &cache, &track));
    }
}
