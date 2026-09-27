//! The playback clock: which frame is at the speaker right now.
//!
//! The audio callback publishes a snapshot per buffer through a seqlock of plain atomics (no
//! locks, no allocation). Readers interpolate between callbacks using the monotonic time base
//! of the sink, compensating output latency and handling a gapless track boundary inside the
//! buffer.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering, fence};

use crate::{PlayState, TrackId};

/// Maximum user A/V offset, in milliseconds.
pub const MAX_OFFSET_MS: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClockSnapshot {
    /// Track of the first frame in the buffer (0 = none).
    pub track: TrackId,
    /// Track frame of the first frame in the buffer.
    pub frame: u64,
    /// Callback start, in the sink's monotonic time base.
    pub host_ns: u64,
    /// Callback start → first frame audible.
    pub latency_ns: u64,
    pub sample_rate: u32,
    pub state: PlayState,
    /// From this buffer offset on, a different run is audible (gapless next track, or new
    /// position after a seek): `(track, buffer offset in frames, track frame at that offset)`.
    pub boundary: Option<(TrackId, u32, u64)>,
    /// No audio was available for this buffer (loading/seeking); readers freeze the position.
    pub starved: bool,
    pub buffer_frames: u32,
    /// Changes on seek and on device changes.
    pub epoch: u64,
    /// The epoch of positions at or past `boundary`, when it differs from `epoch` (a splice
    /// inside this buffer: positions after it may move backwards).
    pub boundary_epoch: Option<u64>,
}

#[derive(Default)]
struct Shared {
    seq: AtomicU64,
    track: AtomicU64,
    frame: AtomicU64,
    host_ns: AtomicU64,
    latency_ns: AtomicU64,
    sample_rate: AtomicU64,
    state: AtomicU64,
    boundary_track: AtomicU64,
    boundary_at: AtomicU64,
    boundary_frame: AtomicU64,
    starved: AtomicU64,
    buffer_frames: AtomicU64,
    epoch: AtomicU64,
    boundary_epoch: AtomicU64,
    offset_ns: AtomicI64,
}

pub fn clock() -> (ClockWriter, ClockReader) {
    let shared = Arc::new(Shared::default());
    (
        ClockWriter {
            shared: shared.clone(),
        },
        ClockReader { shared, last: None },
    )
}

/// Owned by the audio callback.
pub struct ClockWriter {
    shared: Arc<Shared>,
}

impl ClockWriter {
    pub fn publish(&self, s: &ClockSnapshot) {
        let sh = &*self.shared;
        let seq = sh.seq.load(Ordering::Relaxed);
        sh.seq.store(seq.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        sh.track.store(s.track, Ordering::Relaxed);
        sh.frame.store(s.frame, Ordering::Relaxed);
        sh.host_ns.store(s.host_ns, Ordering::Relaxed);
        sh.latency_ns.store(s.latency_ns, Ordering::Relaxed);
        sh.sample_rate
            .store(s.sample_rate as u64, Ordering::Relaxed);
        sh.state.store(s.state as u64, Ordering::Relaxed);
        let (bt, ba, bf) = s
            .boundary
            .map_or((0, u64::MAX, 0), |(t, at, f)| (t, at as u64, f));
        sh.boundary_track.store(bt, Ordering::Relaxed);
        sh.boundary_at.store(ba, Ordering::Relaxed);
        sh.boundary_frame.store(bf, Ordering::Relaxed);
        sh.starved.store(s.starved as u64, Ordering::Relaxed);
        sh.buffer_frames
            .store(s.buffer_frames as u64, Ordering::Relaxed);
        sh.epoch.store(s.epoch, Ordering::Relaxed);
        sh.boundary_epoch
            .store(s.boundary_epoch.unwrap_or(u64::MAX), Ordering::Relaxed);
        sh.seq.store(seq.wrapping_add(2), Ordering::Release);
    }
}

/// Where playback is, as heard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub track: TrackId,
    pub frame: u64,
    pub sample_rate: u32,
    pub state: PlayState,
    /// True if a seek or device change happened since this reader's previous call.
    pub discontinuity: bool,
}

impl Position {
    pub fn seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.frame as f64 / self.sample_rate as f64
        }
    }
}

/// Cheap to clone; each clone keeps its own monotonicity state.
#[derive(Clone)]
pub struct ClockReader {
    shared: Arc<Shared>,
    last: Option<(TrackId, u64, u64)>,
}

