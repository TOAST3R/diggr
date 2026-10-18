//! Preparing previews before they play: each downloaded preview's score (sections, drops) and
//! waveform overview are computed and cached, so it opens complete and can be navigated from
//! its first second.
//!
//! A few previews at a time (one per worker), in queue order, on low-priority threads: each
//! worker takes the first track nobody has prepared or started. The work waits on a gate the UI
//! holds open only while the playing track's own analysis is well under way (32 bars) or
//! nothing plays, so preparing never competes with a track start. Previews whose score and
//! overview are already cached (the same file downloaded again) are skipped. Each prepared
//! preview is reported with its tempo, so its entry can show it before it plays, and the share
//! of each file analyzed so far is readable while it runs.

use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use analysis::cache::{ScoreCache, content_hash};
use platform::{FileSource, Priority, Spawner, TrackRef};

const GATE_POLL: Duration = Duration::from_millis(100);
/// How often the UI is woken while a file is analyzed, so its bar moves.
const PROGRESS_WAKE: Duration = Duration::from_millis(500);

/// A preview whose score and overview are ready.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub track: TrackRef,
    /// The analysed tempo of its longest steady stretch, if it has a beat.
    pub bpm: Option<f64>,
}

/// The latest queue, and which of its tracks are done or being worked on.
#[derive(Default)]
struct Queue {
    tracks: Vec<TrackRef>,
    prepared: HashSet<TrackRef>,
    busy: HashSet<TrackRef>,
    closed: bool,
}

