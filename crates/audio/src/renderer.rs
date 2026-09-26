//! The audio callback: ring → EQ → tap → volume/balance → device channels, plus clock updates.
//!
//! Everything here is real-time safe: no allocation, no locks, no waiting.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

use platform::{CallbackInfo, StreamFormat};
use rtrb::Consumer;

use crate::clock::{ClockSnapshot, ClockWriter};
use crate::eq::{EqCoefs, Equalizer};
use crate::ring::{CHANNELS, PcmConsumer, ReadResult};
use crate::tap::TapWriter;
use crate::{PlayState, TrackId, rt_guard};

/// Frames processed per internal block.
pub const BLOCK_FRAMES: usize = 1024;
const MAX_RUNS: usize = 16;

/// State shared between the control side (engine) and the callback.
pub struct Control {
    /// Data with an older generation is discarded (seek/stop/load).
    pub target_generation: AtomicU32,
    state: AtomicU8,
    volume_bits: AtomicU32,
    balance_bits: AtomicU32,
    underruns: AtomicU64,
    /// Latency measurement: when the last play/seek was requested and for which generation.
    start_request_ns: AtomicU64,
    start_generation: AtomicU32,
    last_start_latency_ns: AtomicU64,
    queue_ended: AtomicU64,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            target_generation: AtomicU32::new(0),
            state: AtomicU8::new(PlayState::Stopped as u8),
            volume_bits: AtomicU32::new(1f32.to_bits()),
            balance_bits: AtomicU32::new(0f32.to_bits()),
            underruns: AtomicU64::new(0),
            start_request_ns: AtomicU64::new(0),
            start_generation: AtomicU32::new(0),
            last_start_latency_ns: AtomicU64::new(0),
            queue_ended: AtomicU64::new(0),
        }
    }
}

impl Control {
    pub fn state(&self) -> PlayState {
        PlayState::from_u8(self.state.load(Ordering::Acquire))
    }

    pub fn set_state(&self, s: PlayState) {
        self.state.store(s as u8, Ordering::Release);
    }

    /// Makes all queued audio stale and returns the new generation.
    pub fn bump_generation(&self) -> u32 {
        self.target_generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn generation(&self) -> u32 {
        self.target_generation.load(Ordering::Acquire)
    }

    /// 0.0..=1.0
    pub fn set_volume(&self, v: f32) {
        self.volume_bits
            .store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume_bits.load(Ordering::Relaxed))
    }