impl ClockReader {
    /// A consistent copy of the latest published snapshot.
    pub fn snapshot(&self) -> ClockSnapshot {
        let sh = &*self.shared;
        loop {
            let s1 = sh.seq.load(Ordering::Acquire);
            if s1 & 1 == 1 {
                std::hint::spin_loop();
                continue;
            }
            let boundary_at = sh.boundary_at.load(Ordering::Relaxed);
            let snap = ClockSnapshot {
                track: sh.track.load(Ordering::Relaxed),
                frame: sh.frame.load(Ordering::Relaxed),
                host_ns: sh.host_ns.load(Ordering::Relaxed),
                latency_ns: sh.latency_ns.load(Ordering::Relaxed),
                sample_rate: sh.sample_rate.load(Ordering::Relaxed) as u32,
                state: PlayState::from_u8(sh.state.load(Ordering::Relaxed) as u8),
                boundary: (boundary_at != u64::MAX).then(|| {
                    (
                        sh.boundary_track.load(Ordering::Relaxed),
                        boundary_at as u32,
                        sh.boundary_frame.load(Ordering::Relaxed),
                    )
                }),
                starved: sh.starved.load(Ordering::Relaxed) != 0,
                buffer_frames: sh.buffer_frames.load(Ordering::Relaxed) as u32,
                epoch: sh.epoch.load(Ordering::Relaxed),
                boundary_epoch: Some(sh.boundary_epoch.load(Ordering::Relaxed))
                    .filter(|&e| e != u64::MAX),
            };
            fence(Ordering::Acquire);
            if sh.seq.load(Ordering::Relaxed) == s1 {
                return snap;
            }
        }
    }

    /// User A/V offset applied to every reader: positive values report later positions.
    pub fn set_offset_ms(&self, ms: i64) {
        let ms = ms.clamp(-MAX_OFFSET_MS, MAX_OFFSET_MS);
        self.shared
            .offset_ns
            .store(ms * 1_000_000, Ordering::Relaxed);
    }

    pub fn offset_ms(&self) -> i64 {
        self.shared.offset_ns.load(Ordering::Relaxed) / 1_000_000
    }

    /// The audible position at `now_ns` (sink time base).
    pub fn position(&mut self, now_ns: u64) -> Position {
        let snap = self.snapshot();
        let offset_ns = self.shared.offset_ns.load(Ordering::Relaxed);
        let (mut pos, past_boundary) = compute(&snap, now_ns, offset_ns);
        let epoch = match snap.boundary_epoch {
            Some(e) if past_boundary => e,
            _ => snap.epoch,
        };
        match self.last {
            Some((track, frame, last_epoch)) if last_epoch == epoch => {
                if track == pos.track && pos.frame < frame {
                    pos.frame = frame;
                }
            }
            Some(_) => pos.discontinuity = true,
            None => {}
        }
        self.last = Some((pos.track, pos.frame, epoch));
        pos
    }
}

