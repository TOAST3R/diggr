//! Lock-free PCM transport from the decode thread to the audio callback.
//!
//! Samples (interleaved stereo f32) and segment headers travel in two SPSC rings. A header is
//! pushed only after its samples, so the consumer never sees a header without data. Headers carry
//! a generation: a seek/stop bumps the target generation and the callback discards older data.

use rtrb::{Consumer, Producer, RingBuffer};

use crate::TrackId;

/// The engine's internal channel count. Device channel mapping happens at the very end.
pub const CHANNELS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub generation: u32,
    pub track: TrackId,
    /// Position of the first frame in the track's timeline, in output-rate frames.
    pub start_frame: u64,
    pub frames: u32,
    /// A zero-length marker meaning "nothing follows": the queue has ended.
    pub end_of_queue: bool,
    /// Changes where the worker spliced the stream without a flush (a scheduled jump or a
    /// loop wrap), so the clock can report a discontinuity at exactly that point.
    pub splice: u16,
}

pub fn pcm_ring(capacity_frames: usize, max_segments: usize) -> (PcmProducer, PcmConsumer) {
    let (samples_tx, samples_rx) = RingBuffer::new(capacity_frames * CHANNELS);
    let (segs_tx, segs_rx) = RingBuffer::new(max_segments);
    (
        PcmProducer {
            samples: samples_tx,
            segments: segs_tx,
        },
        PcmConsumer {
            samples: samples_rx,
            segments: segs_rx,
            current: None,
            consumed: 0,
        },
    )
}

pub struct PcmProducer {
    samples: Producer<f32>,
    segments: Producer<Segment>,
}

impl PcmProducer {
    /// Frames that can be pushed right now.
    pub fn free_frames(&self) -> usize {
        if self.segments.slots() == 0 {
            0
        } else {
            self.samples.slots() / CHANNELS
        }
    }

    /// Pushes a whole segment. Returns `false` (pushing nothing) if it does not fit.
    pub fn push(
        &mut self,
        generation: u32,
        track: TrackId,
        start_frame: u64,
        samples: &[f32],
    ) -> bool {
        self.push_spliced(generation, 0, track, start_frame, samples)
    }

    /// [`push`](Self::push) with a splice number (see [`Segment::splice`]).
    pub fn push_spliced(
        &mut self,
        generation: u32,
        splice: u16,
        track: TrackId,
        start_frame: u64,
        samples: &[f32],
    ) -> bool {
        debug_assert_eq!(samples.len() % CHANNELS, 0);
        let frames = samples.len() / CHANNELS;
        if frames == 0 {
            return true;
        }
        if frames > self.free_frames() {
            return false;
        }
        self.samples
            .push_entire_slice(samples)
            .expect("space checked");
        self.segments
            .push(Segment {
                generation,
                track,
                start_frame,
                frames: frames as u32,
                end_of_queue: false,
                splice,
            })
            .expect("space checked");
        true
    }

    pub fn push_end_of_queue(&mut self, generation: u32, track: TrackId, at_frame: u64) -> bool {
        self.segments
            .push(Segment {
                generation,
                track,
                start_frame: at_frame,
                frames: 0,
                end_of_queue: true,
                splice: 0,
            })
            .is_ok()
    }
}

/// One contiguous run of samples from a single segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub generation: u32,
    pub splice: u16,
    pub track: TrackId,
    pub start_frame: u64,
    pub frames: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadResult {
    /// `frames` were copied into the start of the destination.
    Audio(Run),
    EndOfQueue {
        track: TrackId,
        at_frame: u64,
    },
    /// Nothing available (underrun).
    Empty,
}

pub struct PcmConsumer {
    samples: Consumer<f32>,
    segments: Consumer<Segment>,
    current: Option<Segment>,
    /// Frames of `current` already consumed.
    consumed: u32,
}

