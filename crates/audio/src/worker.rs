//! The decode thread: keeps the PCM ring full, handles transport commands, pre-warms the next
//! track and continues into it gaplessly.
//!
//! Decoded audio flows through one continuous [`Stream`] that resamples to the output rate. When
//! consecutive tracks share a source rate the same resampler carries across the boundary, so
//! gapless transitions stay seamless even when resampling. Output frames are attributed back to
//! tracks exactly (a track that starts at input frame `i` starts at output frame
//! `round(i * ratio)`), which keeps the playback clock precise at boundaries.
//!
//! [`DecodeWorker::step`] is a non-blocking state machine so a web worker can drive it; the
//! native thread uses [`DecodeWorker::run`].

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use platform::{FileSource, TrackRef};

use crate::decode::{DecodeError, StereoResampler, TrackDecoder};
use crate::ring::{CHANNELS, PcmProducer};
use crate::{TrackId, TrackInfo};

/// Largest segment pushed at once (keeps fast starts fast and headers frequent).
const MAX_SEGMENT_FRAMES: usize = 2048;
/// How much of the next track is decoded ahead of time.
const PREWARM_BUFFER_SECS: f64 = 1.0;
const RECENT_IDS: usize = 64;
/// Crossfade at a splice (scheduled jump or loop wrap): 2 ms at 48 kHz.
const SPLICE_XFADE_FRAMES: usize = 96;

/// What plays after the last track (or after every track, for `One`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    Off,
    /// After the last queue entry, continue gaplessly with the first.
    All,
    /// Loop the current track gaplessly.
    One,
}

