//! `AnalysisService`: the player-facing API. The player reports the audible track and position
//! every frame and pre-warms the next track; workers analyze ahead on low-priority threads and
//! publish immutable `SongScore` snapshots that readers get without ever blocking.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use arc_swap::{ArcSwap, ArcSwapOption};
use platform::{FileSource, Priority, Spawner, TrackRef};

use crate::assemble::{Piece, assemble, empty};
use crate::cache::{ScoreCache, content_hash};
use crate::region::{Region, UPDATE_EVERY_SECS};
use crate::score::SongScore;
use crate::source::Source;

/// Keep this much analyzed ahead of the playhead.
pub const HORIZON_SECS: f64 = 120.0;
/// Start this far before a seek target, for context.
pub const LEAD_IN_SECS: f64 = 8.0;
/// At most this many tracks decode at once (the audible one and the pre-warmed one).
pub const MAX_DECODERS: usize = 2;
/// A track nobody asked about for this long stops being analyzed (results are kept and cached).
pub const IDLE_STOP: Duration = Duration::from_secs(30);
const SAVE_EVERY: Duration = Duration::from_secs(30);

struct TrackState {
    score: ArcSwapOption<SongScore>,
    playhead: AtomicU64,
    touched_ms: AtomicU64,
    prewarm_ms: AtomicU64,
    worker_alive: AtomicBool,
}

struct Shared {
    spawner: Arc<dyn Spawner>,
    files: Arc<dyn FileSource>,
    cache: Option<ScoreCache>,
    tracks: ArcSwap<HashMap<TrackRef, Arc<TrackState>>>,
    decoders: AtomicUsize,
    epoch: Instant,
}

impl Shared {
    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64 + 1
    }
}

#[derive(Clone)]
pub struct AnalysisService {
    shared: Arc<Shared>,
}

impl AnalysisService {
    pub fn new(
        spawner: Arc<dyn Spawner>,
        files: Arc<dyn FileSource>,
        cache: Option<ScoreCache>,
    ) -> Self {
        Self {
            shared: Arc::new(Shared {
                spawner,
                files,
                cache,
                tracks: ArcSwap::from_pointee(HashMap::new()),
                decoders: AtomicUsize::new(0),
                epoch: Instant::now(),
            }),
        }
    }

    /// The audible track and position; call every frame (cheap: no locks, no I/O).
    pub fn playhead(&self, track: &TrackRef, secs: f64) {
        let st = self.state(track);
        st.playhead
            .store(secs.max(0.0).to_bits(), Ordering::Relaxed);
        st.touched_ms.store(self.shared.now_ms(), Ordering::Relaxed);
    }

    /// A track that is about to play (from `EngineEvent::PreWarm`): analyze its opening now.
    pub fn prewarm(&self, track: &TrackRef) {
        self.state(track)
            .prewarm_ms
            .store(self.shared.now_ms(), Ordering::Relaxed);
    }

    /// The latest snapshot for a track (never blocks).
    pub fn score(&self, track: &TrackRef) -> Option<Arc<SongScore>> {
        self.shared.tracks.load().get(track)?.score.load_full()
    }

    /// Tracks with a live worker (for tests and diagnostics).
    pub fn active_workers(&self) -> usize {
        self.shared
            .tracks
            .load()
            .values()
            .filter(|s| s.worker_alive.load(Ordering::Relaxed))
            .count()
    }

    fn state(&self, track: &TrackRef) -> Arc<TrackState> {
        let st = match self.shared.tracks.load().get(track) {
            Some(st) => st.clone(),
            None => {
                let fresh = Arc::new(TrackState {
                    score: ArcSwapOption::empty(),
                    playhead: AtomicU64::new(0f64.to_bits()),
                    touched_ms: AtomicU64::new(0),
                    prewarm_ms: AtomicU64::new(0),
                    worker_alive: AtomicBool::new(false),
                });
                let mut inserted = fresh.clone();
                self.shared.tracks.rcu(|map| {
                    let mut m = HashMap::clone(map);
                    inserted = m
                        .entry(track.clone())
                        .or_insert_with(|| fresh.clone())
                        .clone();
                    m
                });
                inserted
            }
        };
        if !st.worker_alive.swap(true, Ordering::AcqRel) {
            let (shared, st2, track) = (self.shared.clone(), st.clone(), track.clone());
            let spawned = self.shared.spawner.spawn(
                "analysis",
                Priority::Low,
                Box::new(move || {
                    Worker::new(shared, st2.clone(), track).run();
                    st2.worker_alive.store(false, Ordering::Release);
                }),
            );
            if spawned.is_err() {
                st.worker_alive.store(false, Ordering::Release);
            }
        }
        st
    }
}