/// What the workers and the handle share (UI and workers only, never audio).
#[derive(Default)]
struct Shared {
    queue: Mutex<Queue>,
    changed: Condvar,
    gate: AtomicBool,
    /// The tracks being analyzed and the share of each done.
    progress: Mutex<HashMap<TrackRef, f32>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shared {
    /// Waits for the gate and a track nobody has prepared or started, and claims it. `None`
    /// once the handle is gone.
    fn next(&self) -> Option<TrackRef> {
        let mut q = lock(&self.queue);
        loop {
            if q.closed {
                return None;
            }
            if self.gate.load(Ordering::Acquire) {
                let free = q
                    .tracks
                    .iter()
                    .find(|t| !q.prepared.contains(*t) && !q.busy.contains(*t))
                    .cloned();
                if let Some(t) = free {
                    q.busy.insert(t.clone());
                    return Some(t);
                }
            }
            q = self
                .changed
                .wait_timeout(q, GATE_POLL)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// Waits for the gate; false once the handle is gone.
    fn wait_gate(&self) -> bool {
        let mut q = lock(&self.queue);
        while !self.gate.load(Ordering::Acquire) {
            if q.closed {
                return false;
            }
            q = self
                .changed
                .wait_timeout(q, GATE_POLL)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        !q.closed
    }

    fn finish(&self, track: &TrackRef) {
        let mut q = lock(&self.queue);
        q.busy.remove(track);
        q.prepared.insert(track.clone());
        lock(&self.progress).remove(track);
    }
}

pub struct PrepareHandle {
    shared: Arc<Shared>,
    done: Receiver<Prepared>,
}

impl PrepareHandle {
    /// `cache_dir` is the app's cache folder (scores and overviews live under it). Half the
    /// cores work, at most 3.
    pub fn start(
        spawner: &dyn Spawner,
        files: Arc<dyn FileSource>,
        cache_dir: PathBuf,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, platform::PlatformError> {
        let workers = std::thread::available_parallelism().map_or(1, |n| (n.get() / 2).clamp(1, 3));
        Self::start_with(spawner, files, cache_dir, workers, wake)
    }

    /// [`Self::start`] with this many workers.
    pub fn start_with(
        spawner: &dyn Spawner,
        files: Arc<dyn FileSource>,
        cache_dir: PathBuf,
        workers: usize,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, platform::PlatformError> {
        let shared = Arc::new(Shared::default());
        let (done_tx, done_rx) = channel();
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        for i in 0..workers.max(1) {
            let (sh, files, dir, done, wake) = (
                shared.clone(),
                files.clone(),
                cache_dir.clone(),
                done_tx.clone(),
                wake.clone(),
            );
            spawner.spawn(
                &format!("dig-prepare-{i}"),
                Priority::Low,
                Box::new(move || work(&sh, &*files, &dir, &done, &*wake)),
            )?;
        }
        Ok(Self {
            shared,
            done: done_rx,
        })
    }

    /// The tracks being analyzed and the share of each done (0–1).
    pub fn progress(&self) -> HashMap<TrackRef, f32> {
        lock(&self.shared.progress).clone()
    }

    /// The previews to prepare, in queue order (replaces the previous list).
    pub fn prepare(&self, tracks: Vec<TrackRef>) {
        lock(&self.shared.queue).tracks = tracks;
        self.shared.changed.notify_all();
    }

    /// Open while the playing track has 32 bars analyzed, or nothing plays.
    pub fn set_gate(&self, open: bool) {
        if self.shared.gate.swap(open, Ordering::AcqRel) != open {
            self.shared.changed.notify_all();
        }
    }

    /// Previews prepared since the last call.
    pub fn poll(&self) -> Vec<Prepared> {
        self.done.try_iter().collect()
    }
}

impl Drop for PrepareHandle {
    fn drop(&mut self) {
        lock(&self.shared.queue).closed = true;
        self.shared.changed.notify_all();
    }
}

/// One worker: prepares tracks from the shared queue until the handle is gone.
fn work(
    sh: &Shared,
    files: &dyn FileSource,
    cache_dir: &Path,
    done: &Sender<Prepared>,
    wake: &dyn Fn(),
) {
    let scores = ScoreCache::new(cache_dir);
    while let Some(track) = sh.next() {
        let set = |p: f32| {
            lock(&sh.progress).insert(track.clone(), p);
        };
        // A bug on one file passes it (no tempo) instead of ending every analysis after it.
        let score = catch_unwind(AssertUnwindSafe(|| {
            let cached = content_hash(files, &track)
                .and_then(|h| scores.load(h))
                .filter(|s| s.complete);
            cached.or_else(|| {
                set(0.0);
                let mut woke = Instant::now();
                let score = analysis::eval::analyze_file_with(files, &track, |p| {
                    set(p);
                    if woke.elapsed() >= PROGRESS_WAKE {
                        woke = Instant::now();
                        wake();
                    }
                })?;
                let _ = scores.save(&score);
                Some(score)
            })
        }))
        .unwrap_or(None);
        let bpm = score.as_ref().and_then(|s| s.dominant_bpm());
        if !sh.wait_gate() {
            return;
        }
        let _ = catch_unwind(AssertUnwindSafe(|| {
            if !analysis::overview::is_cached(files, cache_dir, &track) {
                analysis::overview::precompute(files, cache_dir, &track);
            }
        }));
        sh.finish(&track);
        if done.send(Prepared { track, bpm }).is_err() {
            return;
        }
        wake();
    }
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

    /// A mono 16-bit WAV of `secs` of a synthesized 126 BPM track, long enough to watch.
    fn long_track(dir: &std::path::Path, name: &str, secs: usize) -> TrackRef {
        let rate = 22_050u32;
        let s = analysis::synth::render(&analysis::synth::standard_track(126.0), rate, 7);
        let pcm: Vec<u8> = s.samples[..secs * rate as usize]
            .iter()
            .flat_map(|&x| ((x * 32_000.0) as i16).to_le_bytes())
            .collect();
        let mut wav = Vec::new();
        for part in [
            &b"RIFF"[..],
            &(36 + pcm.len() as u32).to_le_bytes(),
            b"WAVEfmt ",
            &16u32.to_le_bytes(),
            &1u16.to_le_bytes(),
            &1u16.to_le_bytes(),
            &rate.to_le_bytes(),
            &(rate * 2).to_le_bytes(),
            &2u16.to_le_bytes(),
            &16u16.to_le_bytes(),
            b"data",
            &(pcm.len() as u32).to_le_bytes(),
            &pcm,
        ] {
            wav.extend_from_slice(part);
        }
        let path = dir.join(name);
        std::fs::write(&path, wav).unwrap();
        TrackRef::new(path.to_string_lossy())
    }

    fn tone(dir: &std::path::Path, name: &str) -> TrackRef {
        let src = format!(
            "{}/../audio/tests/fixtures/tone.m4a",
            env!("CARGO_MANIFEST_DIR")
        );
        let path = dir.join(name);
        std::fs::copy(src, &path).unwrap();
        TrackRef::new(path.to_string_lossy())
    }

    /// Polls until `n` previews are prepared, noting the progress seen meanwhile.
    fn collect(
        h: &PrepareHandle,
        n: usize,
        mut seen: impl FnMut(HashMap<TrackRef, f32>),
    ) -> Vec<Prepared> {
        let t = Instant::now();
        let mut done = Vec::new();
        while done.len() < n {
            seen(h.progress());
            done.extend(h.poll());
            assert!(t.elapsed() < Duration::from_secs(60), "prepared in time");
            std::thread::sleep(Duration::from_millis(5));
        }
        done
    }

    #[test]
    fn progress_shows_while_a_file_is_analyzed_and_clears_after_it() {
        let cache = crate::test_dir("prepare-progress");
        let track = long_track(&cache, "long.wav", 30);
        let files: Arc<dyn FileSource> = Arc::new(NativeFileSource);
        let h = PrepareHandle::start(&NativeSpawner, files, cache.to_path_buf(), || {}).unwrap();
        h.set_gate(true);
        h.prepare(vec![track.clone()]);
        let mut shares: Vec<f32> = Vec::new();
        let done = collect(&h, 1, |p| shares.extend(p.get(&track).copied()));
        assert_eq!(done[0].track, track);
        assert!(shares.iter().any(|&s| s > 0.0 && s < 1.0), "{shares:?}");
        assert!(h.progress().is_empty(), "cleared once prepared");
    }

    /// Reads files, except that opening "bad.m4a" panics (an analyzer bug on one file).
    struct Panicky;

    impl FileSource for Panicky {
        fn open(
            &self,
            track: &TrackRef,
        ) -> Result<Box<dyn platform::MediaSource>, platform::PlatformError> {
            assert!(!format!("{track:?}").contains("bad.m4a"), "a bug");
            NativeFileSource.open(track)
        }
    }

    #[test]
    fn a_file_that_fails_is_passed_and_the_rest_are_still_prepared() {
        let cache = crate::test_dir("prepare-panic");
        let bad = tone(&cache, "bad.m4a");
        let good = tone(&cache, "good.m4a");
        let h = PrepareHandle::start_with(
            &NativeSpawner,
            Arc::new(Panicky),
            cache.to_path_buf(),
            1,
            || {},
        )
        .unwrap();
        h.set_gate(true);
        h.prepare(vec![bad.clone(), good.clone()]);
        let done = collect(&h, 2, |_| {});
        assert_eq!(
            done[0],
            Prepared {
                track: bad,
                bpm: None
            },
            "passed, without a tempo"
        );
        assert_eq!(done[1].track, good, "the worker carried on");
    }

    #[test]
    fn a_track_put_at_the_front_goes_right_after_the_one_in_progress() {
        let cache = crate::test_dir("prepare-order");
        let a = long_track(&cache, "a.wav", 30);
        let b = tone(&cache, "b.m4a");
        let c = tone(&cache, "c.m4a");
        let files: Arc<dyn FileSource> = Arc::new(NativeFileSource);
        let h = PrepareHandle::start_with(&NativeSpawner, files, cache.to_path_buf(), 1, || {})
            .unwrap();
        h.set_gate(true);
        h.prepare(vec![a.clone(), b.clone(), c.clone()]);
        let t = Instant::now();
        while !h.progress().contains_key(&a) {
            assert!(t.elapsed() < Duration::from_secs(20), "a starts");
            std::thread::sleep(Duration::from_millis(2));
        }
        h.prepare(vec![c.clone(), a.clone(), b.clone()]);
        let order: Vec<TrackRef> = collect(&h, 3, |_| {})
            .into_iter()
            .map(|p| p.track)
            .collect();
        assert_eq!(order, [a, c, b]);
    }

    #[test]
    fn workers_analyze_several_files_at_once() {
        let cache = crate::test_dir("prepare-parallel");
        let a = long_track(&cache, "a.wav", 30);
        let b = long_track(&cache, "b.wav", 25);
        let files: Arc<dyn FileSource> = Arc::new(NativeFileSource);
        let h = PrepareHandle::start_with(&NativeSpawner, files, cache.to_path_buf(), 2, || {})
            .unwrap();
        h.set_gate(true);
        h.prepare(vec![a.clone(), b.clone()]);
        let mut both = false;
        let done = collect(&h, 2, |p| both |= p.len() == 2);
        assert!(both, "both analyzed at the same time");
        let mut got: Vec<TrackRef> = done.into_iter().map(|p| p.track).collect();
        got.sort_by_key(|t| format!("{t:?}"));
        assert_eq!(got, [a, b], "each once");
    }
}