impl RepeatMode {
    /// The queue index that follows `index` in a queue of `len` entries.
    pub fn next_index(self, index: usize, len: usize) -> Option<usize> {
        match self {
            _ if index >= len => None,
            RepeatMode::One => Some(index),
            RepeatMode::All => Some((index + 1) % len),
            RepeatMode::Off => (index + 1 < len).then_some(index + 1),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    SetRepeat(RepeatMode),
    /// Replace the queue; `current` is the new index of the playing track, if it is still there.
    SetQueue {
        tracks: Vec<TrackRef>,
        current: Option<usize>,
    },
    Load {
        generation: u32,
        index: usize,
        start_secs: f64,
    },
    Seek {
        generation: u32,
        secs: f64,
    },
    /// Jump within `track` to `target_secs`, splicing exactly at the first of `at_secs` (track
    /// time) that has not been sent to the device yet.
    SeekAt {
        track: TrackId,
        at_secs: Vec<f64>,
        target_secs: f64,
    },
    /// Loop `[start, end)` seconds of `track` gaplessly, or stop looping.
    SetLoop {
        track: TrackId,
        region: Option<(f64, f64)>,
    },
    Stop,
    /// Output format changed: restart `track` at `secs` resampled to `out_rate`.
    Reload {
        generation: u32,
        out_rate: u32,
        track: TrackId,
        secs: f64,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EngineEvent {
    /// A track instance started decoding; the clock reports `id` once it is audible.
    TrackLoaded {
        id: TrackId,
        index: usize,
        track: TrackRef,
    },
    /// Full metadata, sent after the track's first audio has been queued.
    TrackInfo {
        id: TrackId,
        index: usize,
        info: TrackInfo,
    },
    /// The next track has been opened and its first second decoded.
    PreWarm { index: usize, track: TrackRef },
    TrackFailed {
        index: usize,
        track: TrackRef,
        error: String,
    },
    /// A [`Command::SeekAt`] will splice at `at_secs` of the track.
    JumpScheduled { track: TrackId, at_secs: f64 },
    /// A [`Command::SeekAt`] came too late for all its points (or the track changed).
    JumpMissed { track: TrackId },
    /// The loop region now in effect (`None`: not looping).
    LoopChanged {
        track: TrackId,
        region: Option<(f64, f64)>,
    },
    /// A [`Command::SetLoop`] whose end has already been sent to the device.
    LoopRejected { track: TrackId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Made progress; call again.
    Worked,
    /// The ring is full.
    Full,
    /// Nothing to do until a command arrives.
    Idle,
    Shutdown,
}

#[derive(Debug, Clone, Copy)]
struct StreamTrack {
    /// First output frame (stream timeline) belonging to this track.
    out_start: u64,
    id: TrackId,
    /// Track-timeline frame at `out_start`.
    start_frame: u64,
    /// Ring splice number for this stretch (see [`crate::ring::Segment::splice`]).
    splice: u16,
}

/// Source-rate stereo in, output-rate stereo out, with exact per-track attribution.
struct Stream {
    resampler: Option<StereoResampler>,
    src_rate: u32,
    out_rate: u32,
    ratio: f64,
    /// Output frame index at which this resampler's output begins.
    out_base: u64,
    in_total: u64,
    out_total: u64,
    tracks: VecDeque<StreamTrack>,
}

impl Stream {
    fn new(
        src_rate: u32,
        out_rate: u32,
        out_base: u64,
        tracks: VecDeque<StreamTrack>,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            resampler: (src_rate != out_rate)
                .then(|| StereoResampler::new(src_rate, out_rate))
                .transpose()?,
            src_rate,
            out_rate,
            ratio: out_rate as f64 / src_rate as f64,
            out_base,
            in_total: 0,
            out_total: out_base,
            tracks,
        })
    }

    /// Registers a track whose first input frame is the next one fed.
    fn begin_track(&mut self, id: TrackId, start_frame: u64, splice: u16) -> u64 {
        let out_start = self.out_base + (self.in_total as f64 * self.ratio).round() as u64;
        self.tracks.push_back(StreamTrack {
            out_start,
            id,
            start_frame,
            splice,
        });
        out_start
    }

    /// Continues the timeline at output frame `at` with fresh input (a new resampler), after
    /// the caller dropped any output past `at`.
    fn restart_at(
        &mut self,
        at: u64,
        id: TrackId,
        start_frame: u64,
        splice: u16,
    ) -> Result<(), DecodeError> {
        if let Some(r) = &mut self.resampler {
            *r = StereoResampler::new(self.src_rate, self.out_rate)?;
        }
        self.out_base = at;
        self.in_total = 0;
        self.out_total = at;
        while self.tracks.len() > 1 && self.tracks.back().is_some_and(|t| t.out_start >= at) {
            self.tracks.pop_back();
        }
        self.begin_track(id, start_frame, splice);
        Ok(())
    }

    /// The output frame at which `track_frame` of `id`'s latest stretch plays, if ahead of it.
    fn out_frame_of(&self, id: TrackId, track_frame: u64) -> Option<u64> {
        let t = self.tracks.iter().rev().find(|t| t.id == id)?;
        (track_frame >= t.start_frame).then(|| t.out_start + (track_frame - t.start_frame))
    }

    fn feed(&mut self, stereo: &[f32], out: &mut Vec<f32>) {
        let before = out.len();
        self.in_total += (stereo.len() / CHANNELS) as u64;
        match &mut self.resampler {
            Some(r) => r.process(stereo, out),
            None => out.extend_from_slice(stereo),
        }
        self.out_total += ((out.len() - before) / CHANNELS) as u64;
    }

    /// Flushes the resampler tail. Afterwards output is exactly `round(in_total * ratio)`.
    fn finish(&mut self, out: &mut Vec<f32>) {
        if let Some(r) = &mut self.resampler {
            let before = out.len();
            r.finish(out);
            self.out_total += ((out.len() - before) / CHANNELS) as u64;
        }
    }

    /// The track owning output frame `o`, its track frame, frames until the next stretch, and
    /// its splice number.
    fn locate(&self, o: u64) -> (TrackId, u64, Option<u64>, u16) {
        let i = self
            .tracks
            .iter()
            .rposition(|t| t.out_start <= o)
            .unwrap_or(0);
        let t = self.tracks[i];
        let limit = self.tracks.get(i + 1).map(|n| n.out_start - o);
        (t.id, t.start_frame + (o - t.out_start), limit, t.splice)
    }

    /// Forgets tracks that end before output frame `o`.
    fn prune(&mut self, o: u64) {
        while self.tracks.len() > 1 && self.tracks[1].out_start <= o {
            self.tracks.pop_front();
        }
    }
}

struct Active {
    dec: TrackDecoder,
    id: TrackId,
    index: usize,
    out_start: u64,
    start_secs: f64,
    /// Source frames decoded since `start_secs`.
    in_frames: u64,
    finished: bool,
    info_sent: bool,
}

struct Prewarmed {
    dec: TrackDecoder,
    index: usize,
    /// Source-rate stereo.
    buffered: Vec<f32>,
    finished: bool,
}

pub struct WorkerConfig {
    pub out_rate: u32,
    /// Pre-warm the next track when this many seconds of the current one remain.
    pub prewarm_secs: f64,
}

pub struct DecodeWorker {
    files: Arc<dyn FileSource>,
    commands: Receiver<Command>,
    events: Sender<EngineEvent>,
    pcm: PcmProducer,
    out_rate: u32,
    prewarm_secs: f64,
    queue: Vec<TrackRef>,
    generation: u32,
    current: Option<Active>,
    stream: Option<Stream>,
    prewarm: Option<Prewarmed>,
    /// Source-rate scratch for the decoder.
    raw: Vec<f32>,
    /// Stream output not yet pushed; `pending[0]` is output frame `pending_start`.
    pending: Vec<f32>,
    pending_start: u64,
    pending_pushed: usize,
    /// After the last track: an end-of-queue marker is due once `pending` is pushed.
    end_due: bool,
    next_id: TrackId,
    recent: VecDeque<(TrackId, usize)>,
    shutdown: bool,
    repeat: RepeatMode,
    /// Bumped at every splice so the renderer can mark it.
    splice_no: u16,
    /// A scheduled jump: output frame to splice at, and the target in track seconds.
    jump: Option<(u64, f64)>,
    /// Looping `[start_secs, end_secs)` of this track.
    looping: Option<(TrackId, f64, f64)>,
}

impl DecodeWorker {
    pub fn new(
        cfg: WorkerConfig,
        files: Arc<dyn FileSource>,
        commands: Receiver<Command>,
        events: Sender<EngineEvent>,
        pcm: PcmProducer,
    ) -> Self {
        Self {
            files,
            commands,
            events,
            pcm,
            out_rate: cfg.out_rate,
            prewarm_secs: cfg.prewarm_secs,
            queue: Vec::new(),
            generation: 0,
            current: None,
            stream: None,
            prewarm: None,
            raw: Vec::new(),
            pending: Vec::new(),
            pending_start: 0,
            pending_pushed: 0,
            end_due: false,
            next_id: 1,
            recent: VecDeque::new(),
            shutdown: false,
            repeat: RepeatMode::Off,
            splice_no: 0,
            jump: None,
            looping: None,
        }
    }

    /// Blocking loop for a native thread.
    pub fn run(mut self) {
        loop {
            let wait = match self.step() {
                Step::Worked => continue,
                Step::Full => Duration::from_millis(3),
                Step::Idle => Duration::from_millis(100),
                Step::Shutdown => return,
            };
            match self.commands.recv_timeout(wait) {
                Ok(cmd) => self.handle(cmd),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    /// Does a bounded amount of work without blocking.
    pub fn step(&mut self) -> Step {
        while let Ok(cmd) = self.commands.try_recv() {
            self.handle(cmd);
        }
        if self.shutdown {
            return Step::Shutdown;
        }
        if self.pending_pushed * CHANNELS < self.pending.len() {
            return if self.push_pending() {
                Step::Worked
            } else {
                Step::Full
            };
        }
        self.pending_start += self.pending_pushed as u64;
        self.pending.clear();
        self.pending_pushed = 0;

        if self.end_due {
            let Some(stream) = &self.stream else {
                self.end_due = false;
                return Step::Idle;
            };
            let (track, frame, _, _) = stream.locate(stream.out_total);
            if !self.pcm.push_end_of_queue(self.generation, track, frame) {
                return Step::Full;
            }
            self.end_due = false;
            self.stream = None;
            return Step::Worked;
        }

        let Some(cur) = self.current.as_mut() else {
            return Step::Idle;
        };
        if cur.finished {
            if !self.try_splice() {
                self.advance();
            }
            return Step::Worked;
        }
        self.raw.clear();
        match cur.dec.decode_next(&mut self.raw) {
            Ok(more) => {
                cur.finished = !more;
                cur.in_frames += (self.raw.len() / CHANNELS) as u64;
                let stream = self.stream.as_mut().expect("a current track has a stream");
                stream.feed(&self.raw, &mut self.pending);
                self.try_splice();
            }
            Err(e) => {
                let index = cur.index;
                cur.finished = true;
                self.fail(index, e.to_string());
            }
        }
        self.maybe_prewarm();
        Step::Worked
    }

    fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::SetRepeat(mode) => {
                self.repeat = mode;
                self.prewarm = None;
            }
            Command::SetQueue { tracks, current } => {
                self.queue = tracks;
                self.prewarm = None;
                if let Some(cur) = self.current.as_mut() {
                    // A removed track finishes, then the queue ends.
                    cur.index = current.unwrap_or(usize::MAX);
                }
            }
            Command::Load {
                generation,
                index,
                start_secs,
            } => {
                self.generation = generation;
                self.clear_splices();
                self.start(index, start_secs);
            }
            Command::Seek { generation, secs } => {
                self.generation = generation;
                self.clear_splices();
                let Some(cur) = self.current.as_mut() else {
                    return;
                };
                if cur.dec.seek(secs).is_ok() {
                    let (id, rate) = (cur.id, cur.dec.src_rate());
                    cur.start_secs = secs;
                    cur.in_frames = 0;
                    cur.finished = false;
                    self.reset_output();
                    match Stream::new(rate, self.out_rate, 0, VecDeque::new()) {
                        Ok(mut s) => {
                            let out_start = s.begin_track(id, self.frame_at(secs), self.splice_no);
                            self.current.as_mut().expect("checked").out_start = out_start;
                            self.stream = Some(s);
                        }
                        Err(e) => {
                            let index = cur_index(&self.current);
                            self.current = None;
                            self.fail(index, e.to_string());
                        }
                    }
                } else {
                    let index = cur.index;
                    self.start(index, secs);
                }
            }
            Command::Stop => {
                self.clear_splices();
                self.reset_output();
                self.current = None;
                self.prewarm = None;
            }
            Command::Reload {
                generation,
                out_rate,
                track,
                secs,
            } => {
                self.generation = generation;
                self.out_rate = out_rate;
                self.prewarm = None;
                self.clear_splices();
                let index = self
                    .recent
                    .iter()
                    .rev()
                    .find(|(id, _)| *id == track)
                    .map(|&(_, i)| i)
                    .or(self.current.as_ref().map(|c| c.index));
                match index {
                    Some(index) => self.start(index, secs),
                    None => self.reset_output(),
                }
            }
            Command::SeekAt {
                track,
                at_secs,
                target_secs,
            } => {
                let pick = self
                    .current
                    .as_ref()
                    .filter(|c| c.id == track)
                    .and(self.stream.as_ref())
                    .and_then(|stream| {
                        let frontier = self.frontier();
                        at_secs.iter().find_map(|&secs| {
                            stream
                                .out_frame_of(track, self.frame_at(secs))
                                .filter(|&o| o >= frontier)
                                .map(|o| (o, secs))
                        })
                    });
                match pick {
                    Some((o, secs)) => {
                        self.jump = Some((o, target_secs.max(0.0)));
                        let _ = self.events.send(EngineEvent::JumpScheduled {
                            track,
                            at_secs: secs,
                        });
                    }
                    None => {
                        let _ = self.events.send(EngineEvent::JumpMissed { track });
                    }
                }
            }
            Command::SetLoop { track, region } => {
                let valid = region.filter(|(a, b)| b > a && *a >= 0.0);
                let current = self.current.as_ref().is_some_and(|c| c.id == track);
                match valid {
                    None => {
                        self.looping = None;
                        let _ = self.events.send(EngineEvent::LoopChanged {
                            track,
                            region: None,
                        });
                    }
                    Some((a, b))
                        if current
                            && self
                                .loop_end_frame(track, b)
                                .is_some_and(|o| o >= self.frontier()) =>
                    {
                        self.looping = Some((track, a, b));
                        let _ = self.events.send(EngineEvent::LoopChanged {
                            track,
                            region: Some((a, b)),
                        });
                    }
                    Some(_) => {
                        let _ = self.events.send(EngineEvent::LoopRejected { track });
                    }
                }
            }
            Command::Shutdown => self.shutdown = true,
        }
    }

    /// The first output frame not yet sent to the device.
    fn frontier(&self) -> u64 {
        self.pending_start + self.pending_pushed as u64
    }

    fn loop_end_frame(&self, track: TrackId, end_secs: f64) -> Option<u64> {
        self.stream
            .as_ref()?
            .out_frame_of(track, self.frame_at(end_secs))
    }

    fn clear_splices(&mut self) {
        self.jump = None;
        if let Some((track, ..)) = self.looping.take() {
            let _ = self.events.send(EngineEvent::LoopChanged {
                track,
                region: None,
            });
        }
    }

    /// Performs a due scheduled jump or loop wrap once the audio up to (and a little past) its
    /// splice point has been decoded. Returns whether it spliced.
    fn try_splice(&mut self) -> bool {
        let Some((id, finished)) = self.current.as_ref().map(|c| (c.id, c.finished)) else {
            return false;
        };
        let loop_at = match self.looping {
            Some((t, a, b)) if t == id => self.loop_end_frame(t, b).map(|o| (o, a)),
            Some(_) => {
                self.clear_splices();
                None
            }
            None => None,
        };
        let due = match (self.jump, loop_at) {
            (Some(j), Some(l)) => Some(if j.0 <= l.0 { j } else { l }),
            (j, l) => j.or(l),
        };
        let Some((at, target)) = due else {
            return false;
        };
        let Some(stream) = self.stream.as_ref() else {
            return false;
        };
        if stream.out_total < at + SPLICE_XFADE_FRAMES as u64 && !finished {
            return false; // keep decoding up to the splice point
        }
        let is_jump = self.jump.is_some_and(|j| j.0 == at);
        if at < self.frontier() || at > stream.out_total {
            // Already sent, or the track ended first.
            if is_jump {
                self.jump = None;
                let _ = self.events.send(EngineEvent::JumpMissed { track: id });
            } else {
                self.looping = None;
                let _ = self.events.send(EngineEvent::LoopRejected { track: id });
            }
            return false;
        }
        if is_jump {
            self.jump = None;
        }
        self.splice(at, target)
    }

    /// Replaces everything from output frame `at` on with the track from `target_secs`,
    /// crossfading the first frames with the audio it replaces.
    fn splice(&mut self, at: u64, target_secs: f64) -> bool {
        let cur = self.current.as_mut().expect("checked by caller");
        if cur.dec.seek(target_secs).is_err() {
            let _ = self.events.send(EngineEvent::JumpMissed { track: cur.id });
            return false;
        }
        let i = (at - self.pending_start) as usize * CHANNELS;
        let tail: Vec<f32> = self.pending[i..]
            .iter()
            .take(SPLICE_XFADE_FRAMES * CHANNELS)
            .copied()
            .collect();
        self.pending.truncate(i);
        self.splice_no = self.splice_no.wrapping_add(1);
        let start_frame = (target_secs * self.out_rate as f64).round() as u64;
        let stream = self.stream.as_mut().expect("a current track has a stream");
        if stream
            .restart_at(at, cur.id, start_frame, self.splice_no)
            .is_err()
        {
            return false;
        }
        cur.start_secs = target_secs;
        cur.in_frames = 0;
        cur.finished = false;
        // Decode enough of the new audio to crossfade over.
        while stream.out_total < at + SPLICE_XFADE_FRAMES as u64 && !cur.finished {
            self.raw.clear();
            match cur.dec.decode_next(&mut self.raw) {
                Ok(more) => {
                    cur.finished = !more;
                    cur.in_frames += (self.raw.len() / CHANNELS) as u64;
                    stream.feed(&self.raw, &mut self.pending);
                }
                Err(_) => cur.finished = true,
            }
        }
        let n = (tail.len() / CHANNELS).min((self.pending.len() - i) / CHANNELS);
        for j in 0..SPLICE_XFADE_FRAMES.min((self.pending.len() - i) / CHANNELS) {
            let w = (j as f32 + 0.5) / SPLICE_XFADE_FRAMES as f32 * std::f32::consts::FRAC_PI_2;
            let (g_new, g_old) = (w.sin(), w.cos());
            for c in 0..CHANNELS {
                let k = i + j * CHANNELS + c;
                let old = if j < n { tail[j * CHANNELS + c] } else { 0.0 };
                self.pending[k] = self.pending[k] * g_new + old * g_old;
            }
        }
        true
    }

    fn frame_at(&self, secs: f64) -> u64 {
        (secs * self.out_rate as f64).round() as u64
    }

    fn reset_output(&mut self) {
        self.stream = None;
        self.pending.clear();
        self.pending_start = 0;
        self.pending_pushed = 0;
        self.end_due = false;
    }

    fn new_id(&mut self, index: usize) -> TrackId {
        let id = self.next_id;
        self.next_id += 1;
        self.recent.push_back((id, index));
        if self.recent.len() > RECENT_IDS {
            self.recent.pop_front();
        }
        id
    }

    /// Opens the first playable track at or after `index`, taking the pre-warmed one if it
    /// matches. Returns the decoder, its index, and any already decoded source audio.
    fn open_from(
        &mut self,
        mut index: usize,
        secs: f64,
    ) -> Option<(TrackDecoder, usize, Vec<f32>, bool)> {
        while index < self.queue.len() {
            if secs == 0.0
                && let Some(p) = self.prewarm.take_if(|p| p.index == index)
            {
                return Some((p.dec, index, p.buffered, p.finished));
            }
            let opened =
                TrackDecoder::open(&*self.files, &self.queue[index]).and_then(|mut dec| {
                    if secs > 0.0 {
                        dec.seek(secs)?;
                    }
                    Ok(dec)
                });
            match opened {
                Ok(dec) => return Some((dec, index, Vec::new(), false)),
                Err(e) => {
                    self.fail(index, e.to_string());
                    index += 1;
                }
            }
        }
        None
    }

    /// Starts `index` (or the next playable track after it) at `secs` on a fresh stream.
    fn start(&mut self, index: usize, secs: f64) {
        self.reset_output();
        self.current = None;
        let Some((dec, index, buffered, finished)) = self.open_from(index, secs) else {
            return;
        };
        let mut stream = match Stream::new(dec.src_rate(), self.out_rate, 0, VecDeque::new()) {
            Ok(s) => s,
            Err(e) => {
                self.fail(index, e.to_string());
                return;
            }
        };
        let id = self.new_id(index);
        let out_start = stream.begin_track(id, self.frame_at(secs), self.splice_no);
        self.activate(
            dec,
            id,
            index,
            out_start,
            secs,
            buffered,
            finished,
            &mut stream,
        );
        self.stream = Some(stream);
    }

    #[allow(clippy::too_many_arguments)]
    fn activate(
        &mut self,
        dec: TrackDecoder,
        id: TrackId,
        index: usize,
        out_start: u64,
        start_secs: f64,
        buffered: Vec<f32>,
        finished: bool,
        stream: &mut Stream,
    ) {
        let track = self.queue[index].clone();
        let _ = self
            .events
            .send(EngineEvent::TrackLoaded { id, index, track });
        stream.feed(&buffered, &mut self.pending);
        self.current = Some(Active {
            dec,
            id,
            index,
            out_start,
            start_secs,
            in_frames: (buffered.len() / CHANNELS) as u64,
            finished,
            info_sent: false,
        });
    }

    /// The current track reached its end: continue gaplessly, or end the queue.
    fn advance(&mut self) {
        self.clear_splices();
        let Some(cur) = self.current.take() else {
            return;
        };
        let mut stream = self.stream.take().expect("a current track has a stream");
        let next = self
            .repeat
            .next_index(cur.index, self.queue.len())
            .and_then(|i| self.open_from(i, 0.0));
        let Some((dec, index, buffered, finished)) = next else {
            stream.finish(&mut self.pending);
            self.stream = Some(stream);
            self.end_due = true;
            return;
        };
        if dec.src_rate() != stream.src_rate {
            // Different source rate: flush this resampler and start another, continuing the
            // output timeline.
            stream.finish(&mut self.pending);
            let tracks = std::mem::take(&mut stream.tracks);
            match Stream::new(dec.src_rate(), self.out_rate, stream.out_total, tracks) {
                Ok(s) => stream = s,
                Err(e) => {
                    self.fail(index, e.to_string());
                    self.stream = Some(stream);
                    self.end_due = true;
                    return;
                }
            }
        }
        let id = self.new_id(index);
        let out_start = stream.begin_track(id, 0, self.splice_no);
        self.activate(
            dec,
            id,
            index,
            out_start,
            0.0,
            buffered,
            finished,
            &mut stream,
        );
        self.stream = Some(stream);
    }

    fn fail(&mut self, index: usize, error: String) {
        let track = self
            .queue
            .get(index)
            .cloned()
            .unwrap_or_else(|| TrackRef::new(""));
        let _ = self.events.send(EngineEvent::TrackFailed {
            index,
            track,
            error,
        });
    }

    /// Pushes as much of `pending` as fits. Returns false if nothing could be pushed.
    fn push_pending(&mut self) -> bool {
        let Some(stream) = self.stream.as_mut() else {
            self.pending.clear();
            self.pending_pushed = 0;
            return true;
        };
        let total = self.pending.len() / CHANNELS;
        let mut pushed_any = false;
        while self.pending_pushed < total {
            let o = self.pending_start + self.pending_pushed as u64;
            let (id, frame, limit, splice) = stream.locate(o);
            let mut n = (total - self.pending_pushed)
                .min(MAX_SEGMENT_FRAMES)
                .min(self.pcm.free_frames());
            if let Some(limit) = limit {
                n = n.min(limit as usize);
            }
            if n == 0 {
                break;
            }
            let a = self.pending_pushed * CHANNELS;
            let ok = self.pcm.push_spliced(
                self.generation,
                splice,
                id,
                frame,
                &self.pending[a..a + n * CHANNELS],
            );
            debug_assert!(ok);
            self.pending_pushed += n;
            pushed_any = true;
        }
        let pushed_to = self.pending_start + self.pending_pushed as u64;
        stream.prune(pushed_to);
        if let Some(cur) = self.current.as_mut()
            && !cur.info_sent
            && pushed_to > cur.out_start
        {
            cur.info_sent = true;
            let track = self
                .queue
                .get(cur.index)
                .cloned()
                .unwrap_or_else(|| TrackRef::new(""));
            let info = cur.dec.complete_info(&*self.files, &track).clone();
            let _ = self.events.send(EngineEvent::TrackInfo {
                id: cur.id,
                index: cur.index,
                info,
            });
        }
        pushed_any
    }

    fn maybe_prewarm(&mut self) {
        if self.prewarm.is_some() {
            return;
        }
        let Some(cur) = self.current.as_ref() else {
            return;
        };
        let Some(duration) = cur.dec.info().duration_secs else {
            return;
        };
        let position = cur.start_secs + cur.in_frames as f64 / cur.dec.src_rate() as f64;
        let Some(index) = self.repeat.next_index(cur.index, self.queue.len()) else {
            return;
        };
        if duration - position > self.prewarm_secs {
            return;
        }
        let track = self.queue[index].clone();
        let Ok(mut dec) = TrackDecoder::open(&*self.files, &track) else {
            // Reported when (if) playback actually reaches it.
            return;
        };
        let want = (PREWARM_BUFFER_SECS * dec.src_rate() as f64) as usize * CHANNELS;
        let mut buffered = Vec::with_capacity(want + 8192);
        let mut finished = false;
        while buffered.len() < want {
            match dec.decode_next(&mut buffered) {
                Ok(true) => {}
                Ok(false) => {
                    finished = true;
                    break;
                }
                Err(_) => return,
            }
        }
        let _ = self.events.send(EngineEvent::PreWarm { index, track });
        self.prewarm = Some(Prewarmed {
            dec,
            index,
            buffered,
            finished,
        });
    }
}

fn cur_index(current: &Option<Active>) -> usize {
    current.as_ref().map_or(usize::MAX, |c| c.index)
}

#[cfg(test)]
mod tests {
    use super::RepeatMode;

    #[test]
    fn next_index_per_repeat_mode() {
        assert_eq!(RepeatMode::Off.next_index(0, 3), Some(1));
        assert_eq!(RepeatMode::Off.next_index(2, 3), None);
        assert_eq!(RepeatMode::All.next_index(2, 3), Some(0));
        assert_eq!(RepeatMode::One.next_index(1, 3), Some(1));
        // A removed track (index past the end) never continues.
        for mode in [RepeatMode::Off, RepeatMode::All, RepeatMode::One] {
            assert_eq!(mode.next_index(usize::MAX, 3), None);
            assert_eq!(mode.next_index(0, 0), None);
        }
    }
}