/// The position, and whether it lies at or past the snapshot's in-buffer boundary.
fn compute(s: &ClockSnapshot, now_ns: u64, offset_ns: i64) -> (Position, bool) {
    let mut pos = Position {
        track: s.track,
        frame: s.frame,
        sample_rate: s.sample_rate,
        state: s.state,
        discontinuity: false,
    };
    if s.state != PlayState::Playing || s.starved || s.sample_rate == 0 {
        return (pos, false);
    }
    let rate = s.sample_rate as f64;
    let audible_at = s.host_ns as f64 + s.latency_ns as f64;
    let elapsed_s = (now_ns as f64 - audible_at + offset_ns as f64) / 1e9;
    // Never extrapolate far past what has been handed to the device (a late callback).
    let max_ahead = s.buffer_frames as f64 + rate * 0.05;
    let off = (elapsed_s * rate).min(max_ahead).floor();
    match s.boundary {
        Some((track, at, frame)) if off >= at as f64 => {
            pos.track = track;
            pos.frame = frame + (off - at as f64) as u64;
            (pos, true)
        }
        _ => {
            pos.frame = (s.frame as f64 + off).max(0.0) as u64;
            (pos, false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    fn playing(frame: u64, host_ns: u64) -> ClockSnapshot {
        ClockSnapshot {
            track: 1,
            frame,
            host_ns,
            latency_ns: 10 * MS,
            sample_rate: 48_000,
            state: PlayState::Playing,
            boundary: None,
            starved: false,
            buffer_frames: 512,
            epoch: 1,
            boundary_epoch: None,
        }
    }

    #[test]
    fn interpolates_and_compensates_latency() {
        let (w, mut r) = clock();
        w.publish(&playing(48_000, 1_000 * MS));
        // Exactly when the buffer's first frame becomes audible.
        assert_eq!(r.position(1_010 * MS).frame, 48_000);
        // 5 ms later → 240 frames later.
        assert_eq!(r.position(1_015 * MS).frame, 48_240);
    }

    #[test]
    fn before_audible_reports_earlier_frames_and_stays_monotonic() {
        let (w, mut r) = clock();
        w.publish(&playing(48_000, 1_000 * MS));
        let early = r.position(1_000 * MS).frame;
        assert_eq!(early, 48_000 - 480, "10 ms of latency still to play");
        // Going back in time never moves a reader backwards.
        assert_eq!(r.position(990 * MS).frame, early);
    }

    #[test]
    fn extrapolation_is_capped() {
        let (w, mut r) = clock();
        w.publish(&playing(0, 0));
        let far = r.position(10_000 * MS).frame;
        assert_eq!(far, 512 + 2400);
    }

    #[test]
    fn gapless_boundary_switches_at_audible_frame() {
        let (w, mut r) = clock();
        let mut s = playing(10_000, 0);
        s.boundary = Some((2, 100, 0));
        w.publish(&s);
        let before = r.position(10 * MS + 99 * MS / 48);
        assert_eq!((before.track, before.frame), (1, 10_099));
        let after = r.position(10 * MS + 101 * MS / 48 + 1);
        assert_eq!((after.track, after.frame), (2, 1));
    }

    #[test]
    fn boundary_after_seek_carries_new_position() {
        let (w, mut r) = clock();
        let mut s = playing(10_000, 0);
        s.boundary = Some((1, 200, 480_000));
        s.epoch = 2;
        w.publish(&s);
        let p = r.position(10 * MS + 300 * MS / 48 + 1);
        assert_eq!((p.track, p.frame), (1, 480_100));
    }

    #[test]
    fn starved_buffer_freezes_position() {
        let (w, mut r) = clock();
        let mut s = playing(777, 0);
        s.starved = true;
        w.publish(&s);
        assert_eq!(r.position(500 * MS).frame, 777);
    }

    #[test]
    fn paused_is_frozen() {
        let (w, mut r) = clock();
        let mut s = playing(1234, 0);
        s.state = PlayState::Paused;
        w.publish(&s);
        assert_eq!(r.position(0).frame, 1234);
        assert_eq!(r.position(5_000 * MS).frame, 1234);
    }

    #[test]
    fn epoch_change_flags_discontinuity_and_allows_backwards() {
        let (w, mut r) = clock();
        w.publish(&playing(96_000, 0));
        assert!(!r.position(10 * MS).discontinuity);
        let mut s = playing(0, 20 * MS);
        s.epoch = 2;
        w.publish(&s);
        let p = r.position(30 * MS);
        assert!(p.discontinuity);
        assert_eq!(p.frame, 0);
        assert!(!r.position(31 * MS).discontinuity);
    }

    #[test]
    fn user_offset_shifts_positions() {
        let (w, r) = clock();
        w.publish(&playing(0, 0));
        let base = r.clone().position(20 * MS).frame;
        r.set_offset_ms(20);
        assert_eq!(r.clone().position(20 * MS).frame, base + 960);
        r.set_offset_ms(500);
        assert_eq!(r.offset_ms(), MAX_OFFSET_MS);
    }

    #[test]
    fn concurrent_reads_are_consistent() {
        let (w, r) = clock();
        let writer = std::thread::spawn(move || {
            for i in 0..200_000u64 {
                let mut s = playing(i, i);
                s.latency_ns = i;
                s.epoch = i;
                w.publish(&s);
            }
        });
        let reader = std::thread::spawn(move || {
            for _ in 0..200_000 {
                let s = r.snapshot();
                assert!(
                    s.frame == s.host_ns && s.host_ns == s.latency_ns && s.latency_ns == s.epoch
                );
            }
        });
        writer.join().unwrap();
        reader.join().unwrap();
    }
}