    /// -1.0 (left) ..= 1.0 (right)
    pub fn set_balance(&self, b: f32) {
        self.balance_bits
            .store(b.clamp(-1.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn balance(&self) -> f32 {
        f32::from_bits(self.balance_bits.load(Ordering::Relaxed))
    }

    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    /// Start measuring the time until audio of `generation` reaches the device.
    pub fn mark_start_request(&self, generation: u32, now_ns: u64) {
        self.start_request_ns.store(now_ns, Ordering::Relaxed);
        self.start_generation.store(generation, Ordering::Release);
    }

    /// Time from the last play/seek request until its first audio was written to the device.
    pub fn last_start_latency_ns(&self) -> u64 {
        self.last_start_latency_ns.load(Ordering::Relaxed)
    }

    /// Increments each time playback runs off the end of the queue.
    pub fn queue_ended_count(&self) -> u64 {
        self.queue_ended.load(Ordering::Relaxed)
    }
}

/// Maps the internal 0..1 volume to a gain with a perceptual (squared) curve.
fn volume_gain(v: f32) -> f32 {
    v * v
}

fn balance_gains(b: f32) -> (f32, f32) {
    ((1.0 - b).min(1.0), (1.0 + b).min(1.0))
}

pub struct RendererParts {
    pub control: Arc<Control>,
    pub pcm: PcmConsumer,
    pub eq_rx: Consumer<EqCoefs>,
    pub eq: EqCoefs,
    pub tap: TapWriter,
    pub clock: ClockWriter,
    pub format: StreamFormat,
}

#[derive(Clone, Copy)]
struct RunNote {
    track: TrackId,
    start_frame: u64,
    offset: usize,
    frames: usize,
}

pub struct Renderer {
    control: Arc<Control>,
    pcm: PcmConsumer,
    eq: Equalizer,
    eq_rx: Consumer<EqCoefs>,
    tap: TapWriter,
    clock: ClockWriter,
    sample_rate: u32,
    device_channels: usize,
    scratch: Box<[f32]>,
    gains: (f32, f32),
    /// Track and frame that will be audible after everything written so far.
    pos: (TrackId, u64),
    played_generation: u32,
    device_epoch: u32,
    /// Whether audio of the current generation has started (for underrun accounting).
    started: bool,
}

impl Renderer {
    pub fn new(p: RendererParts) -> Self {
        let v = volume_gain(p.control.volume());
        let (l, r) = balance_gains(p.control.balance());
        Self {
            eq: Equalizer::new(p.eq, p.format.sample_rate),
            control: p.control,
            pcm: p.pcm,
            eq_rx: p.eq_rx,
            tap: p.tap,
            clock: p.clock,
            sample_rate: p.format.sample_rate,
            device_channels: p.format.channels.max(1) as usize,
            scratch: vec![0.0; BLOCK_FRAMES * CHANNELS].into_boxed_slice(),
            gains: (v * l, v * r),
            pos: (0, 0),
            played_generation: 0,
            device_epoch: 0,
            started: false,
        }
    }

    /// Adapts to a new device format. Called off the audio thread while the callback is idle.
    pub fn reconfigure(&mut self, format: StreamFormat, eq: EqCoefs) {
        self.sample_rate = format.sample_rate;
        self.device_channels = format.channels.max(1) as usize;
        self.eq.reconfigure(eq, format.sample_rate);
        self.device_epoch = self.device_epoch.wrapping_add(1);
        self.started = false;
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn epoch(&self) -> u64 {
        ((self.device_epoch as u64) << 32) | self.played_generation as u64
    }

    /// The real-time callback body. `out` is interleaved in the device's channel layout.
    pub fn render(&mut self, out: &mut [f32], info: CallbackInfo) {
        let _rt = rt_guard::enter();
        while let Ok(coefs) = self.eq_rx.pop() {
            self.eq.set(coefs);
        }
        let ch = self.device_channels;
        let total = out.len() / ch;
        let target = self.control.generation();
        let state = self.control.state();

        let mut snap = ClockSnapshot {
            track: self.pos.0,
            frame: self.pos.1,
            host_ns: info.host_ns,
            latency_ns: info.output_latency_ns,
            sample_rate: self.sample_rate,
            state,
            boundary: None,
            starved: false,
            buffer_frames: total as u32,
            epoch: self.epoch(),
        };

        if state != PlayState::Playing {
            if state == PlayState::Stopped {
                self.pcm.discard_stale(target);
            }
            out.fill(0.0);
            self.clock.publish(&snap);
            return;
        }

        let mut done = 0;
        let mut any_audio = false;
        let mut underrun = false;
        let mut ended = false;
        while done < total {
            let n = (total - done).min(BLOCK_FRAMES);
            let mut runs = [RunNote {
                track: 0,
                start_frame: 0,
                offset: 0,
                frames: 0,
            }; MAX_RUNS];
            let mut run_count = 0;
            let mut filled = 0;
            while filled < n && !ended {
                match self
                    .pcm
                    .read(target, &mut self.scratch[filled * CHANNELS..n * CHANNELS])
                {
                    ReadResult::Audio(run) => {
                        let at = done + filled;
                        if run.generation != self.played_generation {
                            self.played_generation = run.generation;
                            self.started = false;
                            snap.epoch = self.epoch();
                        }
                        if !self.started {
                            self.started = true;
                            self.note_start_latency(run.generation, info.host_ns);
                        }
                        // Clock: the buffer starts where this run starts, unless audio already
                        // played in this buffer and this run is not its continuation.
                        let continues = run.track == self.pos.0 && run.start_frame == self.pos.1;
                        if at == 0 {
                            snap.track = run.track;
                            snap.frame = run.start_frame;
                        } else if !continues && snap.boundary.is_none() {
                            snap.boundary = Some((run.track, at as u32, run.start_frame));
                        }
                        if run_count < MAX_RUNS {
                            runs[run_count] = RunNote {
                                track: run.track,
                                start_frame: run.start_frame,
                                offset: filled,
                                frames: run.frames,
                            };
                            run_count += 1;
                        }
                        self.pos = (run.track, run.start_frame + run.frames as u64);
                        filled += run.frames;
                        any_audio = true;
                    }
                    ReadResult::EndOfQueue { track, at_frame } => {
                        self.pos = (track, at_frame);
                        ended = true;
                    }
                    ReadResult::Empty => {
                        underrun = true;
                        break;
                    }
                }
            }
            self.scratch[filled * CHANNELS..n * CHANNELS].fill(0.0);
            self.eq.process(&mut self.scratch[..n * CHANNELS]);
            for r in &runs[..run_count] {
                let data = &self.scratch[r.offset * CHANNELS..(r.offset + r.frames) * CHANNELS];
                self.tap.write(r.track, r.start_frame, data);
            }
            self.write_device(&mut out[done * ch..(done + n) * ch], n);
            done += n;
            if underrun || ended {
                out[done * ch..].fill(0.0);
                break;
            }
        }
        self.tap.flush();

        // Only a gap in audio that had already started counts; waiting for the first audio of a
        // new play/seek request (a newer target generation) does not.
        if underrun && self.started && self.played_generation >= target && !ended {
            self.control.underruns.fetch_add(1, Ordering::Relaxed);
        }
        if ended {
            let _ = self.control.state.compare_exchange(
                PlayState::Playing as u8,
                PlayState::Stopped as u8,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
            self.control.queue_ended.fetch_add(1, Ordering::Relaxed);
            self.started = false;
        }
        snap.starved = !any_audio;
        self.clock.publish(&snap);
    }

    fn note_start_latency(&self, generation: u32, host_ns: u64) {
        let c = &self.control;
        if c.start_generation.load(Ordering::Acquire) == generation
            && c.start_generation
                .compare_exchange(generation, 0, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        {
            let requested = c.start_request_ns.load(Ordering::Relaxed);
            c.last_start_latency_ns
                .store(host_ns.saturating_sub(requested), Ordering::Relaxed);
        }
    }

    /// Volume/balance with a per-block linear ramp, then map stereo to the device layout.
    fn write_device(&mut self, out: &mut [f32], frames: usize) {
        let v = volume_gain(self.control.volume());
        let (bl, br) = balance_gains(self.control.balance());
        let target = (v * bl, v * br);
        let (mut gl, mut gr) = self.gains;
        let step = (
            (target.0 - gl) / frames as f32,
            (target.1 - gr) / frames as f32,
        );
        let ch = self.device_channels;
        for i in 0..frames {
            gl += step.0;
            gr += step.1;
            let l = self.scratch[i * CHANNELS] * gl;
            let r = self.scratch[i * CHANNELS + 1] * gr;
            let o = &mut out[i * ch..(i + 1) * ch];
            match ch {
                1 => o[0] = 0.5 * (l + r),
                _ => {
                    o[0] = l;
                    o[1] = r;
                    o[2..].fill(0.0);
                }
            }
        }
        self.gains = target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{ClockReader, clock};
    use crate::eq::EqSettings;
    use crate::ring::{PcmProducer, pcm_ring};
    use crate::tap::{TapReader, tap};

    const RATE: u32 = 48_000;

    struct Rig {
        r: Renderer,
        tx: PcmProducer,
        control: Arc<Control>,
        clock: ClockReader,
        tap: TapReader,
        eq_tx: rtrb::Producer<EqCoefs>,
    }

    fn rig(channels: u16) -> Rig {
        let control = Arc::new(Control::default());
        let (tx, pcm) = pcm_ring(RATE as usize, 256);
        let (eq_tx, eq_rx) = rtrb::RingBuffer::new(8);
        let (tap_w, tap_r) = tap(64);
        let (cw, cr) = clock();
        let format = StreamFormat {
            sample_rate: RATE,
            channels,
            buffer_frames: Some(512),
        };
        let r = Renderer::new(RendererParts {
            control: control.clone(),
            pcm,
            eq_rx,
            eq: EqCoefs::from_settings(&EqSettings::default(), RATE),
            tap: tap_w,
            clock: cw,
            format,
        });
        Rig {
            r,
            tx,
            control,
            clock: cr,
            tap: tap_r,
            eq_tx,
        }
    }

    fn ramp(frames: usize, from: u64) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v = ((from + i as u64) % 1000) as f32 / 1000.0;
                [v, -v]
            })
            .collect()
    }

    fn info(host_ns: u64) -> CallbackInfo {
        CallbackInfo {
            host_ns,
            output_latency_ns: 0,
        }
    }

    #[test]
    fn plays_samples_through_unchanged_at_full_volume() {
        let mut g = rig(2);
        let gen_ = g.control.bump_generation();
        g.tx.push(gen_, 1, 0, &ramp(512, 0));
        g.control.set_state(PlayState::Playing);
        let mut out = vec![0.0; 512 * 2];
        g.r.render(&mut out, info(0));
        assert_eq!(out, ramp(512, 0));
        let s = g.clock.snapshot();
        assert_eq!((s.track, s.frame, s.starved), (1, 0, false));
        assert_eq!(g.control.underruns(), 0);
    }

    #[test]
    fn stopped_and_paused_output_silence_without_consuming() {
        let mut g = rig(2);
        let gen_ = g.control.bump_generation();
        g.tx.push(gen_, 1, 0, &ramp(256, 1));
        let mut out = vec![1.0; 256 * 2];
        g.control.set_state(PlayState::Paused);
        g.r.render(&mut out, info(0));
        assert!(out.iter().all(|&s| s == 0.0));
        g.control.set_state(PlayState::Playing);
        g.r.render(&mut out, info(1));
        assert_eq!(
            out,
            ramp(256, 1),
            "resume continues at the exact paused frame"
        );
    }

    #[test]
    fn seek_discards_old_generation_and_moves_clock() {
        let mut g = rig(2);
        let g1 = g.control.bump_generation();
        g.tx.push(g1, 1, 0, &vec![0.9; 2048 * 2]);
        g.control.set_state(PlayState::Playing);
        let mut out = vec![0.0; 512 * 2];
        g.r.render(&mut out, info(0));
        let before = g.clock.snapshot().epoch;

        let g2 = g.control.bump_generation();
        g.tx.push(g2, 1, 96_000, &vec![0.1; 1024 * 2]);
        g.r.render(&mut out, info(1));
        assert!(
            out.iter().all(|&s| s == 0.1),
            "no pre-seek audio after the seek"
        );
        let s = g.clock.snapshot();
        assert_eq!(s.frame, 96_000);
        assert_ne!(s.epoch, before);
    }

    #[test]
    fn gapless_boundary_is_published_in_the_clock() {
        let mut g = rig(2);
        let gen_ = g.control.bump_generation();
        g.tx.push(gen_, 1, 5_000, &vec![0.2; 100 * 2]);
        g.tx.push(gen_, 2, 0, &vec![0.3; 400 * 2]);
        g.control.set_state(PlayState::Playing);
        let mut out = vec![0.0; 512 * 2];
        g.r.render(&mut out, info(0));
        let s = g.clock.snapshot();
        assert_eq!((s.track, s.frame), (1, 5_000));
        assert_eq!(s.boundary, Some((2, 100, 0)));
        assert_eq!(out[99 * 2], 0.2);
        assert_eq!(out[100 * 2], 0.3);
    }

    #[test]
    fn underrun_outputs_silence_and_is_counted_once_started() {
        let mut g = rig(2);
        g.control.set_state(PlayState::Playing);
        let mut out = vec![1.0; 256 * 2];
        g.r.render(&mut out, info(0));
        assert!(out.iter().all(|&s| s == 0.0));
        assert_eq!(
            g.control.underruns(),
            0,
            "waiting for the first audio is not an underrun"
        );
        assert!(g.clock.snapshot().starved);

        let gen_ = g.control.generation();
        g.tx.push(gen_, 1, 0, &vec![0.5; 100 * 2]);
        g.r.render(&mut out, info(1));
        assert_eq!(out[99 * 2], 0.5);
        assert_eq!(out[100 * 2], 0.0);
        assert_eq!(g.control.underruns(), 1);
    }

    #[test]
    fn waiting_for_a_new_generation_is_not_an_underrun() {
        let mut g = rig(2);
        let g1 = g.control.bump_generation();
        g.tx.push(g1, 1, 0, &[0.5; 512 * 2]);
        g.control.set_state(PlayState::Playing);
        let mut out = vec![0.0; 256 * 2];
        g.r.render(&mut out, info(0));
        // A seek / new track: old audio is discarded and the new one is not there yet.
        g.control.bump_generation();
        g.r.render(&mut out, info(1));
        g.r.render(&mut out, info(2));
        assert!(out.iter().all(|&s| s == 0.0));
        assert_eq!(g.control.underruns(), 0);
    }

    #[test]
    fn end_of_queue_stops_and_counts() {
        let mut g = rig(2);
        let gen_ = g.control.bump_generation();
        g.tx.push(gen_, 1, 0, &[0.5; 10 * 2]);
        g.tx.push_end_of_queue(gen_, 1, 10);
        g.control.set_state(PlayState::Playing);
        let mut out = vec![0.0; 64 * 2];
        g.r.render(&mut out, info(0));
        assert_eq!(g.control.state(), PlayState::Stopped);
        assert_eq!(g.control.queue_ended_count(), 1);
        assert_eq!(g.control.underruns(), 0);
    }

    #[test]
    fn volume_balance_and_channel_mapping() {
        let mut g = rig(1);
        let gen_ = g.control.bump_generation();
        g.tx.push(gen_, 1, 0, &[1.0, 0.0].repeat(2048));
        g.control.set_state(PlayState::Playing);
        g.control.set_volume(0.5);
        g.control.set_balance(1.0); // hard right: left channel silent
        let mut out = vec![0.0; 1024];
        g.r.render(&mut out, info(0)); // ramp happens here
        g.r.render(&mut out, info(1));
        assert!(
            out.iter().all(|&s| s == 0.0),
            "left-only input panned hard right is silent"
        );

        let mut g = rig(4);
        let gen_ = g.control.bump_generation();
        g.tx.push(gen_, 1, 0, &[0.25, 0.5].repeat(64));
        g.control.set_state(PlayState::Playing);
        let mut out = vec![9.0; 64 * 4];
        g.r.render(&mut out, info(0));
        assert_eq!(&out[..4], &[0.25, 0.5, 0.0, 0.0]);
    }

    #[test]
    fn tap_receives_post_eq_pre_volume_samples() {
        let mut g = rig(2);
        let gen_ = g.control.bump_generation();
        g.tx.push(gen_, 3, 1000, &vec![0.5; 512 * 2]);
        g.control.set_state(PlayState::Playing);
        g.control.set_volume(0.0);
        let mut out = vec![0.0; 512 * 2];
        g.r.render(&mut out, info(0));
        let chunk = g.tap.pop().unwrap();
        assert_eq!(
            (chunk.track, chunk.start_frame, chunk.frames),
            (3, 1000, 256)
        );
        assert!(chunk.data().iter().all(|&s| s == 0.5), "tap ignores volume");

        // EQ preamp applies to the tap.
        let s = EqSettings {
            enabled: true,
            preamp_db: 6.0206,
            ..Default::default()
        };
        g.eq_tx.push(EqCoefs::from_settings(&s, RATE)).unwrap();
        g.tx.push(gen_, 3, 1512, &vec![0.25; 1024 * 2]);
        g.r.render(&mut out, info(1));
        g.r.render(&mut out, info(2));
        let mut last = None;
        while let Some(c) = g.tap.pop() {
            last = Some(c);
        }
        let last = last.unwrap();
        assert!((last.data()[last.data().len() - 1] - 0.5).abs() < 1e-3);
    }

    #[test]
    fn start_latency_is_measured_from_request_to_first_audio() {
        let mut g = rig(2);
        let gen_ = g.control.bump_generation();
        g.control.mark_start_request(gen_, 1_000);
        g.control.set_state(PlayState::Playing);
        let mut out = vec![0.0; 256 * 2];
        g.r.render(&mut out, info(2_000)); // no data yet
        g.tx.push(gen_, 1, 0, &vec![0.5; 256 * 2]);
        g.r.render(&mut out, info(9_000));
        assert_eq!(g.control.last_start_latency_ns(), 8_000);
    }
}
