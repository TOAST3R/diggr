//! Lossy, position-stamped copy of the post-EQ signal for visualizers and analyzers.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rtrb::{Consumer, Producer, RingBuffer};

use crate::TrackId;
use crate::ring::{CHANNELS, track_frames};

pub const TAP_CHUNK_FRAMES: usize = 256;

#[derive(Debug, Clone, Copy)]
pub struct TapChunk {
    pub track: TrackId,
    /// Track frame of `samples[0]`, comparable with the playback clock.
    pub start_frame: u64,
    pub frames: usize,
    /// Track frames per sample frame (varispeed; 1 at the track's own tempo).
    pub speed: f64,
    /// Interleaved stereo; only the first `frames * 2` values are valid.
    pub samples: [f32; TAP_CHUNK_FRAMES * CHANNELS],
}

impl TapChunk {
    const EMPTY: Self = Self {
        track: 0,
        start_frame: 0,
        frames: 0,
        speed: 1.0,
        samples: [0.0; TAP_CHUNK_FRAMES * CHANNELS],
    };

    /// The track frame just after this chunk.
    pub fn end_frame(&self) -> u64 {
        self.start_frame + track_frames(self.frames, self.speed)
    }

    pub fn data(&self) -> &[f32] {
        &self.samples[..self.frames * CHANNELS]
    }
}

pub fn tap(capacity_chunks: usize) -> (TapWriter, TapReader) {
    let (tx, rx) = RingBuffer::new(capacity_chunks);
    let dropped = Arc::new(AtomicU64::new(0));
    (
        TapWriter {
            tx,
            pending: TapChunk::EMPTY,
            dropped: dropped.clone(),
        },
        TapReader {
            rx,
            dropped,
            seen_dropped: 0,
        },
    )
}

/// Owned by the audio callback. Never blocks: full ring → chunk dropped and counted.
pub struct TapWriter {
    tx: Producer<TapChunk>,
    pending: TapChunk,
    dropped: Arc<AtomicU64>,
}

impl TapWriter {
    pub fn write(&mut self, track: TrackId, start_frame: u64, speed: f64, mut stereo: &[f32]) {
        let p = &self.pending;
        // (±1: track frames are rounded per run under varispeed.)
        if p.frames > 0
            && (p.track != track || p.speed != speed || p.end_frame().abs_diff(start_frame) > 1)
        {
            self.flush();
        }
        let mut written = 0;
        while !stereo.is_empty() {
            if self.pending.frames == 0 {
                self.pending.track = track;
                self.pending.start_frame = start_frame + track_frames(written, speed);
                self.pending.speed = speed;
            }
            let room = TAP_CHUNK_FRAMES - self.pending.frames;
            let n = room.min(stereo.len() / CHANNELS);
            let at = self.pending.frames * CHANNELS;
            self.pending.samples[at..at + n * CHANNELS].copy_from_slice(&stereo[..n * CHANNELS]);
            self.pending.frames += n;
            written += n;
            stereo = &stereo[n * CHANNELS..];
            if self.pending.frames == TAP_CHUNK_FRAMES {
                self.flush();
            }
        }
    }

    pub fn flush(&mut self) {
        if self.pending.frames == 0 {
            return;
        }
        if self.tx.push(self.pending).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        self.pending.frames = 0;
    }
}

pub struct TapReader {
    rx: Consumer<TapChunk>,
    dropped: Arc<AtomicU64>,
    seen_dropped: u64,
}

impl TapReader {
    pub fn pop(&mut self) -> Option<TapChunk> {
        self.rx.pop().ok()
    }

    /// Chunks dropped because this reader fell behind, since the previous call.
    pub fn take_dropped(&mut self) -> u64 {
        let total = self.dropped.load(Ordering::Relaxed);
        let new = total - self.seen_dropped;
        self.seen_dropped = total;
        new
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_are_stamped_and_split_on_discontinuity() {
        let (mut w, mut r) = tap(8);
        w.write(1, 1000, 1.0, &vec![0.5; 300 * CHANNELS]);
        w.write(1, 1300, 1.0, &[0.25; 10 * CHANNELS]);
        w.write(2, 0, 1.0, &[0.75; 5 * CHANNELS]); // track change
        w.flush();
        let a = r.pop().unwrap();
        assert_eq!((a.track, a.start_frame, a.frames), (1, 1000, 256));
        let b = r.pop().unwrap();
        assert_eq!((b.track, b.start_frame, b.frames), (1, 1256, 54));
        assert_eq!(b.data()[b.data().len() - 1], 0.25);
        let c = r.pop().unwrap();
        assert_eq!((c.track, c.start_frame, c.frames), (2, 0, 5));
        assert!(r.pop().is_none());
    }

    #[test]
    fn varispeed_chunks_are_stamped_in_track_frames() {
        let (mut w, mut r) = tap(8);
        w.write(1, 1000, 1.04, &vec![0.5; 300 * CHANNELS]);
        w.write(1, 1312, 1.04, &[0.25; 10 * CHANNELS]); // continues (1000 + 300 × 1.04)
        w.flush();
        let a = r.pop().unwrap();
        assert_eq!((a.start_frame, a.end_frame()), (1000, 1266));
        let b = r.pop().unwrap();
        assert_eq!((b.start_frame, b.frames, b.speed), (1266, 54, 1.04));
        assert!(r.pop().is_none());
    }

    #[test]
    fn full_ring_drops_and_counts() {
        let (mut w, mut r) = tap(2);
        for i in 0..10 {
            w.write(1, i * 256, 1.0, &vec![0.0; 256 * CHANNELS]);
        }
        assert_eq!(r.take_dropped(), 8);
        assert_eq!(r.take_dropped(), 0);
        assert_eq!(r.pop().unwrap().start_frame, 0);
    }
}