impl PcmConsumer {
    /// Reads up to `dst.len() / CHANNELS` frames from a single segment, skipping any data whose
    /// generation is older than `min_generation`.
    pub fn read(&mut self, min_generation: u32, dst: &mut [f32]) -> ReadResult {
        loop {
            let seg = match self.current {
                Some(seg) => seg,
                None => match self.segments.pop() {
                    Ok(seg) => {
                        self.current = Some(seg);
                        self.consumed = 0;
                        seg
                    }
                    Err(_) => return ReadResult::Empty,
                },
            };
            let remaining = (seg.frames - self.consumed) as usize;
            if seg.generation < min_generation {
                self.skip(remaining * CHANNELS);
                self.current = None;
                continue;
            }
            if seg.end_of_queue {
                self.current = None;
                return ReadResult::EndOfQueue {
                    track: seg.track,
                    at_frame: seg.start_frame,
                };
            }
            let frames = remaining.min(dst.len() / CHANNELS);
            if frames == 0 {
                return ReadResult::Empty;
            }
            let chunk = self
                .samples
                .read_chunk(frames * CHANNELS)
                .expect("samples precede header");
            let (a, b) = chunk.as_slices();
            dst[..a.len()].copy_from_slice(a);
            dst[a.len()..a.len() + b.len()].copy_from_slice(b);
            chunk.commit_all();
            let run = Run {
                generation: seg.generation,
                splice: seg.splice,
                track: seg.track,
                start_frame: seg.start_frame + self.consumed as u64,
                frames,
            };
            self.consumed += frames as u32;
            if self.consumed == seg.frames {
                self.current = None;
            }
            return ReadResult::Audio(run);
        }
    }

    /// Drops everything older than `min_generation` that is already queued.
    pub fn discard_stale(&mut self, min_generation: u32) {
        loop {
            let seg = match self.current {
                Some(seg) => seg,
                None => match self.segments.peek() {
                    Ok(seg) => *seg,
                    Err(_) => return,
                },
            };
            if seg.generation >= min_generation {
                return;
            }
            if self.current.is_none() {
                let _ = self.segments.pop();
                self.consumed = 0;
            }
            self.skip((seg.frames - self.consumed) as usize * CHANNELS);
            self.current = None;
        }
    }

    fn skip(&mut self, samples: usize) {
        if samples > 0 {
            self.samples
                .read_chunk(samples)
                .expect("samples precede header")
                .commit_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(n: usize, value: f32) -> Vec<f32> {
        vec![value; n * CHANNELS]
    }

    #[test]
    fn reads_runs_in_order() {
        let (mut tx, mut rx) = pcm_ring(1024, 16);
        assert!(tx.push(1, 7, 0, &frames(100, 0.5)));
        assert!(tx.push(1, 7, 100, &frames(50, 0.25)));
        let mut dst = vec![0.0; 64 * CHANNELS];
        assert_eq!(
            rx.read(1, &mut dst),
            ReadResult::Audio(Run {
                splice: 0,
                generation: 1,
                track: 7,
                start_frame: 0,
                frames: 64
            })
        );
        assert!(dst.iter().all(|&s| s == 0.5));
        assert_eq!(
            rx.read(1, &mut dst),
            ReadResult::Audio(Run {
                splice: 0,
                generation: 1,
                track: 7,
                start_frame: 64,
                frames: 36
            })
        );
        assert_eq!(
            rx.read(1, &mut dst),
            ReadResult::Audio(Run {
                splice: 0,
                generation: 1,
                track: 7,
                start_frame: 100,
                frames: 50
            })
        );
        assert_eq!(dst[0], 0.25);
        assert_eq!(rx.read(1, &mut dst), ReadResult::Empty);
    }

    #[test]
    fn stale_generations_are_skipped() {
        let (mut tx, mut rx) = pcm_ring(1024, 16);
        tx.push(1, 1, 0, &frames(100, 1.0));
        tx.push(2, 1, 5000, &frames(10, 2.0));
        let mut dst = vec![0.0; 32 * CHANNELS];
        // Partially consume generation 1, then a seek makes it stale.
        rx.read(1, &mut dst);
        match rx.read(2, &mut dst) {
            ReadResult::Audio(run) => {
                assert_eq!((run.generation, run.start_frame, run.frames), (2, 5000, 10));
                assert_eq!(dst[0], 2.0);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn discard_stale_frees_space() {
        let (mut tx, mut rx) = pcm_ring(100, 16);
        assert!(tx.push(1, 1, 0, &frames(100, 1.0)));
        assert_eq!(tx.free_frames(), 0);
        rx.discard_stale(2);
        assert_eq!(tx.free_frames(), 100);
        assert!(
            !tx.push(2, 1, 0, &frames(101, 1.0)),
            "oversized push must be refused whole"
        );
    }

    #[test]
    fn end_of_queue_marker() {
        let (mut tx, mut rx) = pcm_ring(64, 4);
        tx.push(1, 3, 0, &frames(4, 1.0));
        tx.push_end_of_queue(1, 3, 4);
        let mut dst = vec![0.0; 16 * CHANNELS];
        assert!(matches!(rx.read(1, &mut dst), ReadResult::Audio(_)));
        assert_eq!(
            rx.read(1, &mut dst),
            ReadResult::EndOfQueue {
                track: 3,
                at_frame: 4
            }
        );
    }
}