struct Live {
    region: Region,
    source: Option<Source>,
}

struct Worker {
    shared: Arc<Shared>,
    st: Arc<TrackState>,
    track: TrackRef,
    hash: u64,
    frozen: Vec<Piece>,
    live: Vec<Live>,
    duration: Option<f64>,
    last_save: Instant,
}

impl Worker {
    fn new(shared: Arc<Shared>, st: Arc<TrackState>, track: TrackRef) -> Self {
        Self {
            shared,
            st,
            track,
            hash: 0,
            frozen: Vec::new(),
            live: Vec::new(),
            duration: None,
            last_save: Instant::now(),
        }
    }

    /// Publishes the current score. A complete score is written to the cache *before* readers
    /// can see it, so anyone who observes `complete` (e.g. the next app start) finds it cached.
    fn publish(&mut self) -> Arc<SongScore> {
        let mut pieces = self.frozen.clone();
        pieces.extend(self.live.iter().map(|l| Piece::from(l.region.latest())));
        let score = Arc::new(if pieces.is_empty() {
            empty(self.hash)
        } else {
            assemble(pieces, self.hash)
        });
        if score.complete {
            self.save_score(&score);
        }
        self.st.score.store(Some(score.clone()));
        score
    }

    fn save_score(&mut self, score: &SongScore) {
        if let Some(cache) = &self.shared.cache
            && !score.coverage.spans().is_empty()
        {
            let _ = cache.save(score);
        }
        self.last_save = Instant::now();
    }

    /// Saves the latest published score (partial coverage included).
    fn save(&mut self) {
        if let Some(score) = self.st.score.load_full() {
            self.save_score(&score);
        }
    }

