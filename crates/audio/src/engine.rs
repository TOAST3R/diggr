//! The public audio engine: owns the always-open output stream, the decode thread, and the
//! supervisor that rebuilds the stream when the device goes away.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use platform::{
    AudioSink, ErrorFn, FileSource, PlatformError, Priority, RenderFn, SinkConfig, SinkError,
    SinkHandle, Spawner, StreamFormat, TrackRef,
};

use crate::clock::{ClockReader, Position, clock};
use crate::eq::{EqCoefs, EqSettings};
use crate::renderer::{Control, Renderer, RendererParts};
use crate::ring::pcm_ring;
use crate::tap::{TapReader, tap};
use crate::trycell::TryCell;
use crate::worker::{Command, DecodeWorker, EngineEvent, RepeatMode, WorkerConfig};
use crate::{PlayState, TrackId, TrackInfo, rt_guard};

#[derive(Debug, Clone, Copy)]
pub struct EngineConfig {
    /// Requested device buffer (frames). 256–512 keeps output latency around 5–10 ms.
    pub buffer_frames: Option<u32>,
    /// Decoded audio queued ahead of the device. Absorbs decode hiccups; does not add A/V offset.
    pub ring_secs: f64,
    /// Pre-warm the next track when this much of the current one remains.
    pub prewarm_secs: f64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            buffer_frames: Some(512),
            ring_secs: 0.5,
            prewarm_secs: 30.0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Platform(#[from] PlatformError),
    #[error("audio output failed to start: {0}")]
    Start(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineStats {
    pub format: StreamFormat,
    pub underruns: u64,
    /// Play/seek request → first audio written to the device, in milliseconds.
    pub last_start_latency_ms: f64,
    /// Allocations detected in the audio callback (needs [`rt_guard::GuardAlloc`]).
    pub rt_alloc_violations: u64,
    pub queue_ended: u64,
}

struct Shared {
    format: Mutex<StreamFormat>,
    eq: Mutex<EqSettings>,
    config: EngineConfig,
}

enum Supervise {
    Device(SinkError),
    Shutdown,
}

pub struct Engine {
    sink: Arc<dyn AudioSink>,
    shared: Arc<Shared>,
    control: Arc<Control>,
    commands: Sender<Command>,
    events: Receiver<EngineEvent>,
    supervisor: Sender<Supervise>,
    eq_tx: rtrb::Producer<EqCoefs>,
    clock: ClockReader,
    tap: Option<TapReader>,
    queue: Vec<TrackRef>,
    loaded: HashMap<TrackId, usize>,
    infos: HashMap<TrackId, TrackInfo>,
    unread: Vec<EngineEvent>,
    last_index: Option<usize>,
    repeat: RepeatMode,
}

impl Engine {
    /// Opens the output device immediately (it stays open, emitting silence while idle).
    pub fn new(
        sink: Arc<dyn AudioSink>,
        spawner: &dyn Spawner,
        files: Arc<dyn FileSource>,
        config: EngineConfig,
    ) -> Result<Self, EngineError> {
        let format = sink.default_format(&SinkConfig {
            buffer_frames: config.buffer_frames,
        })?;
        let control = Arc::new(Control::default());
        let ring_frames = ((config.ring_secs * format.sample_rate as f64) as usize).max(4096);
        let (pcm_tx, pcm_rx) = pcm_ring(ring_frames, 1024);
        let (eq_tx, eq_rx) = rtrb::RingBuffer::new(16);
        let (tap_w, tap_r) = tap(256);
        let (clock_w, clock_r) = clock();
        let eq = EqSettings::default();
        let renderer = Renderer::new(RendererParts {
            control: control.clone(),
            pcm: pcm_rx,
            eq_rx,
            eq: EqCoefs::from_settings(&eq, format.sample_rate),
            tap: tap_w,
            clock: clock_w,
            format,
        });
        let shared = Arc::new(Shared {
            format: Mutex::new(format),
            eq: Mutex::new(eq),
            config,
        });

        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (evt_tx, evt_rx) = mpsc::channel();
        let (sup_tx, sup_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let sup = Supervisor {
            sink: sink.clone(),
            cell: Arc::new(TryCell::new(renderer)),
            shared: shared.clone(),
            control: control.clone(),
            clock: clock_r.clone(),
            commands: cmd_tx.clone(),
            inbox: sup_rx,
            outbox: sup_tx.clone(),
        };
        spawner.spawn(
            "audio-supervisor",
            Priority::Normal,
            Box::new(move || sup.run(format, ready_tx)),
        )?;
        ready_rx
            .recv()
            .map_err(|_| EngineError::Start("supervisor exited".into()))?
            .map_err(EngineError::Start)?;

        let worker = DecodeWorker::new(
            WorkerConfig {
                out_rate: format.sample_rate,
                prewarm_secs: config.prewarm_secs,
            },
            files,
            cmd_rx,
            evt_tx,
            pcm_tx,
        );
        spawner.spawn(
            "audio-decode",
            Priority::High,
            Box::new(move || worker.run()),
        )?;

        Ok(Self {
            sink,
            shared,
            control,
            commands: cmd_tx,
            events: evt_rx,
            supervisor: sup_tx,
            eq_tx,
            clock: clock_r,
            tap: Some(tap_r),
            queue: Vec::new(),
            loaded: HashMap::new(),
            infos: HashMap::new(),
            unread: Vec::new(),
            last_index: None,
            repeat: RepeatMode::Off,
        })
    }

    // ---- queue & transport -------------------------------------------------------------

    /// Replaces the play queue. The playing track keeps playing if it is still in the queue.
    pub fn set_queue(&mut self, tracks: Vec<TrackRef>) {
        let playing = self
            .current_index()
            .and_then(|i| self.queue.get(i).cloned());
        let current = playing.and_then(|t| tracks.iter().position(|x| *x == t));
        self.queue = tracks.clone();
        self.last_index = current;
        self.loaded.clear();
        let _ = self.commands.send(Command::SetQueue { tracks, current });
    }

    pub fn queue(&self) -> &[TrackRef] {
        &self.queue
    }

    /// Starts the queue entry `index` from the beginning.
    pub fn play_index(&mut self, index: usize) {
        if index >= self.queue.len() {
            return;
        }
        let generation = self.control.bump_generation();
        self.control
            .mark_start_request(generation, self.sink.now_ns());
        self.control.set_state(PlayState::Playing);
        self.last_index = Some(index);
        let _ = self.commands.send(Command::Load {
            generation,
            index,
            start_secs: 0.0,
        });
    }

    /// Resumes if paused, otherwise (re)starts the current queue entry.
    pub fn play(&mut self) {
        match self.control.state() {
            PlayState::Paused => self.control.set_state(PlayState::Playing),
            PlayState::Playing => {}
            PlayState::Stopped => {
                let index = self.current_index().unwrap_or(0);
                self.play_index(index);
            }
        }
    }

    pub fn pause(&self) {
        if self.control.state() == PlayState::Playing {
            self.control.set_state(PlayState::Paused);
        }
    }

    pub fn resume(&self) {
        if self.control.state() == PlayState::Paused {
            self.control.set_state(PlayState::Playing);
        }
    }

    pub fn toggle_pause(&mut self) {
        match self.control.state() {
            PlayState::Playing => self.pause(),
            _ => self.play(),
        }
    }

    pub fn stop(&self) {
        self.control.bump_generation();
        self.control.set_state(PlayState::Stopped);
        let _ = self.commands.send(Command::Stop);
    }

    /// Seeks within the current track to `secs` (keeps the paused/playing state).
    pub fn seek(&self, secs: f64) {
        if self.control.state() == PlayState::Stopped {
            return;
        }
        let generation = self.control.bump_generation();
        self.control
            .mark_start_request(generation, self.sink.now_ns());
        let _ = self.commands.send(Command::Seek {
            generation,
            secs: secs.max(0.0),
        });
    }

    /// Jumps within `track` to `target_secs`, sample-accurately at the first of `at_secs`
    /// (track time, e.g. upcoming downbeats) not yet sent to the device, with a 2 ms
    /// crossfade and no gap. Reports [`EngineEvent::JumpScheduled`] or
    /// [`EngineEvent::JumpMissed`].
    pub fn seek_at(&self, track: TrackId, at_secs: Vec<f64>, target_secs: f64) {
        if self.control.state() == PlayState::Stopped {
            return;
        }
        let _ = self.commands.send(Command::SeekAt {
            track,
            at_secs,
            target_secs,
        });
    }

    /// Loops `[start, end)` seconds of `track` gaplessly (`None` stops looping). The loop ends
    /// on a seek, stop, or track change. Reports [`EngineEvent::LoopChanged`] or
    /// [`EngineEvent::LoopRejected`].
    pub fn set_loop(&self, track: TrackId, region: Option<(f64, f64)>) {
        let _ = self.commands.send(Command::SetLoop { track, region });
    }

    /// Next queue entry; wraps to the first with [`RepeatMode::All`].
    pub fn next(&mut self) {
        let len = self.queue.len();
        let next = match self.current_index() {
            Some(i) if self.repeat == RepeatMode::All && len > 0 => (i + 1) % len,
            Some(i) => i + 1,
            None => 0,
        };
        self.play_index(next);
    }

    /// What follows each track: nothing, the next one (wrapping), or itself. Gapless.
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
        let _ = self.commands.send(Command::SetRepeat(mode));
    }

    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    pub fn previous(&mut self) {
        let prev = self.current_index().map_or(0, |i| i.saturating_sub(1));
        self.play_index(prev);
    }

    pub fn state(&self) -> PlayState {
        self.control.state()
    }

    // ---- levels & EQ ---------------------------------------------------------------------

    /// 0.0..=1.0
    pub fn set_volume(&self, volume: f32) {
        self.control.set_volume(volume);
    }

    pub fn volume(&self) -> f32 {
        self.control.volume()
    }

    /// -1.0 (left) ..= 1.0 (right)
    pub fn set_balance(&self, balance: f32) {
        self.control.set_balance(balance);
    }

    pub fn balance(&self) -> f32 {
        self.control.balance()
    }

    pub fn set_eq(&mut self, settings: EqSettings) {
        let settings = settings.clamped();
        *self.shared.eq.lock().unwrap() = settings;
        let rate = self.shared.format.lock().unwrap().sample_rate;
        // If the ring is full the callback is not draining it (no stream); the supervisor
        // applies the stored settings when the stream is rebuilt.
        let _ = self.eq_tx.push(EqCoefs::from_settings(&settings, rate));
    }

    pub fn eq(&self) -> EqSettings {
        *self.shared.eq.lock().unwrap()
    }

    // ---- position, info, events ---------------------------------------------------------

    /// Monotonic time in the clock's time base.
    pub fn now_ns(&self) -> u64 {
        self.sink.now_ns()
    }

    /// The audible position right now.
    pub fn position(&mut self) -> Position {
        let now = self.sink.now_ns();
        self.clock.position(now)
    }

    /// An independent clock reader for another thread (e.g. the renderer of visuals).
    pub fn clock_reader(&self) -> ClockReader {
        self.clock.clone()
    }

    /// Positive values make readers report later positions (±50 ms).
    pub fn set_av_offset_ms(&self, ms: i64) {
        self.clock.set_offset_ms(ms);
    }

    /// The post-EQ sample tap. There is a single reader.
    pub fn take_tap(&mut self) -> Option<TapReader> {
        self.tap.take()
    }

    /// Queue index of the audible track (or of the last requested one).
    pub fn current_index(&mut self) -> Option<usize> {
        self.sync_events();
        let track = self.clock.snapshot().track;
        self.loaded.get(&track).copied().or(self.last_index)
    }

    pub fn track_info(&mut self, id: TrackId) -> Option<&TrackInfo> {
        self.sync_events();
        self.infos.get(&id)
    }

    /// Events since the previous call.
    pub fn poll_events(&mut self) -> Vec<EngineEvent> {
        self.sync_events();
        std::mem::take(&mut self.unread)
    }

    fn sync_events(&mut self) {
        while let Ok(e) = self.events.try_recv() {
            match &e {
                EngineEvent::TrackLoaded { id, index, .. } => {
                    self.loaded.insert(*id, *index);
                    if self.loaded.len() > 256 {
                        let oldest = *self.loaded.keys().min().expect("non-empty");
                        self.loaded.remove(&oldest);
                    }
                }
                EngineEvent::TrackInfo { id, info, .. } => {
                    self.infos.insert(*id, info.clone());
                    if self.infos.len() > 256 {
                        let oldest = *self.infos.keys().min().expect("non-empty");
                        self.infos.remove(&oldest);
                    }
                }
                _ => {}
            }
            self.unread.push(e);
        }
    }

    pub fn stats(&self) -> EngineStats {
        EngineStats {
            format: *self.shared.format.lock().unwrap(),
            underruns: self.control.underruns(),
            last_start_latency_ms: self.control.last_start_latency_ns() as f64 / 1e6,
            rt_alloc_violations: rt_guard::violations(),
            queue_ended: self.control.queue_ended_count(),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        let _ = self.supervisor.send(Supervise::Shutdown);
    }
}

/// Owns the output stream; rebuilds it when the device is lost.
struct Supervisor {
    sink: Arc<dyn AudioSink>,
    cell: Arc<TryCell<Renderer>>,
    shared: Arc<Shared>,
    control: Arc<Control>,
    clock: ClockReader,
    commands: Sender<Command>,
    inbox: Receiver<Supervise>,
    outbox: Sender<Supervise>,
}

impl Supervisor {
    fn run(mut self, format: StreamFormat, ready: Sender<Result<(), String>>) {
        let mut stream = match self.start(format) {
            Ok(s) => {
                let _ = ready.send(Ok(()));
                Some(s)
            }
            Err(e) => {
                let _ = ready.send(Err(e.to_string()));
                return;
            }
        };
        while let Ok(msg) = self.inbox.recv() {
            match msg {
                Supervise::Shutdown => return,
                Supervise::Device(SinkError::DeviceLost) => {
                    drop(stream.take());
                    match self.rebuild() {
                        Some(s) => stream = Some(s),
                        None => return,
                    }
                }
                Supervise::Device(_) => {}
            }
        }
        drop(stream);
    }

    fn start(&self, format: StreamFormat) -> Result<Box<dyn SinkHandle>, PlatformError> {
        let cell = self.cell.clone();
        let render: RenderFn = Box::new(move |out, info| {
            if cell.try_with(|r| r.render(out, info)).is_none() {
                out.fill(0.0);
            }
        });
        let outbox = self.outbox.clone();
        let on_error: ErrorFn = Box::new(move |e| {
            let _ = outbox.send(Supervise::Device(e));
        });
        self.sink.start(&format, render, on_error)
    }

    /// Retries until a device is available again. Returns `None` on shutdown.
    fn rebuild(&mut self) -> Option<Box<dyn SinkHandle>> {
        let now = self.sink.now_ns();
        let pos = self.clock.position(now);
        let was_playing = self.control.state() == PlayState::Playing;
        let cfg = SinkConfig {
            buffer_frames: self.shared.config.buffer_frames,
        };
        loop {
            while let Ok(msg) = self.inbox.try_recv() {
                if matches!(msg, Supervise::Shutdown) {
                    return None;
                }
            }
            if let Ok(format) = self.sink.default_format(&cfg) {
                let old_rate = self.shared.format.lock().unwrap().sample_rate;
                let eq =
                    EqCoefs::from_settings(&self.shared.eq.lock().unwrap(), format.sample_rate);
                // The old stream is gone, so the callback cannot be holding the renderer.
                while self.cell.try_with(|r| r.reconfigure(format, eq)).is_none() {
                    std::thread::sleep(Duration::from_millis(1));
                }
                *self.shared.format.lock().unwrap() = format;
                if format.sample_rate != old_rate && pos.track != 0 {
                    let generation = self.control.bump_generation();
                    self.control.mark_start_request(generation, now);
                    let _ = self.commands.send(Command::Reload {
                        generation,
                        out_rate: format.sample_rate,
                        track: pos.track,
                        secs: pos.seconds(),
                    });
                    if was_playing {
                        self.control.set_state(PlayState::Playing);
                    }
                }
                if let Ok(stream) = self.start(format) {
                    return Some(stream);
                }
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}