    fn run(mut self) {
        self.hash = content_hash(&*self.shared.files, &self.track).unwrap_or(0);
        if let Some(cached) = self.shared.cache.as_ref().and_then(|c| c.load(self.hash)) {
            let done = cached.complete;
            self.frozen.push(Piece::from(&cached));
            self.publish();
            if done {
                return;
            }
        } else {
            self.publish();
        }
        loop {
            let now = self.shared.now_ms();
            let touched = self.st.touched_ms.load(Ordering::Relaxed);
            let pre = self.st.prewarm_ms.load(Ordering::Relaxed);
            let last = touched.max(pre);
            if now.saturating_sub(last) > IDLE_STOP.as_millis() as u64 {
                break;
            }
            // Pre-warmed and not yet playing: analyze from the start.
            let playing = touched > 0 && now - touched < 3_000;
            let p = if playing {
                f64::from_bits(self.st.playhead.load(Ordering::Relaxed))
            } else {
                0.0
            };
            let worked = self.step(p);
            if self.last_save.elapsed() > SAVE_EVERY {
                self.save();
            }
            if !worked {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        self.save();
    }

    /// Decodes one chunk of the region that serves playhead `p`, if any is needed.
    fn step(&mut self, p: f64) -> bool {
        let score = self
            .st
            .score
            .load_full()
            .unwrap_or_else(|| Arc::new(empty(self.hash)));
        if score.complete {
            return false;
        }
        // Where analysis must continue from so that [p, p + horizon) gets covered.
        let from = match score.coverage.covered_until(p) {
            // Near the end of the file keep reading until the decoder reports the end, which
            // is what marks the score complete.
            Some(e) if e >= p + HORIZON_SECS - 0.01 => return false,
            Some(e) => e,
            None => (p - LEAD_IN_SECS).max(0.0),
        };
        // Continue a live region ending near `from`, or start one there.
        let idx = match self
            .live
            .iter()
            .position(|l| l.source.is_some() && (l.region.end() - from).abs() < 1.0)
        {
            Some(i) => i,
            None => {
                let start = if score.coverage.contains(from - 0.5) {
                    from
                } else {
                    (p - LEAD_IN_SECS).max(0.0).min(from)
                };
                match Source::open(&*self.shared.files, &self.track, start) {
                    Ok(src) => {
                        self.duration = self.duration.or(src.duration());
                        self.live.push(Live {
                            region: Region::new(start),
                            source: Some(src),
                        });
                        self.live.len() - 1
                    }
                    Err(_) => return false,
                }
            }
        };
        if self.shared.decoders.fetch_add(1, Ordering::AcqRel) >= MAX_DECODERS {
            self.shared.decoders.fetch_sub(1, Ordering::AcqRel);
            return false;
        }
        let live = &mut self.live[idx];
        let chunk = live.source.as_mut().and_then(|s| s.read(UPDATE_EVERY_SECS));
        match chunk {
            Some(c) => {
                live.region.push(&c);
                live.region.update(false);
            }
            None => {
                live.region.update(true);
                live.source = None;
            }
        }
        self.shared.decoders.fetch_sub(1, Ordering::AcqRel);
        // Stop a region once it runs into the next analyzed stretch.
        let end = self.live[idx].region.end();
        let start = self.live[idx].region.start();
        if score
            .coverage
            .spans()
            .iter()
            .any(|&(s, _)| s > start + 1.0 && s <= end)
        {
            self.live[idx].source = None;
        }
        self.publish();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::SectionKind;
    use crate::synth;
    use platform::native::{NativeFileSource, NativeSpawner};

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("analysis-svc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Writes a synthetic track as a 44.1 kHz stereo WAV (the analyzer resamples it).
    fn write_track(
        dir: &std::path::Path,
        name: &str,
        parts: &[synth::Part],
    ) -> (TrackRef, synth::Synth) {
        let s = synth::render(parts, 44_100, 21);
        let path = dir.join(name);
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for &x in &s.samples {
            let v = (x * 32_000.0) as i16;
            w.write_sample(v).unwrap();
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
        (TrackRef::new(path.to_string_lossy()), s)
    }

    fn service(cache: &std::path::Path) -> AnalysisService {
        AnalysisService::new(
            Arc::new(NativeSpawner),
            Arc::new(NativeFileSource),
            Some(ScoreCache::new(cache)),
        )
    }

    fn wait(mut f: impl FnMut() -> bool, secs: f64) -> Duration {
        let t = Instant::now();
        while !f() {
            assert!(t.elapsed().as_secs_f64() < secs, "timed out after {secs} s");
            std::thread::sleep(Duration::from_millis(10));
        }
        t.elapsed()
    }

    #[test]
    fn first_32_bars_quickly_then_cached_for_the_next_play() {
        let dir = tmp("first");
        let (track, s) = write_track(&dir, "a.wav", &synth::standard_track(125.0));
        let bars32 = 32.0 * 4.0 * 60.0 / 125.0;
        let svc = service(&dir.join("cache"));
        let took = wait(
            || {
                svc.playhead(&track, 1.0);
                svc.score(&track)
                    .is_some_and(|sc| sc.coverage.covered_until(0.0).is_some_and(|e| e >= bars32))
            },
            60.0,
        );
        eprintln!(
            "first 32 bars covered after {:.0} ms",
            took.as_secs_f64() * 1e3
        );
        // Analysis stops 120 s ahead of the playhead; once playback is past 60 s the rest of this
        // 168 s track is in range, gets finished, and is saved to the cache.
        wait(
            || {
                svc.playhead(&track, 60.0);
                svc.score(&track).is_some_and(|sc| sc.complete)
            },
            60.0,
        );
        let full = svc.score(&track).unwrap();
        assert_eq!(full.beats.len(), s.beats.len());
        assert_eq!(
            full.sections.iter().map(|x| x.kind).collect::<Vec<_>>(),
            s.sections.iter().map(|x| x.1).collect::<Vec<_>>()
        );

        // A new service (next app start) gets the complete score without analyzing.
        let svc2 = service(&dir.join("cache"));
        let took = wait(
            || {
                svc2.playhead(&track, 0.0);
                svc2.score(&track).is_some_and(|sc| sc.complete)
            },
            60.0,
        );
        assert!(took < Duration::from_secs(2), "cached score after {took:?}");
        assert_eq!(*svc2.score(&track).unwrap(), *full);
    }

    #[test]
    fn seeking_far_ahead_starts_analysis_there() {
        let dir = tmp("seek");
        let parts: Vec<synth::Part> = (0..3).flat_map(|_| synth::standard_track(128.0)).collect(); // ≈ 8 min
        let (track, _) = write_track(&dir, "mix.wav", &parts);
        let svc = service(&dir.join("cache"));
        svc.playhead(&track, 0.0);
        wait(
            || svc.score(&track).is_some_and(|sc| !sc.beats.is_empty()),
            60.0,
        );
        let target = 400.0;
        let took = wait(
            || {
                svc.playhead(&track, target);
                svc.score(&track)
                    .is_some_and(|sc| sc.coverage.contains(target + 10.0))
            },
            60.0,
        );
        eprintln!("seek coverage after {:.0} ms", took.as_secs_f64() * 1e3);
        let sc = svc.score(&track).unwrap();
        assert!(
            sc.coverage.spans().len() >= 2,
            "a second span starting near the seek: {:?}",
            sc.coverage.spans()
        );
        assert!(
            sc.beat_at(target + 5.0).is_some(),
            "beats are available right after the seek"
        );
    }

    #[test]
    fn prewarm_analyzes_the_next_track_before_it_plays() {
        let dir = tmp("prewarm");
        let (next, _) = write_track(&dir, "next.wav", &synth::standard_track(126.0));
        let svc = service(&dir.join("cache"));
        svc.prewarm(&next);
        let bars32 = 32.0 * 4.0 * 60.0 / 126.0;
        wait(
            || {
                svc.score(&next)
                    .is_some_and(|sc| sc.coverage.covered_until(0.0).is_some_and(|e| e >= bars32))
            },
            60.0,
        );
        let sc = svc.score(&next).unwrap();
        assert!(sc.sections.iter().any(|x| x.kind == SectionKind::Drop));
    }

    #[test]
    fn idles_once_the_horizon_is_covered() {
        let dir = tmp("horizon");
        let (track, _) = write_track(&dir, "a.wav", &synth::standard_track(125.0)); // 168 s
        let svc = service(&dir.join("cache"));
        wait(
            || {
                svc.playhead(&track, 1.0);
                svc.score(&track)
                    .is_some_and(|sc| sc.coverage.covered_until(0.0).is_some_and(|e| e >= 121.0))
            },
            60.0,
        );
        std::thread::sleep(Duration::from_millis(500));
        svc.playhead(&track, 1.0);
        let e = svc
            .score(&track)
            .unwrap()
            .coverage
            .covered_until(0.0)
            .unwrap();
        assert!(
            e < 121.0 + UPDATE_EVERY_SECS + 0.5,
            "analyzed to {e:.1} s with the playhead at 1 s"
        );
        assert!(!svc.score(&track).unwrap().complete);
    }

    #[test]
    fn reads_never_block_while_workers_publish() {
        let dir = tmp("reads");
        let (track, _) = write_track(&dir, "a.wav", &synth::standard_track(128.0));
        let svc = service(&dir.join("cache"));
        let mut times = Vec::new();
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(2) {
            let t = Instant::now();
            svc.playhead(&track, 60.0);
            let _ = svc.score(&track);
            times.push(t.elapsed());
            std::thread::sleep(Duration::from_micros(200));
        }
        times.sort();
        let p99 = times[times.len() * 99 / 100];
        let worst = *times.last().unwrap();
        eprintln!(
            "playhead+score over {} calls: p99 {p99:?}, worst {worst:?}",
            times.len()
        );
        // Per-frame cost stays far below a frame (16 ms). The single worst call can include the
        // one-time worker spawn or an OS preemption (other tests load every core), so it only
        // gets a loose bound that a blocking wait on analysis work would still exceed.
        assert!(p99 < Duration::from_millis(1), "p99 {p99:?}");
        assert!(worst < Duration::from_millis(50), "worst {worst:?}");
        assert!(svc.active_workers() <= MAX_DECODERS);
    }
}
