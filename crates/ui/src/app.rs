//! The eframe application: main player, equalizer and playlist stacked in one borderless
//! window, plus fullscreen visual mode.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use audio::{Engine, EngineEvent, EqPreset, EqPresets, PlayState, Position, TapReader, TrackInfo};
use egui::{
    Color32, Id, Key, Modifiers, Pos2, Rect, Sense, TextureHandle, Ui, ViewportCommand, pos2, vec2,
};
use platform::{FileSource, Spawner, TrackRef};

use crate::crates::{CrateId, Crates, MetaKey, PLAYLIST};
use crate::eqcurve;
use crate::files;
use crate::format;
use crate::fullscreen::{SceneFrame, VisualScene};
use crate::metadata::{MetaResult, MetaWorker};
use crate::playlist::{ClickMods, EntryId, EntryStatus};
use crate::settings::{PRESETS_FILE, Repeat, SETTINGS_FILE, Settings, Store, VisMode};
use crate::skin::LoadedSkin;
use crate::spectrum::{Analyzer, BARS};
use crate::widgets::{self, Skinned, SliderSprites, color};

pub type EngineFactory = Box<dyn FnOnce() -> Result<Engine, String> + Send>;

/// What the host app provides.
pub struct AppContext {
    pub engine: EngineFactory,
    pub spawner: Arc<dyn Spawner>,
    pub files: Arc<dyn FileSource>,
    /// Where settings/playlist/presets live; `None` disables persistence.
    pub store: Option<Store>,
    /// Files from the command line: replace the Playlist crate and play.
    pub open: Vec<PathBuf>,
    pub scene: Box<dyn VisualScene>,
    pub startup: Startup,
    /// Music analysis ahead of the playhead (beats, sections) for the visuals.
    pub analysis: Option<analysis::AnalysisService>,
    /// Where annotation mode saves its JSON files.
    pub annotations_dir: Option<PathBuf>,
    /// Native-rate track overviews for the waveform section.
    pub overviews: Option<analysis::overview::OverviewService>,
    /// Renders a track's visual show to a video (playlist menu "Render show…").
    pub show_renderer: Option<Box<dyn crate::render_job::ShowRenderer>>,
}

#[derive(Clone, Copy)]
pub struct Startup {
    pub process_start: Instant,
    /// Print time-to-first-frame.
    pub report: bool,
    /// Quit right after the first frame (for launch-time measurements).
    pub exit_after_first_frame: bool,
}

enum EngineSlot {
    Starting(Receiver<Result<Engine, String>>),
    Ready(Box<Engine>),
    Failed(String),
}

struct Fullscreen {
    restore: Option<Rect>,
    last_pointer: Instant,
    pointer_at: Option<Pos2>,
}

pub struct WinampApp {
    skin: LoadedSkin,
    def: Arc<crate::skin::SkinDef>,
    tex: Option<TextureHandle>,
    store: Option<Store>,
    settings: Settings,
    presets: EqPresets,
    /// The crates: the window edits the shown one, the engine queue comes from the playing one.
    crates: Crates,
    /// Entry ids in engine-queue order, and the crate they belong to.
    queue: Vec<EntryId>,
    queue_crate: CrateId,
    queue_dirty: bool,
    shuffle_seed: u64,
    engine: EngineSlot,
    /// Play once the engine is up: an entry, or whatever Play would start.
    pending_play: Option<Option<(CrateId, EntryId)>>,
    /// A waiting entry that starts as soon as its audio arrives.
    armed: Option<(CrateId, EntryId)>,
    tap: Option<TapReader>,
    analyzer: Analyzer,
    meta: Option<MetaWorker<MetaKey>>,
    now_playing: Option<TrackInfo>,
    position: Position,
    seek_drag: Option<f32>,
    title_offset: usize,
    title_tick: f64,
    pl_scroll: usize,
    pl_drag_from: Option<usize>,
    pl_resize_acc: f32,
    name_dialog: Option<NameDialog>,
    /// A crate with entries waiting for "Delete crate…" to be confirmed.
    confirm_delete: Option<CrateId>,
    fullscreen: Option<Fullscreen>,
    scene: Box<dyn VisualScene>,
    scene_ready: bool,
    render_state: Option<eframe::egui_wgpu::RenderState>,
    last_frame: Instant,
    startup: Startup,
    first_frame_done: bool,
    /// The shortcuts help panel (`H` / `F1`).
    help: bool,
    overviews: Option<analysis::overview::OverviewService>,
    /// The audible track's overview, as far as it is built.
    overview: Option<Arc<analysis::overview::Overview>>,
    nav: Nav,
    /// The spectrogram window (`S`) and whether it is open.
    spectro: crate::spectrogram::SpectrogramWindow,
    spectro_open: bool,
    show_renderer: Option<Box<dyn crate::render_job::ShowRenderer>>,
    /// The running (or just finished) show render and its output file name.
    render_job: Option<(Arc<dyn crate::render_job::RenderJob>, String)>,
    /// The "Render show" options dialog, while open.
    render_dialog: Option<crate::render_job::RenderDialog>,
    /// Looks the renderer offers, read when the dialog opens.
    render_looks: Vec<(String, String)>,
    /// `WINAMP_FRAME_STATS=1`: per-second frame phase timings on stderr and in
    /// `$TMPDIR/winamp_frame_stats.log`.
    profile: Option<FrameProfile>,
    /// `WINAMP_AUTO_FULLSCREEN=1`: enter fullscreen once playback starts (for unattended runs).
    auto_fullscreen: bool,
    /// `WINAMP_AUTO_QUIT_SECS=n`: close the app after n seconds.
    auto_quit: Option<Instant>,
    settings_dirty: Option<Instant>,
    last_size: Option<egui::Vec2>,
    message: Option<(String, Instant)>,
    analysis: Option<analysis::AnalysisService>,
    files: Arc<dyn FileSource>,
    /// Engine track instances → files, from `TrackLoaded` events.
    track_refs: std::collections::HashMap<audio::TrackId, TrackRef>,
    score: Option<Arc<analysis::SongScore>>,
    strip: bool,
    annotating: Option<analysis::eval::Annotations>,
    annotations_dir: Option<PathBuf>,
}

const SAVE_DELAY: Duration = Duration::from_millis(800);

impl WinampApp {
    pub fn new(cc: &eframe::CreationContext<'_>, ctx: AppContext) -> Self {
        Self::build(cc.egui_ctx.clone(), cc.wgpu_render_state.clone(), ctx)
    }

    fn build(
        egui_ctx: egui::Context,
        render_state: Option<eframe::egui_wgpu::RenderState>,
        ctx: AppContext,
    ) -> Self {
        let store = ctx.store;
        let settings = store.as_ref().map(Store::load_settings).unwrap_or_default();
        let presets = store.as_ref().map(Store::load_presets).unwrap_or_default();
        let mut crates = store.as_ref().map_or_else(Crates::in_memory, Crates::open);

        // Open the audio device in the background: the window must not wait for it.
        let (tx, rx) = std::sync::mpsc::channel();
        let factory = ctx.engine;
        let wake_ctx = egui_ctx.clone();
        std::thread::Builder::new()
            .name("engine-start".into())
            .spawn(move || {
                let _ = tx.send(factory());
                wake_ctx.request_repaint();
            })
            .expect("spawn engine starter");

        let wake_ctx = egui_ctx.clone();
        let meta = MetaWorker::start(&*ctx.spawner, ctx.files.clone(), move || {
            wake_ctx.request_repaint()
        })
        .ok();

        let mut pending_play = None;
        if !ctx.open.is_empty() {
            let added = crates.replace_playlist(
                files::expand(&ctx.open)
                    .into_iter()
                    .map(|p| TrackRef::new(p.to_string_lossy())),
            );
            pending_play = Some(added.first().map(|(key, _)| *key));
            if let Some(m) = &meta {
                m.request(added);
            }
        }
        if let Some(m) = &meta {
            m.request(crates.take_pending_meta());
        }
        let message = Some(crates.take_messages().join("\n"))
            .filter(|m| !m.is_empty())
            .map(|m| (m, Instant::now()));
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);

        let spectro = crate::spectrogram::SpectrogramWindow::new(
            settings.spectrogram.clone(),
            Some(Arc::new(analysis::detail::DetailService::new(
                ctx.spawner.clone(),
                ctx.files.clone(),
            ))),
        );
        let skin = LoadedSkin::default_skin();
        Self {
            def: Arc::new(skin.def.clone()),
            skin,
            tex: None,
            store,
            analyzer: Analyzer::new(48_000),
            settings,
            presets,
            queue: Vec::new(),
            queue_crate: crates.playing_id(),
            crates,
            queue_dirty: true,
            shuffle_seed: seed,
            engine: EngineSlot::Starting(rx),
            pending_play,
            armed: None,
            tap: None,
            meta,
            now_playing: None,
            position: Position {
                track: 0,
                frame: 0,
                sample_rate: 0,
                state: PlayState::Stopped,
                discontinuity: false,
            },
            seek_drag: None,
            title_offset: 0,
            title_tick: 0.0,
            pl_scroll: 0,
            pl_drag_from: None,
            pl_resize_acc: 0.0,
            name_dialog: None,
            confirm_delete: None,
            fullscreen: None,
            scene: ctx.scene,
            scene_ready: false,
            render_state,
            last_frame: Instant::now(),
            startup: ctx.startup,
            first_frame_done: false,
            profile: std::env::var_os("WINAMP_FRAME_STATS").map(|_| FrameProfile::new()),
            auto_fullscreen: std::env::var_os("WINAMP_AUTO_FULLSCREEN").is_some(),
            auto_quit: std::env::var("WINAMP_AUTO_QUIT_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .map(|s| Instant::now() + Duration::from_secs(s)),
            settings_dirty: None,
            last_size: None,
            message,
            analysis: ctx.analysis,
            files: ctx.files.clone(),
            track_refs: std::collections::HashMap::new(),
            score: None,
            strip: false,
            annotating: None,
            annotations_dir: ctx.annotations_dir,
            help: false,
            overviews: ctx.overviews,
            overview: None,
            nav: Nav::default(),
            spectro,
            spectro_open: false,
            show_renderer: ctx.show_renderer,
            render_job: None,
            render_dialog: None,
            render_looks: Vec::new(),
        }
    }

    /// Window size in points for the current settings.
    pub fn window_size(settings: &Settings, skin: &LoadedSkin) -> egui::Vec2 {
        let d = &skin.def;
        let mut h = d.main_size.1 as f32;
        if settings.show_waveform {
            h += crate::waveform::HEIGHT as f32;
        }
        if settings.show_eq {
            h += d.eq_size.1 as f32;
        }
        if settings.show_playlist {
            h += (d.pl_top_h + d.pl_bottom_h + settings.playlist_rows * d.pl_row_h) as f32;
        }
        vec2(d.main_size.0 as f32, h) * settings.scale as f32
    }

    fn engine(&mut self) -> Option<&mut Engine> {
        match &mut self.engine {
            EngineSlot::Ready(e) => Some(e),
            _ => None,
        }
    }

    fn mark_settings(&mut self) {
        self.settings_dirty.get_or_insert_with(Instant::now);
    }

    /// A crate's entries changed: save it soon, and rebuild the engine queue if it is playing.
    fn mark_crate(&mut self, id: CrateId) {
        self.crates.touch(id);
        if id == self.crates.playing_id() {
            self.queue_dirty = true;
        }
    }

    fn mark_shown(&mut self) {
        self.mark_crate(self.crates.shown_id());
    }

    fn notify(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), Instant::now()));
    }

    // ---- engine glue ---------------------------------------------------------------------

    fn poll_engine_start(&mut self) {
        let EngineSlot::Starting(rx) = &self.engine else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(mut engine)) => {
                let s = &self.settings;
                engine.set_volume(s.volume);
                engine.set_balance(s.balance);
                engine.set_eq(s.eq);
                engine.set_repeat(s.repeat.to_engine());
                engine.set_av_offset_ms(s.av_offset_ms);
                self.tap = engine.take_tap();
                self.analyzer.set_rate(engine.stats().format.sample_rate);
                self.engine = EngineSlot::Ready(Box::new(engine));
                self.queue_dirty = true;
            }
            Ok(Err(e)) => self.engine = EngineSlot::Failed(e),
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(_) => self.engine = EngineSlot::Failed("audio engine did not start".into()),
        }
    }

    /// Rebuilds the engine queue from the playing crate (and shuffle order).
    fn sync_queue(&mut self, first: Option<EntryId>) {
        let playing = self.crates.playing();
        let queue = playing.queue(
            self.settings.shuffle,
            first.or(playing.current()),
            self.shuffle_seed,
        );
        let tracks: Vec<TrackRef>;
        (self.queue, tracks) = queue.into_iter().unzip();
        self.queue_crate = self.crates.playing_id();
        self.queue_dirty = false;
        if let EngineSlot::Ready(e) = &mut self.engine {
            e.set_queue(tracks);
        }
    }

    /// Starts an entry, which makes its crate the playing one. A waiting entry is armed
    /// instead: it starts when its audio arrives, and the current track plays on meanwhile.
    fn play_entry(&mut self, crate_id: CrateId, id: EntryId) {
        let Some(entry) = self.crates.get(crate_id).and_then(|p| p.get(id)) else {
            return;
        };
        match &entry.status {
            EntryStatus::Waiting(_) => {
                self.armed = Some((crate_id, id));
                return;
            }
            EntryStatus::Unavailable(reason) => {
                let text = format!("{}: {reason}", entry.display_name());
                self.notify(text);
                return;
            }
            _ => {}
        }
        if self.engine().is_none() {
            self.pending_play = Some(Some((crate_id, id)));
            return;
        }
        self.armed = None;
        if self.crates.playing_id() != crate_id && self.crates.set_playing(crate_id) {
            self.queue_dirty = true;
        }
        // In shuffle mode the chosen track starts a fresh shuffled order.
        if self.queue_dirty || self.settings.shuffle || !self.queue.contains(&id) {
            self.shuffle_seed = self
                .shuffle_seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            self.sync_queue(Some(id));
        }
        if let Some(q) = self.queue.iter().position(|&x| x == id) {
            self.crates.playing_mut().set_current(Some(id));
            if let Some(e) = self.engine() {
                e.play_index(q);
            }
        }
    }

    /// Play: resumes when paused, restarts the playing track, and when stopped starts the
    /// shown crate (at its current entry).
    fn play(&mut self) {
        if self.crates.shown().is_empty() {
            self.open_files_dialog(Open::AddAndPlay);
            return;
        }
        let Some(e) = self.engine() else {
            self.pending_play = Some(None);
            return;
        };
        match e.state() {
            PlayState::Paused => e.resume(),
            PlayState::Playing => {
                if let Some(id) = self.crates.playing().current() {
                    self.play_entry(self.crates.playing_id(), id);
                }
            }
            PlayState::Stopped => {
                let shown = self.crates.shown();
                let id = shown
                    .current()
                    .or_else(|| shown.queue(false, None, 0).first().map(|(id, _)| *id));
                if let Some(id) = id {
                    self.play_entry(self.crates.shown_id(), id);
                }
            }
        }
    }

    fn with_engine(&mut self, f: impl FnOnce(&mut Engine)) {
        if let Some(e) = self.engine() {
            f(e);
        }
    }

    // ---- structure navigation ----------------------------------------------------------

    /// Jumps to `target(score, now)`, on the next downbeat when the beat grid allows.
    fn structure_jump(&mut self, target: fn(&analysis::SongScore, f64) -> Option<f64>) {
        let Some(score) = self.score.clone() else {
            self.message = Some(("Not analyzed yet".into(), Instant::now()));
            return;
        };
        let now = self.position.seconds();
        let Some(t) = target(&score, now) else { return };
        let track = self.position.track;
        if self.nav.loop_region.is_some() {
            self.with_engine(|e| e.set_loop(track, None));
        }
        let at = crate::navigation::downbeats_after(&score, now, crate::navigation::CANDIDATES);
        if crate::navigation::grid_ok(&score, now) && !at.is_empty() {
            self.nav.jump_target = Some(t);
            self.with_engine(|e| e.seek_at(track, at, t));
        } else {
            self.with_engine(|e| e.seek(t));
        }
    }

    fn set_loop(&mut self, region: Option<(f64, f64)>) {
        let track = self.position.track;
        self.with_engine(|e| e.set_loop(track, region));
    }

    /// `L`: loop the current section (at most 32 bars), or stop looping.
    fn section_loop(&mut self) {
        if self.nav.loop_region.is_some() {
            self.set_loop(None);
            return;
        }
        let now = self.position.seconds();
        match self
            .score
            .as_deref()
            .and_then(|s| crate::navigation::section_loop(s, now))
        {
            Some(region) => self.set_loop(Some(region)),
            None => self.message = Some(("Not analyzed yet".into(), Instant::now())),
        }
    }

    /// `Shift+L`: loop 4, 8, 16 bars from the current downbeat (each press doubles).
    fn bar_loop(&mut self) {
        let bars = crate::navigation::next_loop_bars(self.nav.loop_bars);
        let now = self.position.seconds();
        match self
            .score
            .as_deref()
            .and_then(|s| crate::navigation::bar_loop(s, now, bars))
        {
            Some(region) => {
                self.nav.loop_bars = Some(bars);
                self.set_loop(Some(region));
            }
            None => self.message = Some(("Not analyzed yet".into(), Instant::now())),
        }
    }

    fn waveform_section(&mut self, ui: &mut Ui, rect: Rect) {
        let drops = self
            .score
            .as_deref()
            .map(crate::navigation::rise_marks)
            .unwrap_or_default();
        let overview = self.overview.clone();
        let duration = self
            .now_playing
            .as_ref()
            .and_then(|i| i.duration_secs)
            .or(overview
                .as_ref()
                .filter(|o| o.complete)
                .map(|o| o.seconds()));
        let playing = self.position.state != PlayState::Stopped;
        let view = crate::waveform::View {
            overview: overview.as_deref().filter(|_| playing),
            score: self.score.as_deref(),
            now: self.position.seconds(),
            duration: duration.filter(|_| playing),
            zoom_bars: self.settings.waveform_bars,
            loop_region: self.nav.loop_region,
            pending_jump: self.nav.pending_jump,
            drops: &drops,
        };
        match crate::waveform::draw(ui, rect, self.settings.scale as f32, &view) {
            Some(crate::waveform::Action::Seek(t)) => self.with_engine(|e| e.seek(t)),
            Some(crate::waveform::Action::Zoom(bars)) => {
                self.settings.waveform_bars = bars;
                self.mark_settings();
            }
            None => {}
        }
    }

    fn update_audio(&mut self, dt: f32) {
        self.poll_engine_start();
        if self.queue_dirty && self.engine().is_some() {
            self.sync_queue(None);
        }
        if let Some(p) = self.pending_play.take() {
            if self.engine().is_some() {
                match p {
                    Some((c, id)) => self.play_entry(c, id),
                    None => self.play(),
                }
            } else {
                self.pending_play = Some(p);
            }
        }
        if let Some(meta) = &self.meta {
            meta.request(self.crates.take_pending_meta());
            for r in meta.poll() {
                let (c, id) = match r {
                    MetaResult::Info(key, _) | MetaResult::Failed(key) => key,
                };
                let Some(playlist) = self.crates.get_mut(c) else {
                    continue; // deleted meanwhile
                };
                match r {
                    MetaResult::Info(_, i) => {
                        playlist.set_info(id, i.title, i.artist, i.duration_secs)
                    }
                    MetaResult::Failed(_) => playlist.set_failed(id),
                }
                self.crates.touch(c);
            }
        }
        let messages = self.crates.take_messages();
        if !messages.is_empty() {
            self.notify(messages.join("\n"));
        }
        self.check_armed();
        let queue = self.queue.clone();
        let queue_crate = self.queue_crate;
        let EngineSlot::Ready(engine) = &mut self.engine else {
            return;
        };
        for ev in engine.poll_events() {
            match ev {
                EngineEvent::TrackInfo { index, info, .. } => {
                    if let (Some(&id), Some(p)) =
                        (queue.get(index), self.crates.get_mut(queue_crate))
                    {
                        p.set_info(
                            id,
                            info.title.clone(),
                            info.artist.clone(),
                            info.duration_secs,
                        );
                    }
                }
                EngineEvent::TrackFailed { index, .. } => {
                    if let (Some(&id), Some(p)) =
                        (queue.get(index), self.crates.get_mut(queue_crate))
                    {
                        p.set_failed(id);
                    }
                }
                EngineEvent::TrackLoaded { id, track, .. } => {
                    // Keep the recent instances only (ids grow monotonically).
                    self.track_refs.retain(|&k, _| k + 256 > id);
                    self.track_refs.insert(id, track);
                }
                EngineEvent::JumpScheduled { at_secs, .. } => self.nav.pending_jump = Some(at_secs),
                EngineEvent::JumpMissed { .. } => {
                    // Too late to quantize: jump now rather than not at all.
                    self.nav.pending_jump = None;
                    if let Some(t) = self.nav.jump_target.take() {
                        engine.seek(t);
                    }
                }
                EngineEvent::LoopChanged { region, .. } => {
                    self.nav.loop_region = region;
                    if region.is_none() {
                        self.nav.loop_bars = None;
                    }
                }
                EngineEvent::LoopRejected { .. } => {
                    self.nav.loop_bars = None;
                    self.message = Some(("Too late to loop there".into(), Instant::now()));
                }
                EngineEvent::PreWarm { track, .. } => {
                    if let Some(a) = &self.analysis {
                        a.prewarm(&track);
                    }
                    if let Some(o) = &self.overviews {
                        o.request(&track);
                    }
                }
            }
        }
        let live = self.spectro_open && self.spectro.live_mode();
        let mut chunks = Vec::new();
        if let Some(tap) = &mut self.tap {
            while let Some(chunk) = tap.pop() {
                self.analyzer.push(&chunk);
                if live {
                    chunks.push(chunk);
                }
            }
        }
        if live {
            self.spectro
                .push_tap(&chunks, engine.stats().format.sample_rate);
        }
        self.position = engine.position();
        if self.position.discontinuity {
            self.analyzer.set_rate(engine.stats().format.sample_rate);
        }
        self.analyzer.update(&self.position, dt);
        self.now_playing = engine.track_info(self.position.track).cloned();
        // Drive the analyzer with the audible position and pick up its latest score.
        self.score = None;
        if self.position.state != PlayState::Stopped
            && let (Some(a), Some(track)) =
                (&self.analysis, self.track_refs.get(&self.position.track))
        {
            a.playhead(track, self.position.seconds());
            self.score = a.score(track);
        }
        // The overview is built only once the analyzer has its first results, so it never
        // competes with playback start or the playhead analysis.
        self.overview = None;
        if self.position.state != PlayState::Stopped
            && let (Some(ov), Some(track)) =
                (&self.overviews, self.track_refs.get(&self.position.track))
        {
            if self.score.is_some() {
                ov.request(track);
            }
            self.overview = ov.get(track);
        }
        // A scheduled jump has happened once its landing point is audible.
        if let Some(at) = self.nav.pending_jump
            && (self.position.discontinuity || self.position.seconds() >= at + 0.25)
        {
            self.nav.pending_jump = None;
            self.nav.jump_target = None;
        }
        if self.position.state != PlayState::Stopped
            && let Some(&id) = engine.current_index().and_then(|q| queue.get(q))
            && let Some(p) = self.crates.get_mut(queue_crate)
        {
            p.set_current(Some(id));
        }
    }

    // ---- entries waiting for their audio ---------------------------------------------------

    /// An audio producer delivered a waiting entry's file: it becomes playable, joins the
    /// queue if its crate is playing, and starts at once if it was armed.
    pub fn set_audio(&mut self, crate_id: CrateId, id: EntryId, track: TrackRef) {
        let Some(p) = self.crates.get_mut(crate_id) else {
            return;
        };
        if !p.set_audio(id, track.clone()) {
            return;
        }
        if let Some(m) = &self.meta {
            m.request(vec![((crate_id, id), track)]);
        }
        self.mark_crate(crate_id);
        self.check_armed();
    }

    /// Starts the armed entry once it can play, and forgets it if it never will.
    fn check_armed(&mut self) {
        let Some((c, id)) = self.armed else {
            return;
        };
        let status = self
            .crates
            .get(c)
            .and_then(|p| p.get(id))
            .map(|e| &e.status);
        let (waiting, playable) = (
            matches!(status, Some(EntryStatus::Waiting(_))),
            status.is_some_and(EntryStatus::is_playable),
        );
        if playable && matches!(self.engine, EngineSlot::Ready(_)) {
            self.play_entry(c, id);
        } else if !waiting && !playable {
            self.armed = None; // removed, or it will never play
        }
    }

    /// "Waiting for ‹title› (downloading 40%)" while an entry is armed.
    fn armed_line(&self) -> Option<String> {
        let (c, id) = self.armed?;
        let e = self.crates.get(c)?.get(id)?;
        Some(match e.status.note() {
            Some(note) => format!("Waiting for {} ({note})", e.display_name()),
            None => format!("Waiting for {}", e.display_name()),
        })
    }

    // ---- files -----------------------------------------------------------------------------

    /// Adds music to the shown crate (ADD, drag-and-drop, Cmd+O, M3U import), or replaces the
    /// Playlist crate (Eject). Adding to an empty crate plays it if nothing else is playing.
    fn add_paths(&mut self, paths: Vec<PathBuf>, open: Open) {
        let found: Vec<TrackRef> = files::expand(&paths)
            .into_iter()
            .map(|p| TrackRef::new(p.to_string_lossy()))
            .collect();
        if found.is_empty() {
            return;
        }
        let (crate_id, added, play) = if open == Open::Replace {
            let added = self.crates.replace_playlist(found);
            self.queue_dirty = true;
            (PLAYLIST, added, true)
        } else {
            let crate_id = self.crates.shown_id();
            let was_empty = self.crates.shown().is_empty();
            let added = self.crates.shown_mut().add(found);
            let added: Vec<_> = added.into_iter().map(|(e, t)| ((crate_id, e), t)).collect();
            self.mark_shown();
            let idle = self.position.state == PlayState::Stopped;
            (
                crate_id,
                added,
                open == Open::AddAndPlay || (was_empty && idle),
            )
        };
        let first = added.first().map(|((_, id), _)| *id);
        if let Some(m) = &self.meta {
            m.request(added);
        }
        if play && let Some(id) = first {
            self.play_entry(crate_id, id);
        }
    }

    /// Opens the "Render show" dialog for a playlist entry.
    fn open_render_dialog(&mut self, id: EntryId) {
        let (Some(renderer), Some(e)) = (&self.show_renderer, self.crates.shown().get(id)) else {
            return;
        };
        self.render_looks = renderer.looks();
        let s = &self.settings;
        self.render_dialog = Some(crate::render_job::RenderDialog::new(
            e.track.clone(),
            e.display_name(),
            e.duration,
            s.render_size,
            s.render_fps,
            s.render_overlay,
        ));
    }

    /// Shows the dialog; on "Render…" asks where to save and starts the background render.
    fn render_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.render_dialog else {
            return;
        };
        let looks = &self.render_looks;
        let modal = egui::Modal::new(Id::new("render-dialog")).show(ctx, |ui| {
            ui.set_max_width(360.0);
            dialog.ui(ui, looks)
        });
        let outcome = if modal.should_close() {
            crate::render_job::DialogOutcome::Cancel
        } else {
            modal.inner
        };
        match outcome {
            crate::render_job::DialogOutcome::Open => {}
            crate::render_job::DialogOutcome::Cancel => self.render_dialog = None,
            crate::render_job::DialogOutcome::Render => {
                let name: String = dialog
                    .name
                    .chars()
                    .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
                    .collect();
                let Some(out) = rfd::FileDialog::new()
                    .add_filter("MP4 video", &["mp4"])
                    .set_file_name(format!("{name}.mp4"))
                    .save_file()
                else {
                    return; // back to the dialog
                };
                let request = match dialog.request(out.clone(), looks) {
                    Ok(r) => r,
                    Err(e) => {
                        dialog.error = Some(e);
                        return;
                    }
                };
                let Some(renderer) = &self.show_renderer else {
                    return;
                };
                let label = out
                    .file_name()
                    .map_or_else(|| name.clone(), |n| n.to_string_lossy().into_owned());
                let (size, fps, overlay) = (request.size, request.fps, request.overlay);
                self.render_job = Some((renderer.start(request), label));
                self.render_dialog = None;
                self.settings.render_size = size;
                self.settings.render_fps = fps;
                self.settings.render_overlay = overlay;
                self.mark_settings();
            }
        }
    }

    fn open_files_dialog(&mut self, open: Open) {
        let exts: Vec<&str> = files::AUDIO_EXTENSIONS
            .iter()
            .copied()
            .chain(["m3u", "m3u8"])
            .collect();
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Audio", &exts)
            .pick_files()
        {
            self.add_paths(paths, open);
        }
    }

    fn open_folder_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.add_paths(vec![dir], Open::Add);
        }
    }

    fn export_m3u(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Playlist", &["m3u8", "m3u"])
            .set_file_name(format!("{}.m3u8", self.crates.name(self.crates.shown_id())))
            .save_file()
        else {
            return;
        };
        let entries = files::m3u_entries(self.crates.shown());
        match std::fs::write(&path, files::write_m3u(&entries)) {
            Ok(()) => self.notify(format!("Saved {}", path.display())),
            Err(e) => self.notify(format!("Could not save: {e}")),
        }
    }

    // ---- keyboard ----------------------------------------------------------------------------

    fn handle_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (pressed, mods): (Vec<Key>, Modifiers) = ctx.input(|i| {
            let keys = i
                .events
                .iter()
                .filter_map(|e| match e {
                    // Brackets by position (right of P), so they work on any keyboard layout.
                    egui::Event::Key {
                        key,
                        physical_key,
                        pressed: true,
                        ..
                    } => Some(match physical_key {
                        Some(k @ (Key::OpenBracket | Key::CloseBracket)) => *k,
                        _ => *key,
                    }),
                    _ => None,
                })
                .collect();
            (keys, i.modifiers)
        });
        for key in pressed {
            // The help panel takes its own close keys first (Esc must not leave fullscreen).
            if matches!(key, Key::H | Key::F1) && !mods.command {
                self.help = !self.help;
                continue;
            }
            if self.help && key == Key::Escape {
                self.help = false;
                continue;
            }
            if !self.transport_key(ctx, key, mods) {
                if self.fullscreen.is_some() {
                    if !self.host_key(key) {
                        self.scene.key(key, mods);
                    }
                } else {
                    self.window_key(key, mods);
                }
            }
        }
    }

    /// Keys the fullscreen host keeps for itself (see [`host_action`]); others go to the scene.
    fn host_key(&mut self, key: Key) -> bool {
        let Some(action) = host_action(key, self.annotating.is_some()) else {
            return false;
        };
        let t = self.position.seconds();
        match action {
            HostAction::ToggleStrip => self.strip = !self.strip,
            HostAction::ToggleAnnotating => self.toggle_annotating(),
            HostAction::Tap => {
                if let Some(a) = &mut self.annotating {
                    a.tap(t);
                }
                self.save_annotations();
            }
            HostAction::Boundary(kind) => {
                if let Some(a) = &mut self.annotating {
                    a.boundary(t, kind);
                }
                self.save_annotations();
            }
        }
        true
    }

    fn toggle_annotating(&mut self) {
        if self.annotating.take().is_some() {
            return; // every mark is saved as it is made
        }
        let Some(track) = self.track_refs.get(&self.position.track).cloned() else {
            self.notify("Play a track to annotate it");
            return;
        };
        let Some(dir) = self.annotations_dir.clone() else {
            self.notify("Annotations need a cache directory");
            return;
        };
        let hash = self
            .score
            .as_ref()
            .map(|s| s.content_hash)
            .filter(|&h| h != 0)
            .or_else(|| analysis::cache::content_hash(&*self.files, &track))
            .unwrap_or(0);
        self.annotating = Some(analysis::eval::Annotations::load_or_new(&dir, &track, hash));
        self.strip = true;
    }

    fn save_annotations(&mut self) {
        if let (Some(ann), Some(dir)) = (&self.annotating, &self.annotations_dir)
            && let Err(e) = ann.save(dir)
        {
            self.message = Some((format!("Could not save annotations: {e}"), Instant::now()));
        }
    }

    /// Keys that work in both windowed and fullscreen mode.
    fn transport_key(&mut self, ctx: &egui::Context, key: Key, mods: Modifiers) -> bool {
        if mods.command {
            return false;
        }
        let secs = self.position.seconds();
        match key {
            Key::Z => self.with_engine(|e| e.previous()),
            Key::X => self.play(),
            Key::C => self.with_engine(|e| e.toggle_pause()),
            Key::V => self.with_engine(|e| e.stop()),
            Key::B => self.with_engine(|e| e.next()),
            Key::ArrowLeft => self.with_engine(|e| e.seek((secs - 5.0).max(0.0))),
            Key::ArrowRight => self.with_engine(|e| e.seek(secs + 5.0)),
            Key::ArrowUp => self.set_volume(self.settings.volume + 0.05),
            Key::ArrowDown => self.set_volume(self.settings.volume - 0.05),
            Key::F => self.toggle_fullscreen(ctx),
            Key::Escape if self.fullscreen.is_some() => self.toggle_fullscreen(ctx),
            Key::CloseBracket if mods.shift => self.structure_jump(crate::navigation::next_rise),
            Key::CloseBracket => self.structure_jump(crate::navigation::next_section),
            Key::OpenBracket => self.structure_jump(crate::navigation::previous_section),
            Key::L if mods.shift => self.bar_loop(),
            Key::L => self.section_loop(),
            _ => return false,
        }
        true
    }

    fn window_key(&mut self, key: Key, mods: Modifiers) {
        match key {
            Key::W if !mods.command => {
                self.settings.show_waveform = !self.settings.show_waveform;
                self.mark_settings();
            }
            Key::S if !mods.command => self.spectro_open = !self.spectro_open,
            Key::Delete | Key::Backspace => self.remove_selected(),
            Key::O if mods.command => self.open_files_dialog(Open::Add),
            Key::A if mods.command => self.crates.shown_mut().select_all(),
            Key::Enter => {
                if let Some(&id) = self.crates.shown().selected_ids().first() {
                    self.play_entry(self.crates.shown_id(), id);
                }
            }
            _ => {}
        }
    }

    fn remove_selected(&mut self) {
        if self.crates.shown_mut().remove_selected() > 0 {
            self.mark_shown();
        }
    }

    fn set_volume(&mut self, v: f32) {
        self.settings.volume = v.clamp(0.0, 1.0);
        let v = self.settings.volume;
        self.with_engine(|e| e.set_volume(v));
        self.mark_settings();
    }

    // ---- fullscreen ------------------------------------------------------------------------

    fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        match self.fullscreen.take() {
            Some(fs) => {
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(ViewportCommand::CursorVisible(true));
                if let Some(r) = fs.restore {
                    ctx.send_viewport_cmd(ViewportCommand::InnerSize(Self::window_size(
                        &self.settings,
                        &self.skin,
                    )));
                    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(r.min));
                }
                self.last_size = None;
            }
            None => {
                if !self.scene_ready
                    && let Some(rs) = &self.render_state
                {
                    self.scene.init(rs);
                    self.scene_ready = true;
                }
                self.scene.entered();
                let restore = ctx.input(|i| i.viewport().outer_rect);
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
                self.fullscreen = Some(Fullscreen {
                    restore,
                    last_pointer: Instant::now(),
                    pointer_at: None,
                });
            }
        }
    }

    fn fullscreen_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
        let (artist, title, duration) = self
            .now_playing_names()
            .unwrap_or_else(|| (String::new(), "Nothing playing".into(), None));
        let bars = *self.analyzer.bars();
        let frame = SceneFrame {
            position: self.position,
            bars: &bars,
            artist: &artist,
            title: &title,
            score: self.score.as_deref(),
            duration,
            strip_visible: self.strip || self.annotating.is_some(),
        };
        if let Some(cb) = self.scene.paint(rect, &frame) {
            ui.painter().add(cb);
        }
        self.scene.ui(ui, &frame);
        if self.strip || self.annotating.is_some() {
            crate::timeline::draw(
                ui,
                rect,
                &self.position,
                self.score.as_deref(),
                self.annotating.as_ref(),
            );
        }

        // Hide the cursor after 2 s without movement.
        if let Some(fs) = &mut self.fullscreen {
            let pointer = ctx.input(|i| i.pointer.latest_pos());
            if pointer != fs.pointer_at {
                fs.pointer_at = pointer;
                fs.last_pointer = Instant::now();
                ctx.send_viewport_cmd(ViewportCommand::CursorVisible(true));
            } else if fs.last_pointer.elapsed() > Duration::from_secs(2) {
                ctx.send_viewport_cmd(ViewportCommand::CursorVisible(false));
            }
        }
        ctx.request_repaint(); // one frame per vsync
    }

    // ---- sections ----------------------------------------------------------------------------

    /// A painter for a section; borrows only the (shared) skin definition, not the app.
    fn skinned<'a>(&self, def: &'a crate::skin::SkinDef, ui: &Ui, origin: Pos2) -> Skinned<'a> {
        Skinned {
            painter: ui.painter().clone(),
            tex: self.tex.as_ref().expect("texture loaded").id(),
            def,
            origin,
            scale: self.settings.scale as f32,
        }
    }

    fn titlebar_drag(ui: &mut Ui, sk: &Skinned, id: &str, layout: &str) {
        let resp = ui.interact(sk.at(layout), Id::new(id), Sense::click_and_drag());
        if resp.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
    }

    fn main_section(&mut self, ui: &mut Ui, origin: Pos2) {
        let state = self.position.state;
        let info = self.now_playing.clone();
        let duration = info.as_ref().and_then(|i| i.duration_secs);
        let elapsed = self.position.seconds();
        let mut actions: Vec<Action> = Vec::new();
        {
            let def = self.def.clone();
            let sk = self.skinned(&def, ui, origin);
            sk.sprite("main_bg", 0.0, 0.0);
            Self::titlebar_drag(ui, &sk, "main_title", "titlebar");
            if widgets::button(ui, &sk, "min", "btn_min", "btn_min").clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
            }
            if widgets::button(ui, &sk, "close", "btn_close", "btn_close").clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }

            // LCD: status, time, visualizer.
            let st = sk.def.at("status");
            let status = match state {
                PlayState::Playing => "status_play",
                PlayState::Paused => "status_pause",
                PlayState::Stopped => "status_stop",
            };
            sk.sprite(status, st.x as f32, st.y as f32);
            let t = sk.def.at("time");
            let show_time = state != PlayState::Stopped;
            let remaining = self.settings.time_remaining && duration.is_some();
            let secs = if remaining {
                (duration.unwrap_or(0.0) - elapsed).max(0.0)
            } else {
                elapsed
            };
            let (mm, ss) = format::lcd(secs);
            let (tx, ty) = (t.x as f32, t.y as f32);
            sk.sprite(
                if show_time && remaining {
                    "digit_minus"
                } else {
                    "digit_blank"
                },
                tx,
                ty,
            );
            let digit = |c: char| {
                if show_time {
                    format!("digit_{c}")
                } else {
                    "digit_blank".to_owned()
                }
            };
            let mins: Vec<char> = mm.chars().collect();
            for (i, c) in mins.iter().rev().take(3).enumerate() {
                sk.sprite(&digit(*c), tx + 21.0 - 10.0 * i as f32, ty);
            }
            if show_time {
                sk.sprite("digit_colon", tx + 31.0, ty);
            }
            for (i, c) in ss.chars().enumerate() {
                sk.sprite(&digit(c), tx + 35.0 + 10.0 * i as f32, ty);
            }
            if ui
                .interact(sk.at("time"), Id::new("time"), Sense::click())
                .clicked()
            {
                actions.push(Action::ToggleRemaining);
            }
            self.draw_vis(&sk);
            if ui
                .interact(sk.at("vis"), Id::new("vis"), Sense::click())
                .clicked()
            {
                actions.push(Action::CycleVis);
            }

            // Title, kbps, kHz, mono/stereo.
            let tt = sk.def.at("title_text");
            let number = self.crates.playing().current_index().map_or(0, |i| i + 1);
            let title = match self.now_playing_names() {
                Some((artist, title, duration)) => {
                    format::title_line(number, &artist, &title, duration)
                }
                None => engine_status(&self.engine),
            };
            let width = (tt.w / sk.def.font.advance) as usize;
            let shown = format::scroll(&title, width, self.title_offset);
            let lcd = color([0, 236, 0]);
            sk.text(tt.x as f32, tt.y as f32, &shown, lcd);
            if let Some(i) = &info {
                let k = sk.def.at("kbps");
                if let Some(kbps) = i.bitrate_kbps {
                    let s = kbps.min(999).to_string();
                    sk.text(
                        k.x as f32 + k.w as f32 - sk.text_width(&s),
                        k.y as f32,
                        &s,
                        lcd,
                    );
                }
                let k = sk.def.at("khz");
                let s = ((i.sample_rate as f32 / 1000.0).round() as u32).to_string();
                sk.text(
                    k.x as f32 + k.w as f32 - sk.text_width(&s),
                    k.y as f32,
                    &s,
                    lcd,
                );
            }
            let channels = info.as_ref().map_or(0, |i| i.channels);
            let m = sk.def.at("mono");
            sk.sprite(
                if channels == 1 { "mono_on" } else { "mono_off" },
                m.x as f32,
                m.y as f32,
            );
            let s = sk.def.at("stereo");
            sk.sprite(
                if channels >= 2 {
                    "stereo_on"
                } else {
                    "stereo_off"
                },
                s.x as f32,
                s.y as f32,
            );

            // Sliders.
            let vol = SliderSprites {
                track: "volume_track",
                fill: Some("volume_fill"),
                thumb: "volume_thumb",
            };
            if let (_, Some(v)) =
                widgets::hslider(ui, &sk, "volume", "volume", vol, self.settings.volume)
            {
                actions.push(Action::Volume(v));
            }
            let bal = SliderSprites {
                track: "balance_track",
                fill: None,
                thumb: "balance_thumb",
            };
            if let (_, Some(v)) = widgets::hslider(
                ui,
                &sk,
                "balance",
                "balance",
                bal,
                (self.settings.balance + 1.0) / 2.0,
            ) {
                let b = v * 2.0 - 1.0;
                actions.push(Action::Balance(if b.abs() < 0.08 { 0.0 } else { b }));
            }
            if widgets::toggle(
                ui,
                &sk,
                "eq_tog",
                "eq_toggle",
                "tog_eq",
                self.settings.show_eq,
            )
            .clicked()
            {
                actions.push(Action::ToggleEq);
            }
            if widgets::toggle(
                ui,
                &sk,
                "pl_tog",
                "pl_toggle",
                "tog_pl",
                self.settings.show_playlist,
            )
            .clicked()
            {
                actions.push(Action::TogglePlaylist);
            }
            let r = sk.def.at("seek");
            if let Some(d) = duration.filter(|d| *d > 0.0 && state != PlayState::Stopped) {
                let value = self.seek_drag.unwrap_or((elapsed / d) as f32);
                let seek = SliderSprites {
                    track: "seek_track",
                    fill: None,
                    thumb: "seek_thumb",
                };
                let (resp, new) = widgets::hslider(ui, &sk, "seek", "seek", seek, value);
                if let Some(v) = new {
                    self.seek_drag = Some(v);
                }
                if (resp.drag_stopped() || (resp.clicked() && new.is_some()))
                    && let Some(v) = self.seek_drag.take()
                {
                    actions.push(Action::Seek(v as f64 * d));
                }
            } else {
                sk.sprite("seek_track", r.x as f32, r.y as f32);
            }

            // Transport.
            for (name, action) in [
                ("prev", Action::Prev),
                ("play", Action::Play),
                ("pause", Action::Pause),
                ("stop", Action::Stop),
                ("next", Action::Next),
                ("eject", Action::Eject),
            ] {
                if widgets::button(ui, &sk, name, name, name).clicked() {
                    actions.push(action);
                }
            }
            let shuffle = if self.settings.shuffle {
                "shuffle_on"
            } else {
                "shuffle_off"
            };
            if widgets::button(ui, &sk, "shuffle", "shuffle", shuffle).clicked() {
                actions.push(Action::Shuffle);
            }
            let repeat = match self.settings.repeat {
                Repeat::Off => "repeat_off",
                Repeat::All => "repeat_all",
                Repeat::One => "repeat_one",
            };
            if widgets::button(ui, &sk, "repeat", "repeat", repeat).clicked() {
                actions.push(Action::Repeat);
            }
        }
        for a in actions {
            self.apply(a, ui.ctx());
        }
    }

    /// Artist, title and duration of what plays: the file's tags, except that an entry with
    /// an origin keeps its own artist and title (the record, not the file, is the truth).
    fn now_playing_names(&self) -> Option<(String, String, Option<f64>)> {
        let playing = self.crates.playing();
        let entry = playing.current().and_then(|id| playing.get(id));
        match (&self.now_playing, entry) {
            (Some(i), Some(e)) if e.origin.is_some() => Some((
                e.artist.clone(),
                e.title.clone(),
                i.duration_secs.or(e.duration),
            )),
            (Some(i), _) => Some((i.artist.clone(), i.title.clone(), i.duration_secs)),
            (None, Some(e)) => Some((e.artist.clone(), e.title.clone(), e.duration)),
            (None, None) => None,
        }
    }

    fn draw_vis(&self, sk: &Skinned) {
        let v = sk.def.at("vis");
        let c = &sk.def.colors;
        match self.settings.vis {
            VisMode::Off => {}
            VisMode::Spectrum => {
                let bars = self.analyzer.bars();
                let peaks = self.analyzer.peaks();
                let h = v.h as f32;
                for i in 0..BARS {
                    let x = v.x as f32 + i as f32 * 4.0;
                    let bh = (bars[i] * h).round();
                    for row in 0..bh as u32 {
                        let t = row as f32 / (h - 1.0);
                        let col = lerp_color(c.vis_bar_low, c.vis_bar_high, t);
                        sk.fill(sk.rect(x, v.y as f32 + h - 1.0 - row as f32, 3.0, 1.0), col);
                    }
                    let py = (peaks[i] * h).round();
                    if py >= 1.0 {
                        sk.fill(sk.rect(x, v.y as f32 + h - py, 3.0, 1.0), color(c.vis_peak));
                    }
                }
            }
            VisMode::Scope => {
                let samples = self.analyzer.scope(&self.position, v.w as usize);
                let mid = v.y as f32 + v.h as f32 / 2.0;
                for (i, s) in samples.iter().enumerate() {
                    let y = (mid + s.clamp(-1.0, 1.0) * (v.h as f32 / 2.0 - 1.0)).round();
                    sk.fill(
                        sk.rect(v.x as f32 + i as f32, y, 1.0, 1.0),
                        color(c.vis_scope),
                    );
                }
            }
        }
    }

    fn eq_section(&mut self, ui: &mut Ui, origin: Pos2) {
        let mut eq = self.settings.eq;
        let mut changed = false;
        let mut actions = Vec::new();
        {
            let def = self.def.clone();
            let sk = self.skinned(&def, ui, origin);
            sk.sprite("eq_bg", 0.0, 0.0);
            Self::titlebar_drag(ui, &sk, "eq_title", "eq_titlebar");
            if widgets::button(ui, &sk, "eq_close", "eq_close", "btn_close").clicked() {
                actions.push(Action::ToggleEq);
            }
            if widgets::toggle(ui, &sk, "eq_on", "eq_on", "eq_on", eq.enabled).clicked() {
                eq.enabled = !eq.enabled;
                changed = true;
            }
            let presets_resp = widgets::button(ui, &sk, "eq_presets", "eq_presets", "eq_presets");
            egui::Popup::menu(&presets_resp).show(|ui| {
                ui.label("Load");
                for p in &self.presets.presets {
                    if ui.button(&p.name).clicked() {
                        eq = p.apply_to(&eq);
                        changed = true;
                    }
                }
                ui.separator();
                if ui.button("Save as…").clicked() {
                    actions.push(Action::SavePreset);
                }
                ui.menu_button("Delete", |ui| {
                    for p in &self.presets.presets {
                        if ui.button(&p.name).clicked() {
                            actions.push(Action::DeletePreset(p.name.clone()));
                        }
                    }
                });
            });

            // Response curve.
            let g = sk.def.at("eq_graph");
            let pts = eqcurve::curve(&eq.bands_db, (g.w - 2) as usize);
            let cx = sk.def.colors.eq_curve;
            for (i, db) in pts.iter().enumerate() {
                let y = g.y as f32 + 1.0 + ((12.0 - db) / 24.0 * (g.h - 3) as f32).round();
                sk.fill(sk.rect(g.x as f32 + 1.0 + i as f32, y, 1.0, 1.0), color(cx));
            }

            // Sliders (double-click resets to 0 dB).
            let slider = |ui: &mut Ui, id: &str, layout: &str, db: &mut f32| {
                let (resp, new) = widgets::vslider(
                    ui,
                    &sk,
                    id,
                    layout,
                    "eq_track",
                    "eq_thumb",
                    (*db + 12.0) / 24.0,
                );
                if resp.double_clicked() {
                    *db = 0.0;
                    return true;
                }
                if let Some(v) = new {
                    *db = (v * 24.0 - 12.0).round().clamp(-12.0, 12.0);
                    return true;
                }
                false
            };
            changed |= slider(ui, "eq_pre", "eq_preamp", &mut eq.preamp_db);
            for i in 0..10 {
                changed |= slider(
                    ui,
                    &format!("eq_b{i}"),
                    &format!("eq_band{i}"),
                    &mut eq.bands_db[i],
                );
            }
        }
        if changed && eq != self.settings.eq {
            self.settings.eq = eq;
            self.with_engine(|e| e.set_eq(eq));
            self.mark_settings();
        }
        for a in actions {
            self.apply(a, ui.ctx());
        }
    }

    fn playlist_section(&mut self, ui: &mut Ui, origin: Pos2) {
        let rows = self.settings.playlist_rows as usize;
        let d = self.skin.def.clone();
        let list_h = (rows * d.pl_row_h as usize) as f32;
        let mut actions = Vec::new();
        let len = self.crates.shown().len();
        self.pl_scroll = self.pl_scroll.min(len.saturating_sub(rows));
        {
            let def = self.def.clone();
            let sk = self.skinned(&def, ui, origin);
            let scale = sk.scale;
            sk.sprite("pl_top", 0.0, 0.0);
            // The title bar names the shown crate: a click opens the crate menu, a drag still
            // moves the window.
            let title = ui.interact(
                sk.at("pl_titlebar"),
                Id::new("pl_title"),
                Sense::click_and_drag(),
            );
            if title.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }
            draw_crate_name(&sk, self.crates.name(self.crates.shown_id()));
            egui::Popup::menu(&title)
                .id(Id::new("crate_menu"))
                .show(|ui| self.crate_menu(ui, &mut actions));
            if widgets::button(ui, &sk, "pl_close", "pl_close", "btn_close").clicked() {
                actions.push(Action::TogglePlaylist);
            }
            let top = d.pl_top_h as f32;
            sk.sprite_in("pl_left", sk.rect(0.0, top, 12.0, list_h));
            sk.sprite_in(
                "pl_right",
                sk.rect(d.pl_width as f32 - 20.0, top, 20.0, list_h),
            );
            let l = d.at("pl_list");
            let list = sk.rect(l.x as f32, l.y as f32, l.w as f32, list_h);
            sk.fill(list, color(d.colors.pl_bg));

            // Rows.
            let font = egui::FontId::proportional(9.5 * scale);
            let clip = sk.painter.with_clip_rect(list);
            let row_h = d.pl_row_h as f32;
            let list_resp = ui.interact(list, Id::new("pl_list"), Sense::hover());
            let mods = ui.input(|i| ClickMods {
                shift: i.modifiers.shift,
                command: i.modifiers.command,
            });
            let mut drop_target = None;
            let shown = self.crates.shown();
            for r in 0..rows {
                let idx = self.pl_scroll + r;
                let Some(e) = shown.entries().get(idx) else {
                    break;
                };
                let rr = sk.rect(l.x as f32, top + r as f32 * row_h, l.w as f32, row_h);
                if shown.is_selected(e.id) {
                    clip.rect_filled(rr, 0.0, color(d.colors.pl_selected_bg));
                }
                let current = shown.current() == Some(e.id);
                let (col, dur) = row_look(e, current, &d.colors);
                let dur_w = clip
                    .text(
                        pos2(rr.right() - 3.0 * scale, rr.center().y),
                        egui::Align2::RIGHT_CENTER,
                        &dur,
                        font.clone(),
                        col,
                    )
                    .width();
                let name_clip = clip.with_clip_rect(Rect::from_min_max(
                    rr.min,
                    pos2(rr.right() - dur_w - 8.0 * scale, rr.max.y),
                ));
                name_clip.text(
                    pos2(rr.left() + 3.0 * scale, rr.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{}. {}", idx + 1, e.display_name()),
                    font.clone(),
                    col,
                );
                let resp = ui.interact(rr, Id::new(("pl_row", idx)), Sense::click_and_drag());
                let menu_click = opens_context_menu(
                    resp.secondary_clicked(),
                    resp.clicked(),
                    ui.input(|i| i.modifiers),
                );
                if resp.double_clicked() {
                    actions.push(Action::PlayEntry(e.id));
                } else if resp.clicked() && !menu_click {
                    actions.push(Action::Select(idx, mods));
                }
                let rendering = self
                    .render_job
                    .as_ref()
                    .is_some_and(|(j, _)| !j.status().finished());
                // egui's context menu opens only on the secondary button; on a Mac,
                // Control-click is the usual right-click too.
                let open = if menu_click {
                    Some(egui::SetOpenCommand::Bool(true))
                } else if resp.clicked() {
                    Some(egui::SetOpenCommand::Bool(false))
                } else {
                    None
                };
                egui::Popup::context_menu(&resp)
                    .open_memory(open)
                    .show(|ui| {
                        ui.menu_button("Send to crate", |ui| {
                            for c in self.crates.list() {
                                if c.id == self.crates.shown_id() {
                                    continue;
                                }
                                let readable = !self.crates.is_unreadable(c.id);
                                if ui
                                    .add_enabled(readable, egui::Button::new(&c.name))
                                    .clicked()
                                {
                                    actions.push(Action::SendTo(e.id, Some(c.id)));
                                    ui.close();
                                }
                            }
                            ui.separator();
                            if ui.button("New crate…").clicked() {
                                actions.push(Action::SendTo(e.id, None));
                                ui.close();
                            }
                        });
                        if self.show_renderer.is_some() {
                            let (label, action) = if rendering {
                                ("Cancel show render", Action::CancelRender)
                            } else {
                                ("Render show…", Action::RenderShow(e.id))
                            };
                            if ui.button(label).clicked() {
                                actions.push(action);
                                ui.close();
                            }
                        }
                    });
                if resp.drag_started() {
                    self.pl_drag_from = Some(idx);
                }
                if self.pl_drag_from.is_some() && ui.rect_contains_pointer(rr) {
                    drop_target = Some(idx);
                    clip.rect_filled(
                        Rect::from_min_size(rr.min, vec2(rr.width(), scale)),
                        0.0,
                        Color32::WHITE,
                    );
                }
            }
            if let Some(from) = self.pl_drag_from
                && ui.input(|i| i.pointer.any_released())
            {
                if let Some(to) = drop_target {
                    actions.push(Action::Move(from, to));
                }
                self.pl_drag_from = None;
            }
            if list_resp.hovered() {
                let dy = ui.input(|i| i.smooth_scroll_delta.y);
                if dy != 0.0 {
                    let step = (dy / (row_h * scale)).round() as isize;
                    let s = (self.pl_scroll as isize - step)
                        .clamp(0, len.saturating_sub(rows) as isize);
                    self.pl_scroll = s as usize;
                }
            }

            // Scrollbar.
            let sc = d.at("pl_scroll");
            let thumb = d.sprite("pl_scroll_thumb");
            let travel = (list_h - thumb.h as f32).max(0.0);
            let max_scroll = len.saturating_sub(rows);
            let frac = if max_scroll == 0 {
                0.0
            } else {
                self.pl_scroll as f32 / max_scroll as f32
            };
            let sc_rect = sk.rect(sc.x as f32, top, sc.w as f32, list_h);
            let sresp = ui.interact(sc_rect, Id::new("pl_scroll"), Sense::click_and_drag());
            if let Some(p) = sresp.interact_pointer_pos() {
                let f = (((p.y - sc_rect.top()) / scale - thumb.h as f32 / 2.0) / travel)
                    .clamp(0.0, 1.0);
                self.pl_scroll = (f * max_scroll as f32).round() as usize;
            }
            sk.sprite(
                "pl_scroll_thumb",
                sc.x as f32,
                top + (frac * travel).round(),
            );

            // Bottom bar.
            let bottom_y = top + list_h;
            let bsk = Skinned {
                origin: sk.origin + vec2(0.0, bottom_y * scale),
                ..self.skinned(&def, ui, origin)
            };
            bsk.sprite("pl_bottom", 0.0, 0.0);
            let add = widgets::button(ui, &bsk, "pl_add", "pl_add", "pl_add");
            egui::Popup::menu(&add).show(|ui| {
                if ui.button("Add files…").clicked() {
                    actions.push(Action::AddFiles);
                }
                if ui.button("Add folder…").clicked() {
                    actions.push(Action::AddFolder);
                }
            });
            let rem = widgets::button(ui, &bsk, "pl_rem", "pl_rem", "pl_rem");
            egui::Popup::menu(&rem).show(|ui| {
                if ui.button("Remove selected").clicked() {
                    actions.push(Action::RemoveSelected);
                }
                if ui.button("Clear crate").clicked() {
                    actions.push(Action::Clear);
                }
            });
            let sel = widgets::button(ui, &bsk, "pl_sel", "pl_sel", "pl_sel");
            egui::Popup::menu(&sel).show(|ui| {
                if ui.button("Select all").clicked() {
                    actions.push(Action::SelectAll);
                }
                if ui.button("Select none").clicked() {
                    actions.push(Action::SelectNone);
                }
                if ui.button("Invert selection").clicked() {
                    actions.push(Action::InvertSelection);
                }
            });
            let misc = widgets::button(ui, &bsk, "pl_misc", "pl_misc", "pl_misc");
            egui::Popup::menu(&misc).show(|ui| {
                if ui.button("Import M3U…").clicked() {
                    actions.push(Action::AddFiles);
                }
                if ui.button("Export M3U…").clicked() {
                    actions.push(Action::ExportM3u);
                }
            });
            let opts = widgets::button(ui, &bsk, "pl_opts", "pl_opts", "pl_opts");
            egui::Popup::menu(&opts).show(|ui| {
                let label = if self.settings.scale >= 2 {
                    "Classic size (1×)"
                } else {
                    "Double size (2×)"
                };
                if ui.button(label).clicked() {
                    actions.push(Action::ToggleScale);
                }
                if ui.button("Spectrogram (S)").clicked() {
                    actions.push(Action::ToggleSpectrogram);
                }
            });

            // "selected/total" time, Winamp style.
            let (total, t_unknown) = self.crates.shown().total_duration();
            let (seltime, s_unknown) = self.crates.shown().selected_duration();
            let info = format!(
                "{}{}/{}{}",
                format::clock(seltime),
                if s_unknown { "+" } else { "" },
                format::clock(total),
                if t_unknown { "+" } else { "" }
            );
            let pi = d.at("pl_info");
            bsk.text(pi.x as f32, pi.y as f32, &info, color([0, 236, 0]));

            // Resize handle changes the number of visible rows.
            let rz = bsk.at("pl_resize");
            bsk.sprite(
                "pl_resize",
                d.at("pl_resize").x as f32,
                d.at("pl_resize").y as f32,
            );
            let (_, delta) = widgets::drag_area(ui, &bsk, "pl_resize", rz);
            self.pl_resize_acc += delta.y;
            let step = (self.pl_resize_acc / row_h).trunc();
            if step != 0.0 {
                self.pl_resize_acc -= step * row_h;
                actions.push(Action::Rows(step as i32));
            }
        }
        for a in actions {
            self.apply(a, ui.ctx());
        }
    }

    /// The name dialog: EQ preset names, and new or renamed crates. A refused name keeps the
    /// dialog open with the reason.
    fn name_dialog(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.name_dialog else {
            return;
        };
        let (title, verb) = dialog.purpose.labels();
        let mut done = None;
        egui::Window::new(title)
            .id(Id::new("name-dialog"))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let r = ui.text_edit_singleline(&mut dialog.text);
                // Enter makes the field lose focus; check that before taking focus back.
                let entered = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                r.request_focus();
                if let Some(e) = &dialog.error {
                    ui.colored_label(Color32::from_rgb(230, 90, 90), e);
                }
                ui.horizontal(|ui| {
                    if ui.button(verb).clicked() || entered {
                        done = Some(true);
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        done = Some(false);
                    }
                });
            });
        match done {
            Some(true) => {
                let Some(dialog) = self.name_dialog.take() else {
                    return;
                };
                if let Err(e) = self.apply_name(&dialog.purpose, &dialog.text) {
                    self.name_dialog = Some(NameDialog {
                        error: Some(e),
                        ..dialog
                    });
                }
            }
            Some(false) => self.name_dialog = None,
            None => {}
        }
    }

    fn apply_name(&mut self, purpose: &NameFor, name: &str) -> Result<(), String> {
        match purpose {
            NameFor::Preset => {
                if !name.trim().is_empty() {
                    self.presets
                        .save(EqPreset::from_settings(name.trim(), &self.settings.eq));
                    self.save_presets();
                }
            }
            NameFor::NewCrate => {
                let id = self.crates.create(name)?;
                self.show_crate(id);
            }
            NameFor::RenameCrate(id) => self.crates.rename(*id, name)?,
            NameFor::SendToNew(ids) => {
                let to = self.crates.create(name)?;
                self.send_to(ids, to);
            }
        }
        Ok(())
    }

    /// "Delete crate "X" (40 entries)?" before deleting a crate that has entries.
    fn delete_dialog(&mut self, ctx: &egui::Context) {
        let Some(id) = self.confirm_delete else {
            return;
        };
        let question = format!(
            "Delete crate \"{}\" ({})?",
            self.crates.name(id),
            entries_label(self.crates.entry_count(id))
        );
        let mut choice = None;
        let modal = egui::Modal::new(Id::new("delete-crate")).show(ctx, |ui| {
            ui.label(question);
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(false);
                }
            });
        });
        if modal.should_close() && choice.is_none() {
            choice = Some(false);
        }
        if let Some(delete) = choice {
            self.confirm_delete = None;
            if delete {
                self.delete_crate(id);
            }
        }
    }

    // ---- crates ------------------------------------------------------------------------------

    /// The crate menu under the playlist title bar.
    fn crate_menu(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let (shown, playing) = (self.crates.shown_id(), self.crates.playing_id());
        for c in self.crates.list() {
            let unreadable = self.crates.is_unreadable(c.id);
            let label = crate_menu_label(&c.name, c.id == shown, c.id == playing, unreadable);
            if ui
                .add_enabled(!unreadable, egui::Button::selectable(c.id == shown, label))
                .clicked()
            {
                actions.push(Action::ShowCrate(c.id));
            }
        }
        ui.separator();
        if ui.button("New crate…").clicked() {
            actions.push(Action::NewCrate);
        }
        let editable = shown != PLAYLIST;
        if ui
            .add_enabled(editable, egui::Button::new("Rename crate…"))
            .clicked()
        {
            actions.push(Action::RenameCrate);
        }
        if ui
            .add_enabled(editable, egui::Button::new("Delete crate…"))
            .clicked()
        {
            actions.push(Action::DeleteCrate);
        }
    }

    /// Shows a crate in the window; playback carries on with its own crate.
    fn show_crate(&mut self, id: CrateId) {
        if self.crates.show(id) {
            self.pl_scroll = 0;
            self.pl_drag_from = None;
        }
    }

    /// Deletes a crate; deleting the playing crate stops playback.
    fn delete_crate(&mut self, id: CrateId) {
        if id == self.crates.playing_id() {
            self.with_engine(|e| e.stop());
            self.queue_dirty = true;
        }
        if self.armed.is_some_and(|(c, _)| c == id) {
            self.armed = None;
        }
        if let Err(e) = self.crates.delete(id) {
            self.notify(e);
        }
    }

    /// Copies entries of the shown crate to another one.
    fn send_to(&mut self, ids: &[EntryId], to: CrateId) {
        match self.crates.send(self.crates.shown_id(), ids, to) {
            Ok(sent) => {
                if sent > 0 && to == self.crates.playing_id() {
                    self.queue_dirty = true;
                }
                let skipped = ids.len() - sent;
                let mut text = format!("Sent {} to {}", entries_label(sent), self.crates.name(to));
                if skipped > 0 {
                    text += &format!(" ({skipped} already there)");
                }
                self.notify(text);
            }
            Err(e) => self.notify(e),
        }
    }

    fn save_presets(&mut self) {
        if let Some(s) = &self.store
            && let Err(e) = s.save(PRESETS_FILE, &self.presets)
        {
            self.notify(format!("Could not save presets: {e}"));
        }
    }

    fn apply(&mut self, a: Action, ctx: &egui::Context) {
        match a {
            Action::Prev => {
                self.armed = None;
                self.with_engine(|e| e.previous());
            }
            Action::Play => self.play(),
            Action::Pause => self.with_engine(|e| e.toggle_pause()),
            Action::Stop => self.with_engine(|e| e.stop()),
            Action::Next => {
                self.armed = None;
                self.with_engine(|e| e.next());
            }
            Action::Eject => self.open_files_dialog(Open::Replace),
            Action::Seek(s) => self.with_engine(|e| e.seek(s)),
            Action::Volume(v) => self.set_volume(v),
            Action::Balance(b) => {
                self.settings.balance = b;
                self.with_engine(|e| e.set_balance(b));
                self.mark_settings();
            }
            Action::Shuffle => {
                self.settings.shuffle = !self.settings.shuffle;
                self.queue_dirty = true;
                self.mark_settings();
            }
            Action::Repeat => {
                self.settings.repeat = self.settings.repeat.cycle();
                let r = self.settings.repeat.to_engine();
                self.with_engine(|e| e.set_repeat(r));
                self.mark_settings();
            }
            Action::ToggleEq => {
                self.settings.show_eq = !self.settings.show_eq;
                self.mark_settings();
            }
            Action::TogglePlaylist => {
                self.settings.show_playlist = !self.settings.show_playlist;
                self.mark_settings();
            }
            Action::ToggleRemaining => {
                self.settings.time_remaining = !self.settings.time_remaining;
                self.mark_settings();
            }
            Action::CycleVis => {
                self.settings.vis = self.settings.vis.cycle();
                self.mark_settings();
            }
            Action::ToggleSpectrogram => self.spectro_open = !self.spectro_open,
            Action::ToggleScale => {
                self.settings.scale = if self.settings.scale >= 2 { 1 } else { 2 };
                self.mark_settings();
            }
            Action::Rows(d) => {
                self.settings.playlist_rows =
                    (self.settings.playlist_rows as i32 + d).clamp(4, 60) as u16;
                self.mark_settings();
            }
            Action::SavePreset => self.name_dialog = Some(NameDialog::new(NameFor::Preset, "")),
            Action::DeletePreset(name) => {
                self.presets.remove(&name);
                self.save_presets();
            }
            Action::PlayEntry(id) => self.play_entry(self.crates.shown_id(), id),
            Action::RenderShow(id) => self.open_render_dialog(id),
            Action::CancelRender => {
                if let Some((job, _)) = &self.render_job {
                    job.cancel();
                }
            }
            Action::Select(i, m) => self.crates.shown_mut().click(i, m),
            Action::Move(from, to) => {
                self.crates.shown_mut().move_entry(from, to);
                self.mark_shown();
            }
            Action::AddFiles => self.open_files_dialog(Open::Add),
            Action::AddFolder => self.open_folder_dialog(),
            Action::RemoveSelected => self.remove_selected(),
            Action::Clear => {
                self.crates.shown_mut().clear();
                if self.crates.shown_id() == self.crates.playing_id() {
                    self.with_engine(|e| e.stop());
                }
                self.mark_shown();
            }
            Action::SelectAll => self.crates.shown_mut().select_all(),
            Action::SelectNone => self.crates.shown_mut().select_none(),
            Action::InvertSelection => self.crates.shown_mut().invert_selection(),
            Action::ExportM3u => self.export_m3u(),
            Action::ShowCrate(id) => self.show_crate(id),
            Action::NewCrate => self.name_dialog = Some(NameDialog::new(NameFor::NewCrate, "")),
            Action::RenameCrate => {
                let id = self.crates.shown_id();
                let name = self.crates.name(id).to_owned();
                self.name_dialog = Some(NameDialog::new(NameFor::RenameCrate(id), name));
            }
            Action::DeleteCrate => {
                let id = self.crates.shown_id();
                if self.crates.entry_count(id) > 0 {
                    self.confirm_delete = Some(id);
                } else {
                    self.delete_crate(id);
                }
            }
            Action::SendTo(entry, to) => {
                let shown = self.crates.shown();
                let ids = if shown.is_selected(entry) {
                    shown.selected_ids()
                } else {
                    vec![entry]
                };
                match to {
                    Some(to) => self.send_to(&ids, to),
                    None => self.name_dialog = Some(NameDialog::new(NameFor::SendToNew(ids), "")),
                }
            }
        }
        let _ = ctx;
    }

    fn save_now(&mut self, force: bool) {
        let Some(store) = self.store.clone() else {
            return;
        };
        let due = |t: Option<Instant>| t.is_some_and(|t| force || t.elapsed() > SAVE_DELAY);
        if due(self.settings_dirty) {
            self.settings_dirty = None;
            if let Err(e) = store.save(SETTINGS_FILE, &self.settings) {
                self.notify(format!("Could not save settings: {e}"));
            }
        }
        for e in self.crates.save_due(force, SAVE_DELAY) {
            self.notify(e);
        }
    }
}

enum Action {
    Prev,
    Play,
    Pause,
    Stop,
    Next,
    Eject,
    Seek(f64),
    Volume(f32),
    Balance(f32),
    Shuffle,
    Repeat,
    ToggleEq,
    TogglePlaylist,
    ToggleRemaining,
    CycleVis,
    ToggleScale,
    ToggleSpectrogram,
    Rows(i32),
    SavePreset,
    DeletePreset(String),
    PlayEntry(EntryId),
    /// Render this entry's visual show to a video file.
    RenderShow(EntryId),
    CancelRender,
    Select(usize, ClickMods),
    Move(usize, usize),
    AddFiles,
    AddFolder,
    RemoveSelected,
    Clear,
    SelectAll,
    SelectNone,
    InvertSelection,
    ExportM3u,
    ShowCrate(CrateId),
    NewCrate,
    /// Rename or delete the shown crate.
    RenameCrate,
    DeleteCrate,
    /// Send an entry (with the rest of the selection, when it is selected) to a crate, or to
    /// a new one.
    SendTo(EntryId, Option<CrateId>),
}

/// How opened files are used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Open {
    /// Add to the shown crate.
    Add,
    /// Add to the shown crate and play the first one.
    AddAndPlay,
    /// Replace the Playlist crate and play it (Eject).
    Replace,
}

struct NameDialog {
    purpose: NameFor,
    text: String,
    error: Option<String>,
}

impl NameDialog {
    fn new(purpose: NameFor, text: impl Into<String>) -> Self {
        Self {
            purpose,
            text: text.into(),
            error: None,
        }
    }
}

enum NameFor {
    Preset,
    NewCrate,
    RenameCrate(CrateId),
    /// A new crate for these entries of the shown crate.
    SendToNew(Vec<EntryId>),
}

impl NameFor {
    /// Window title and confirm button.
    fn labels(&self) -> (&'static str, &'static str) {
        match self {
            NameFor::Preset => ("Save EQ preset", "Save"),
            NameFor::NewCrate | NameFor::SendToNew(_) => ("New crate", "Create"),
            NameFor::RenameCrate(_) => ("Rename crate", "Rename"),
        }
    }
}

fn entries_label(n: usize) -> String {
    if n == 1 {
        "1 entry".into()
    } else {
        format!("{n} entries")
    }
}

/// A crate in the crate menu: • marks the shown crate, ⏵ the playing one (marks egui's default
/// fonts can draw).
fn crate_menu_label(name: &str, shown: bool, playing: bool, unreadable: bool) -> String {
    let mut label = format!("{} {name}", if shown { "•" } else { "  " });
    if playing {
        label += "  ⏵";
    }
    if unreadable {
        label += " (unreadable)";
    }
    label
}

/// Colour and right-hand text of a playlist row: the duration, or a waiting or unavailable
/// entry's note, dimmed. Only files that couldn't be opened are drawn in the error colour.
fn row_look(
    e: &crate::playlist::Entry,
    current: bool,
    colors: &crate::skin::Colors,
) -> (Color32, String) {
    let base = if current {
        colors.pl_current
    } else {
        colors.pl_text
    };
    match (&e.status, e.status.note()) {
        (EntryStatus::Failed, _) => (Color32::from_rgb(170, 60, 60), duration_text(e)),
        (_, Some(note)) => (lerp_color(base, colors.pl_bg, 0.55), note.to_owned()),
        _ => (color(base), duration_text(e)),
    }
}

fn duration_text(e: &crate::playlist::Entry) -> String {
    e.duration.map(format::clock).unwrap_or_default()
}

/// The playlist title bar's text: the crate name folded to the skin font's upper case, without
/// characters the font lacks, cut to `max_w` skin pixels.
pub fn crate_title(def: &crate::skin::SkinDef, name: &str, max_w: f32) -> String {
    let advance = def.font.advance as f32;
    let fit = ((max_w + 1.0) / advance).floor().max(0.0) as usize;
    let text: String = name
        .chars()
        .map(crate::skin::fold)
        .filter(|&c| def.glyph(c).is_some())
        .take(fit)
        .collect();
    text.trim().to_owned()
}

/// Draws the shown crate's name centred on the playlist title bar, over a plain strip that
/// hides the bar's decorative lines behind it.
fn draw_crate_name(sk: &Skinned, name: &str) {
    const PAD: f32 = 5.0; // plain title bar either side of the name
    const SIDE: f32 = 40.0; // decoration left visible at each end (the close button's side)
    let bar = sk.def.at("pl_titlebar");
    let text = crate_title(sk.def, name, bar.w as f32 - 2.0 * (SIDE + PAD));
    if text.is_empty() {
        return;
    }
    let w = sk.text_width(&text);
    let x = ((bar.w as f32 - w) / 2.0).round();
    sk.sprite_in(
        "pl_title_fill",
        sk.rect(
            bar.x as f32 + x - PAD,
            bar.y as f32,
            w + 2.0 * PAD,
            bar.h as f32,
        ),
    );
    let y = bar.y as f32 + ((bar.h - sk.def.font.glyph_h) / 2) as f32;
    sk.text(bar.x as f32 + x, y, &text, color(sk.def.colors.pl_title));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostAction {
    ToggleStrip,
    ToggleAnnotating,
    Tap,
    Boundary(analysis::SectionKind),
}

/// Fullscreen keys the host keeps instead of passing them to the scene: `T` (analysis strip),
/// `A` (annotation mode), and while annotating `Space` (beat tap) and `1`–`6` (boundary of kind
/// intro, build, drop, breakdown, groove, outro).
pub fn host_action(key: Key, annotating: bool) -> Option<HostAction> {
    use analysis::SectionKind as K;
    Some(match key {
        Key::T => HostAction::ToggleStrip,
        Key::A => HostAction::ToggleAnnotating,
        _ if !annotating => return None,
        Key::Space => HostAction::Tap,
        Key::Num1 => HostAction::Boundary(K::Intro),
        Key::Num2 => HostAction::Boundary(K::Build),
        Key::Num3 => HostAction::Boundary(K::Drop),
        Key::Num4 => HostAction::Boundary(K::Breakdown),
        Key::Num5 => HostAction::Boundary(K::Groove),
        Key::Num6 => HostAction::Boundary(K::Outro),
        _ => return None,
    })
}

/// What the window is doing, for the repaint policy.
#[derive(Debug, Clone, Copy, Default)]
pub struct Activity {
    pub playing: bool,
    /// The waveform section is visible (it scrolls with the playhead).
    pub waveform: bool,
    pub bars_moving: bool,
    pub engine_starting: bool,
    pub hidden: bool,
    pub fullscreen: bool,
}

/// When to repaint without input. `None` means only on input, so an idle window costs nothing.
/// Fullscreen schedules its own frames (one per vsync), so it is not handled here.
pub fn repaint_after(a: Activity) -> Option<Duration> {
    if a.hidden || a.fullscreen {
        None
    } else if a.playing && a.waveform {
        // The waveform scrolls with the playhead: smooth at the display rate.
        Some(Duration::from_millis(16))
    } else if a.playing || a.bars_moving {
        Some(Duration::from_millis(33))
    } else if a.engine_starting {
        Some(Duration::from_millis(50))
    } else {
        None
    }
}

fn engine_status(slot: &EngineSlot) -> String {
    match slot {
        EngineSlot::Starting(_) => "OPENING AUDIO DEVICE...".into(),
        EngineSlot::Ready(_) => "WINAMP RUST - DROP FILES HERE".into(),
        EngineSlot::Failed(e) => format!("AUDIO ERROR: {e}"),
    }
}

fn lerp_color(a: [u8; 3], b: [u8; 3], t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgb(l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2]))
}

impl eframe::App for WinampApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.logic_inner(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ui_start = Instant::now();
        self.ui_inner(ui);
        if let Some(p) = &mut self.profile {
            p.ui_done(ui_start, self.fullscreen.is_some());
        }
    }

    fn on_exit(&mut self) {
        self.exit();
    }
}

impl WinampApp {
    fn logic_inner(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        if let Some(p) = &mut self.profile {
            p.logic_start(now);
        }
        if self.auto_fullscreen
            && self.fullscreen.is_none()
            && self.position.state == PlayState::Playing
        {
            self.auto_fullscreen = false;
            self.toggle_fullscreen(ctx);
        }
        // A background show render: progress in the main window; it waits while fullscreen
        // visuals use the GPU.
        if let Some((job, name)) = &self.render_job {
            job.set_paused(self.fullscreen.is_some());
            let status = job.status();
            self.message = Some((crate::render_job::status_line(name, &status), now));
            if status.finished() {
                self.render_job = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
        }
        if self.auto_quit.is_some_and(|t| now >= t) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        let dt = (now - self.last_frame).as_secs_f32().min(0.25);
        self.last_frame = now;
        self.update_audio(dt);
        self.handle_keys(ctx);

        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if !dropped.is_empty() {
            self.add_paths(dropped, Open::Add);
        }

        // Scroll the title while playing (Winamp scrolls ~5 characters per second).
        if self.position.state == PlayState::Playing {
            self.title_tick += dt as f64;
            while self.title_tick > 0.2 {
                self.title_tick -= 0.2;
                self.title_offset = self.title_offset.wrapping_add(1);
            }
        }

        // Keep the window sized to the visible sections (not in fullscreen).
        if self.fullscreen.is_none() {
            let size = Self::window_size(&self.settings, &self.skin);
            if self.last_size != Some(size) {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
                self.last_size = Some(size);
            }
            if let Some(r) = ctx.input(|i| i.viewport().outer_rect) {
                let pos = Some((r.min.x, r.min.y));
                if self.settings.window_pos != pos && ctx.input(|i| !i.pointer.any_down()) {
                    self.settings.window_pos = pos;
                    self.mark_settings();
                }
            }
        }

        let hidden = ctx.input(|i| {
            i.viewport().minimized.unwrap_or(false) || i.viewport().occluded.unwrap_or(false)
        });
        let activity = Activity {
            playing: self.position.state == PlayState::Playing,
            waveform: self.settings.show_waveform,
            bars_moving: self.analyzer.is_active(),
            engine_starting: matches!(self.engine, EngineSlot::Starting(_)),
            hidden,
            fullscreen: self.fullscreen.is_some(),
        };
        if let Some(after) = repaint_after(activity) {
            ctx.request_repaint_after(after);
        }
        if self.settings_dirty.is_some() || self.crates.is_dirty() {
            ctx.request_repaint_after(SAVE_DELAY);
        }
        self.save_now(false);
        if let Some(p) = &mut self.profile {
            p.logic_end();
        }
    }

    fn ui_inner(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if self.tex.is_none() {
            let img = egui::ColorImage::from_rgba_unmultiplied(self.skin.size, &self.skin.rgba);
            self.tex = Some(ctx.load_texture("skin-atlas", img, egui::TextureOptions::NEAREST));
        }
        if self.fullscreen.is_some() {
            self.fullscreen_ui(ui);
        } else {
            let origin = ui.max_rect().min;
            let scale = self.settings.scale as f32;
            let d = self.skin.def.clone();
            self.main_section(ui, origin);
            let mut y = d.main_size.1 as f32;
            if self.settings.show_waveform {
                let rect = Rect::from_min_size(
                    origin + vec2(0.0, y * scale),
                    vec2(d.main_size.0 as f32, crate::waveform::HEIGHT as f32) * scale,
                );
                self.waveform_section(ui, rect);
                y += crate::waveform::HEIGHT as f32;
            }
            if self.settings.show_eq {
                self.eq_section(ui, origin + vec2(0.0, y * scale));
                y += d.eq_size.1 as f32;
            }
            if self.settings.show_playlist {
                self.playlist_section(ui, origin + vec2(0.0, y * scale));
            }
            if self
                .message
                .as_ref()
                .is_some_and(|(_, at)| at.elapsed() >= Duration::from_secs(4))
            {
                self.message = None;
            }
            // An armed entry's "Waiting for …" stays up until it plays.
            let line = self
                .armed_line()
                .or_else(|| self.message.as_ref().map(|(text, _)| text.clone()));
            if let Some(text) = line {
                egui::Tooltip::always_open(
                    ctx.clone(),
                    ui.layer_id(),
                    Id::new("msg"),
                    egui::PopupAnchor::Position(origin),
                )
                .show(|ui| ui.label(text));
            }
            self.name_dialog(&ctx);
            self.delete_dialog(&ctx);
            self.render_dialog_ui(&ctx);
        }
        if self.help {
            crate::help::show(&ctx, &mut self.help);
        }
        self.spectrogram_window(&ctx);
        if !self.first_frame_done {
            self.first_frame_done = true;
            let ms = self.startup.process_start.elapsed().as_secs_f64() * 1e3;
            if self.startup.report {
                println!("startup: first frame after {ms:.1} ms");
            }
            if self.startup.exit_after_first_frame {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    /// Feeds and declares the spectrogram window while it is open, and applies its actions.
    fn spectrogram_window(&mut self, ctx: &egui::Context) {
        for action in self.spectro.take_actions() {
            match action {
                crate::spectrogram::Action::Seek(t) => self.with_engine(|e| e.seek(t)),
                crate::spectrogram::Action::Close => self.spectro_open = false,
                crate::spectrogram::Action::Settings(s) => {
                    self.settings.spectrogram = s;
                    self.mark_settings();
                }
            }
        }
        if !self.spectro_open {
            return;
        }
        let playing = self.position.state != PlayState::Stopped;
        let info = self.now_playing.as_ref().filter(|_| playing);
        let title = info.map_or(String::new(), |i| {
            if i.artist.is_empty() {
                i.title.clone()
            } else {
                format!("{} – {}", i.artist, i.title)
            }
        });
        let overview = self.overview.clone();
        let feed = crate::spectrogram::Feed {
            title,
            track: self
                .track_refs
                .get(&self.position.track)
                .cloned()
                .filter(|_| playing),
            duration: info.and_then(|i| i.duration_secs).or(overview
                .as_ref()
                .filter(|o| o.complete)
                .map(|o| o.seconds())),
            overview,
            now: self.position.seconds(),
            playing: self.position.state == PlayState::Playing,
            fullscreen: self.fullscreen.is_some(),
            lossless: info.is_some_and(|i| i.lossless),
        };
        if self.spectro.feed(feed) {
            ctx.request_repaint_of(crate::spectrogram::viewport_id());
        }
        self.spectro.show(ctx);
    }

    fn exit(&mut self) {
        if let Some(r) = self.fullscreen.as_ref().and_then(|f| f.restore) {
            self.settings.window_pos = Some((r.min.x, r.min.y));
        }
        self.settings_dirty.get_or_insert_with(Instant::now);
        self.save_now(true);
    }
}

/// Presentation settings for the app's window: vsync with up to 3 frames queued. With eframe's
/// default of 1 (measured on an M2 MacBook, even for a blank fullscreen window) frames regularly
/// miss their vsync slot and the rate swings between 30 and 60 fps; with 3 it holds 60. The
/// windowed player only draws on input or at 30 fps, so its queue stays short and clicks stay
/// snappy. (Set at startup: eframe 0.36's `Frame::set_wgpu_surface_config` changes a clone of
/// the render state and has no effect.) The visuals read the audio clock ahead to make up for
/// the display delay.
pub fn surface_config() -> eframe::egui_wgpu::SurfaceConfig {
    eframe::egui_wgpu::SurfaceConfig {
        present_mode: eframe::egui_wgpu::wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: Some(3),
    }
}

#[derive(Default)]
struct PhaseStats {
    sum: f64,
    max: f64,
    n: u32,
}

impl PhaseStats {
    fn add(&mut self, ms: f64) {
        self.sum += ms;
        self.max = self.max.max(ms);
        self.n += 1;
    }

    fn show(&self) -> String {
        format!("{:5.2}/{:5.2}", self.sum / self.n.max(1) as f64, self.max)
    }
}

/// Where each frame's time goes: our `logic`, our `ui` (which includes the visuals' `paint` and
/// egui layer), and everything eframe does outside them (egui tessellation, GPU submit, waiting
/// for a drawable, present, event loop).
struct FrameProfile {
    window: Instant,
    frames: u32,
    frame_dt: PhaseStats,
    logic: PhaseStats,
    ui: PhaseStats,
    outside: PhaseStats,
    last_logic_start: Option<Instant>,
    logic_started: Instant,
    ui_end: Option<Instant>,
    log: Option<std::fs::File>,
}

impl FrameProfile {
    fn new() -> Self {
        let path = std::env::temp_dir().join("winamp_frame_stats.log");
        eprintln!("frame stats → {}", path.display());
        Self {
            window: Instant::now(),
            frames: 0,
            frame_dt: PhaseStats::default(),
            logic: PhaseStats::default(),
            ui: PhaseStats::default(),
            outside: PhaseStats::default(),
            last_logic_start: None,
            logic_started: Instant::now(),
            ui_end: None,
            log: std::fs::File::create(path).ok(),
        }
    }

    fn logic_start(&mut self, now: Instant) {
        if let Some(end) = self.ui_end.take() {
            self.outside.add((now - end).as_secs_f64() * 1e3);
        }
        if let Some(prev) = self.last_logic_start {
            self.frame_dt.add((now - prev).as_secs_f64() * 1e3);
        }
        self.last_logic_start = Some(now);
        self.logic_started = now;
    }

    fn logic_end(&mut self) {
        self.logic
            .add(self.logic_started.elapsed().as_secs_f64() * 1e3);
    }

    fn ui_done(&mut self, start: Instant, fullscreen: bool) {
        let now = Instant::now();
        self.ui.add((now - start).as_secs_f64() * 1e3);
        self.ui_end = Some(now);
        self.frames += 1;
        if self.window.elapsed() >= Duration::from_secs(1) {
            let line = format!(
                "{} fps {:3} | frame ms avg/max {} | logic {} | ui+visuals {} | eframe paint+present {}",
                if fullscreen { "full" } else { "win " },
                self.frames,
                self.frame_dt.show(),
                self.logic.show(),
                self.ui.show(),
                self.outside.show(),
            );
            eprintln!("{line}");
            if let Some(f) = &mut self.log {
                use std::io::Write;
                let _ = writeln!(f, "{line}");
            }
            let log = self.log.take();
            *self = Self {
                log,
                last_logic_start: self.last_logic_start,
                ui_end: self.ui_end,
                ..Self::quiet()
            };
        }
    }

    fn quiet() -> Self {
        Self {
            window: Instant::now(),
            frames: 0,
            frame_dt: PhaseStats::default(),
            logic: PhaseStats::default(),
            ui: PhaseStats::default(),
            outside: PhaseStats::default(),
            last_logic_start: None,
            logic_started: Instant::now(),
            ui_end: None,
            log: None,
        }
    }
}

/// Whether a click opens a context menu: the secondary button, or Control-click where Control
/// isn't the command key (macOS; elsewhere Ctrl-click stays multi-select).
pub fn opens_context_menu(secondary: bool, clicked: bool, mods: Modifiers) -> bool {
    secondary || (clicked && mods.ctrl && !mods.command)
}

/// Structure-navigation state mirrored from engine events.
#[derive(Debug, Default)]
struct Nav {
    /// A quantized jump will land at this time of the current track.
    pending_jump: Option<f64>,
    /// Where the pending jump goes (to jump at once if it misses).
    jump_target: Option<f64>,
    loop_region: Option<(f64, f64)>,
    /// Length of the `Shift+L` loop, when that is what's looping.
    loop_bars: Option<usize>,
}

// Keep `Rect`/`Pos2` imports used in all cfgs.
#[allow(dead_code)]
fn _unused(_: Rect, _: Pos2) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_keys_are_not_forwarded_to_the_scene() {
        use analysis::SectionKind as K;
        assert_eq!(host_action(Key::T, false), Some(HostAction::ToggleStrip));
        assert_eq!(
            host_action(Key::A, false),
            Some(HostAction::ToggleAnnotating)
        );
        // Digits and Space belong to the scene unless annotating.
        assert_eq!(host_action(Key::Num1, false), None);
        assert_eq!(host_action(Key::Space, false), None);
        assert_eq!(
            host_action(Key::Num3, true),
            Some(HostAction::Boundary(K::Drop))
        );
        assert_eq!(
            host_action(Key::Num6, true),
            Some(HostAction::Boundary(K::Outro))
        );
        assert_eq!(host_action(Key::Space, true), Some(HostAction::Tap));
        // Anything else still reaches the scene (e.g. the visual engine's M / K / D).
        for k in [Key::M, Key::K, Key::D, Key::Num7] {
            assert_eq!(host_action(k, true), None);
        }
    }

    #[test]
    fn idle_window_never_repaints_on_its_own() {
        assert_eq!(
            repaint_after(Activity::default()),
            None,
            "stopped and quiet"
        );
        let playing = Activity {
            playing: true,
            ..Default::default()
        };
        assert_eq!(
            repaint_after(playing),
            Some(Duration::from_millis(33)),
            "30 Hz while playing"
        );
        let scrolling = Activity {
            playing: true,
            waveform: true,
            ..Default::default()
        };
        assert_eq!(
            repaint_after(scrolling),
            Some(Duration::from_millis(16)),
            "waveform scrolls smoothly"
        );
        let paused_waveform = Activity {
            waveform: true,
            ..Default::default()
        };
        assert_eq!(
            repaint_after(paused_waveform),
            None,
            "no repaint while paused"
        );
        let falling = Activity {
            bars_moving: true,
            ..Default::default()
        };
        assert!(repaint_after(falling).is_some(), "bars settle after stop");
        let minimized = Activity {
            playing: true,
            hidden: true,
            ..Default::default()
        };
        assert_eq!(
            repaint_after(minimized),
            None,
            "no repaints while minimized/occluded"
        );
    }

    #[test]
    fn control_click_opens_the_context_menu_on_macos() {
        let mac_ctrl = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let mac_cmd = Modifiers {
            mac_cmd: true,
            command: true,
            ..Default::default()
        };
        let pc_ctrl = Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        assert!(
            opens_context_menu(true, false, Modifiers::NONE),
            "right button / two-finger click"
        );
        assert!(
            opens_context_menu(false, true, mac_ctrl),
            "Control-click on a Mac"
        );
        assert!(
            !opens_context_menu(false, true, mac_cmd),
            "Cmd-click selects"
        );
        assert!(
            !opens_context_menu(false, true, pc_ctrl),
            "Ctrl-click selects on Windows/Linux"
        );
        assert!(!opens_context_menu(false, true, Modifiers::NONE));
    }

    #[test]
    fn window_size_follows_sections() {
        let skin = LoadedSkin::default_skin();
        let mut s = Settings {
            scale: 1,
            show_eq: false,
            show_playlist: false,
            show_waveform: false,
            ..Default::default()
        };
        assert_eq!(WinampApp::window_size(&s, &skin), vec2(275.0, 116.0));
        s.show_eq = true;
        assert_eq!(WinampApp::window_size(&s, &skin), vec2(275.0, 232.0));
        s.show_playlist = true;
        s.playlist_rows = 10;
        assert_eq!(
            WinampApp::window_size(&s, &skin),
            vec2(275.0, 232.0 + 20.0 + 130.0 + 38.0)
        );
        s.scale = 2;
        assert_eq!(WinampApp::window_size(&s, &skin), vec2(550.0, 840.0));
        s.show_waveform = true;
        assert_eq!(
            WinampApp::window_size(&s, &skin),
            vec2(550.0, 840.0 + 116.0)
        );
    }
}

/// The player driven headlessly: a real engine on a `ManualSink`, crates in a temp config
/// folder, and egui frames fed with synthetic pointer and key events.
#[cfg(test)]
mod headless_tests {
    use super::*;
    use crate::crates::PLAYLIST;
    use crate::playlist::Origin;
    use egui::{Event, PointerButton};
    use platform::CallbackInfo;
    use platform::native::{NativeFileSource, NativeSpawner};
    use platform::testing::ManualSink;
    use std::path::Path;

    const BUF: usize = 512;
    /// Classic size, main window and playlist only: the playlist starts at y = 116.
    const PL_TOP: f32 = 116.0;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(format!(
            "{}/../audio/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ui-app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    struct Rig {
        app: WinampApp,
        ctx: egui::Context,
        sink: ManualSink,
        now_ns: u64,
        dir: PathBuf,
        /// Modifier keys held during the next frames.
        mods: Modifiers,
    }

    impl Rig {
        /// `prepare` may fill the config folder before the app starts.
        fn new(name: &str, open: Vec<PathBuf>, prepare: impl FnOnce(&Store)) -> Self {
            let dir = temp(name);
            let store = Store::new(dir.join("config"));
            store
                .save(
                    SETTINGS_FILE,
                    &Settings {
                        scale: 1,
                        show_eq: false,
                        show_waveform: false,
                        repeat: Repeat::One, // the 2 s fixtures keep playing
                        ..Default::default()
                    },
                )
                .unwrap();
            prepare(&store);
            let sink = ManualSink::new(48_000, 2);
            let engine_sink = sink.clone();
            let ctx = egui::Context::default();
            let app = WinampApp::build(
                ctx.clone(),
                None,
                AppContext {
                    engine: Box::new(move || {
                        Engine::new(
                            Arc::new(engine_sink),
                            &NativeSpawner,
                            Arc::new(NativeFileSource),
                            audio::EngineConfig::default(),
                        )
                        .map_err(|e| e.to_string())
                    }),
                    spawner: Arc::new(NativeSpawner),
                    files: Arc::new(NativeFileSource),
                    store: Some(store),
                    open,
                    scene: Box::new(crate::fullscreen::BeatFlash::default()),
                    startup: Startup {
                        process_start: Instant::now(),
                        report: false,
                        exit_after_first_frame: false,
                    },
                    analysis: None,
                    annotations_dir: None,
                    overviews: None,
                    show_renderer: None,
                },
            );
            let mut rig = Rig {
                app,
                ctx,
                sink,
                now_ns: 0,
                dir,
                mods: Modifiers::NONE,
            };
            rig.until(|r| r.app.engine().is_some(), "the engine starts");
            rig
        }

        fn frame(&mut self, mut events: Vec<Event>) -> egui::FullOutput {
            events.insert(0, Event::ModifiersChanged(self.mods));
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(275.0, 400.0))),
                events,
                ..Default::default()
            };
            let app = &mut self.app;
            let mut out = self.ctx.run_ui(input, |ui| {
                app.logic_inner(ui.ctx());
                app.ui_inner(ui);
            });
            out.textures_delta.clear();
            out
        }

        /// One device callback (advancing fake time by a buffer), then a UI frame.
        fn pump(&mut self) -> egui::FullOutput {
            std::thread::sleep(Duration::from_micros(BUF as u64 * 1_000_000 / 48_000 / 2));
            self.now_ns += BUF as u64 * 1_000_000_000 / 48_000;
            self.sink.set_now_ns(self.now_ns);
            self.sink.pull(
                BUF,
                CallbackInfo {
                    host_ns: self.now_ns,
                    output_latency_ns: 0,
                },
            );
            self.frame(Vec::new())
        }

        fn until(&mut self, mut done: impl FnMut(&mut Self) -> bool, what: &str) {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !done(self) {
                assert!(Instant::now() < deadline, "timed out: {what}");
                self.pump();
            }
        }

        fn engine(&mut self) -> &mut Engine {
            self.app.engine().expect("engine ready")
        }

        fn engine_queue(&mut self) -> Vec<TrackRef> {
            self.engine().queue().to_vec()
        }

        fn press(&mut self, pos: Pos2, button: PointerButton, pressed: bool) -> egui::FullOutput {
            self.frame(vec![Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: self.mods,
            }])
        }

        fn click_with(&mut self, pos: Pos2, button: PointerButton) -> egui::FullOutput {
            self.frame(vec![Event::PointerMoved(pos)]);
            self.press(pos, button, true);
            self.press(pos, button, false);
            self.frame(Vec::new())
        }

        fn click(&mut self, pos: Pos2) -> egui::FullOutput {
            self.click_with(pos, PointerButton::Primary)
        }

        fn double_click(&mut self, pos: Pos2) -> egui::FullOutput {
            self.frame(vec![Event::PointerMoved(pos)]);
            for _ in 0..2 {
                self.press(pos, PointerButton::Primary, true);
                self.press(pos, PointerButton::Primary, false);
            }
            self.frame(Vec::new())
        }

        /// Clicks the text `label` wherever it is drawn (menu items, buttons).
        fn click_text(&mut self, label: &str) -> egui::FullOutput {
            let out = self.frame(Vec::new());
            let rect = texts(&out)
                .into_iter()
                .find(|t| t.text == label)
                .unwrap_or_else(|| panic!("{label:?} is not on screen: {:?}", text_list(&out)))
                .rect;
            self.click(rect.center())
        }

        fn type_text(&mut self, text: &str) {
            self.frame(vec![Event::Text(text.into())]);
            self.frame(vec![Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }]);
            self.frame(Vec::new());
        }

        /// Cmd+A in the focused text field.
        fn select_all_text(&mut self) {
            self.mods = Modifiers::COMMAND;
            self.frame(vec![Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            }]);
            self.mods = Modifiers::NONE;
        }

        fn title_bar(&self) -> Pos2 {
            pos2(60.0, PL_TOP + 10.0)
        }

        fn row(&self, index: usize) -> Pos2 {
            pos2(60.0, PL_TOP + 20.0 + index as f32 * 13.0 + 6.5)
        }

        fn ids(&self, crate_id: CrateId) -> Vec<EntryId> {
            let p = self.app.crates.get(crate_id).expect("loaded");
            p.entries().iter().map(|e| e.id).collect()
        }

        /// A new crate holding these fixtures (not shown).
        fn crate_with(&mut self, name: &str, files: &[&str]) -> CrateId {
            let id = self.app.crates.create(name).unwrap();
            let tracks = files
                .iter()
                .map(|f| TrackRef::new(fixture(f).to_string_lossy()));
            self.app.crates.get_mut(id).unwrap().add(tracks);
            id
        }

        fn menu_open(&self) -> bool {
            egui::Popup::is_id_open(&self.ctx, Id::new("crate_menu"))
        }
    }

    struct Text {
        text: String,
        rect: Rect,
        color: Option<Color32>,
    }

    fn texts(out: &egui::FullOutput) -> Vec<Text> {
        fn walk(shape: &egui::Shape, acc: &mut Vec<Text>) {
            match shape {
                egui::Shape::Text(t) => acc.push(Text {
                    text: t.galley.text().to_owned(),
                    rect: t.visual_bounding_rect(),
                    color: t.galley.job.sections.first().map(|s| s.format.color),
                }),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, acc)),
                _ => {}
            }
        }
        let mut acc = Vec::new();
        for s in &out.shapes {
            walk(&s.shape, &mut acc);
        }
        acc
    }

    fn text_list(out: &egui::FullOutput) -> Vec<String> {
        texts(out).into_iter().map(|t| t.text).collect()
    }

    fn shows(out: &egui::FullOutput, text: &str) -> bool {
        texts(out).iter().any(|t| t.text == text)
    }

    fn start_drag(out: &egui::FullOutput) -> bool {
        out.viewport_output.values().any(|v| {
            v.commands
                .iter()
                .any(|c| matches!(c, ViewportCommand::StartDrag))
        })
    }

    // ---- playback follows its crate -------------------------------------------------------

    #[test]
    fn switching_crates_leaves_the_engine_queue_alone() {
        let mut rig = Rig::new("switch", Vec::new(), |_| {});
        rig.app.add_paths(
            vec![
                fixture("tone.flac"),
                fixture("tone.wav"),
                fixture("tone.ogg"),
            ],
            Open::Add,
        );
        rig.until(|r| r.engine().state() == PlayState::Playing, "playing");
        let queue = rig.engine_queue();
        assert_eq!(queue.len(), 3);

        let b = rig.crate_with("B", &["tone.mp3", "tone.m4a"]);
        rig.app.apply(Action::ShowCrate(b), &rig.ctx.clone());
        // Edits to a crate that isn't playing don't touch the queue either.
        rig.app.add_paths(vec![fixture("mono48k.wav")], Open::Add);
        for _ in 0..5 {
            rig.pump();
        }
        assert_eq!(rig.app.crates.shown_id(), b);
        assert_eq!(rig.app.crates.playing_id(), PLAYLIST);
        assert_eq!(
            rig.engine_queue(),
            queue,
            "the queue still is the Playlist crate"
        );
        assert_eq!(rig.engine().state(), PlayState::Playing);
        assert_eq!(rig.engine().current_index(), Some(0));
    }

    #[test]
    fn starting_a_track_in_another_crate_retargets_next_and_previous() {
        let mut rig = Rig::new("retarget", Vec::new(), |_| {});
        rig.app
            .add_paths(vec![fixture("tone.flac"), fixture("tone.wav")], Open::Add);
        rig.until(|r| r.engine().state() == PlayState::Playing, "playing");
        let b = rig.crate_with("B", &["tone.mp3", "tone.ogg", "tone.m4a"]);
        rig.app.apply(Action::ShowCrate(b), &rig.ctx.clone());
        rig.frame(Vec::new());

        rig.double_click(rig.row(0));
        assert_eq!(rig.app.crates.playing_id(), b);
        let b_tracks: Vec<TrackRef> = rig
            .app
            .crates
            .get(b)
            .unwrap()
            .entries()
            .iter()
            .map(|e| e.track.clone())
            .collect();
        assert_eq!(rig.engine_queue(), b_tracks, "the queue is crate B");

        rig.app.apply(Action::Next, &rig.ctx.clone());
        let second = rig.ids(b)[1];
        rig.until(
            |r| r.app.crates.get(b).unwrap().current() == Some(second),
            "next plays B's second entry",
        );
        rig.app.apply(Action::Prev, &rig.ctx.clone());
        let first = rig.ids(b)[0];
        rig.until(
            |r| r.app.crates.get(b).unwrap().current() == Some(first),
            "previous goes back within B",
        );

        // Deleting the playing crate stops playback.
        rig.app.delete_crate(b);
        rig.frame(Vec::new());
        assert_eq!(rig.engine().state(), PlayState::Stopped);
        assert_eq!(rig.app.crates.playing_id(), PLAYLIST);
        assert_eq!(rig.app.crates.shown_id(), PLAYLIST);
    }

    // ---- entries waiting for their audio ------------------------------------------------

    #[test]
    fn an_armed_entry_starts_within_100_ms_of_its_audio_arriving() {
        let mut rig = Rig::new("armed", Vec::new(), |_| {});
        rig.app.add_paths(vec![fixture("tone.flac")], Open::Add);
        rig.until(|r| r.engine().state() == PlayState::Playing, "playing");
        let waiting = rig.app.crates.shown_mut().add_waiting(
            "Nightcraft",
            "Glasshouse",
            Some("https://www.youtube.com/watch?v=abcdefghijk".into()),
            None,
            "downloading 40%",
        );
        rig.app.mark_shown();
        rig.frame(Vec::new());

        let out = rig.double_click(rig.row(1));
        assert_eq!(rig.app.armed, Some((PLAYLIST, waiting)));
        assert!(
            shows(
                &out,
                "Waiting for Nightcraft - Glasshouse (downloading 40%)"
            ),
            "{:?}",
            text_list(&out)
        );
        for _ in 0..10 {
            rig.pump();
        }
        assert_eq!(
            rig.engine().state(),
            PlayState::Playing,
            "the current track plays on"
        );
        assert_eq!(rig.engine().current_index(), Some(0));
        assert_eq!(
            rig.engine_queue().len(),
            1,
            "the waiting entry isn't queued"
        );

        // The download finishes: the file lands in the cache and the producer reports it.
        let cached = rig.dir.join("abcdefghijk.wav");
        std::fs::copy(fixture("tone.wav"), &cached).unwrap();
        let before = rig.app.position.track;
        let arrived_ns = rig.now_ns;
        rig.app
            .set_audio(PLAYLIST, waiting, TrackRef::new(cached.to_string_lossy()));
        assert_eq!(rig.app.armed, None);
        // Audible: the clock runs on a new track instance, past its first frames.
        rig.until(
            |r| {
                let p = r.app.position;
                p.track != before && p.state == PlayState::Playing && p.frame > 0
            },
            "the armed entry plays",
        );
        assert_eq!(rig.app.crates.playing().current(), Some(waiting));
        let ms = (rig.now_ns - arrived_ns) as f64 / 1e6;
        assert!(ms < 100.0, "audible {ms:.1} ms after its audio arrived");
        assert_eq!(rig.engine().stats().underruns, 0);
    }

    #[test]
    fn starting_another_track_cancels_the_arming() {
        let mut rig = Rig::new("disarm", Vec::new(), |_| {});
        rig.app
            .add_paths(vec![fixture("tone.flac"), fixture("tone.wav")], Open::Add);
        rig.until(|r| r.engine().state() == PlayState::Playing, "playing");
        let waiting = rig
            .app
            .crates
            .shown_mut()
            .add_waiting("", "Later", None, None, "listed");
        rig.app.apply(Action::PlayEntry(waiting), &rig.ctx.clone());
        assert!(rig.app.armed.is_some());
        rig.double_click(rig.row(1));
        assert_eq!(rig.app.armed, None);
        rig.frame(Vec::new());
        assert!(rig.app.armed_line().is_none());
    }

    #[test]
    fn opening_files_replaces_only_the_playlist_crate() {
        let lowtide = std::cell::Cell::new(0);
        let mut rig = Rig::new(
            "open",
            vec![fixture("tone.flac"), fixture("tone.wav")],
            |store| {
                let mut c = Crates::open(store);
                c.shown_mut()
                    .add([TrackRef::new(fixture("tone.ogg").to_string_lossy())]);
                c.touch(PLAYLIST);
                let lt = c.create("Lowtide Tapes").unwrap();
                c.get_mut(lt)
                    .unwrap()
                    .add([TrackRef::new(fixture("tone.mp3").to_string_lossy())]);
                c.touch(lt);
                c.show(lt);
                c.save_due(true, Duration::ZERO);
                lowtide.set(lt);
            },
        );
        let lt = lowtide.get();
        let stem = |r: &Rig, c: CrateId| -> Vec<String> {
            r.app
                .crates
                .get(c)
                .unwrap()
                .entries()
                .iter()
                .map(|e| e.track.stem().to_owned())
                .collect()
        };
        assert_eq!(rig.app.crates.shown_id(), PLAYLIST);
        assert_eq!(rig.app.crates.playing_id(), PLAYLIST);
        rig.until(
            |r| r.engine().state() == PlayState::Playing,
            "plays the first file",
        );
        let first = rig.ids(PLAYLIST)[0];
        assert_eq!(rig.app.crates.playing().current(), Some(first));
        assert_eq!(rig.app.crates.shown().len(), 2, "exactly the opened files");
        rig.app.crates.load(lt);
        assert_eq!(stem(&rig, lt), ["tone"], "Lowtide Tapes is unchanged");
        assert!(
            rig.app.crates.get(lt).unwrap().entries()[0]
                .track
                .0
                .ends_with("tone.mp3")
        );

        // Eject does the same while another crate is shown.
        rig.app.apply(Action::ShowCrate(lt), &rig.ctx.clone());
        rig.app.add_paths(vec![fixture("tone.m4a")], Open::Replace);
        assert_eq!(
            (rig.app.crates.shown_id(), rig.app.crates.playing_id()),
            (PLAYLIST, PLAYLIST)
        );
        assert_eq!(rig.app.crates.shown().len(), 1);
        assert!(
            rig.app.crates.get(lt).unwrap().entries()[0]
                .track
                .0
                .ends_with("tone.mp3")
        );
    }

    #[test]
    fn waiting_and_unavailable_rows_are_dimmed_and_failed_rows_red() {
        let mut rig = Rig::new("rows", Vec::new(), |_| {});
        rig.app.add_paths(
            vec![fixture("tone.flac"), fixture("garbage.mp3")],
            Open::Add,
        );
        let p = rig.app.crates.shown_mut();
        p.add_waiting("Nightcraft", "Glasshouse", None, None, "downloading 40%");
        let gone = p.add_waiting("Nightcraft", "B-side", None, None, "listed");
        p.set_unavailable(gone, "no clip");
        rig.until(
            |r| r.app.crates.shown().entries()[1].status == EntryStatus::Failed,
            "the broken file is found",
        );
        let out = rig.frame(Vec::new());
        let colors = rig.app.skin.def.colors.clone();
        let dim = lerp_color(colors.pl_text, colors.pl_bg, 0.55);
        let color_of = |text: &str| {
            texts(&out)
                .into_iter()
                .find(|t| t.text == text)
                .unwrap_or_else(|| panic!("{text:?} not drawn: {:?}", text_list(&out)))
                .color
        };
        assert_eq!(color_of("downloading 40%"), Some(dim));
        assert_eq!(color_of("3. Nightcraft - Glasshouse"), Some(dim));
        assert_eq!(color_of("no clip"), Some(dim));
        assert_eq!(color_of("4. Nightcraft - B-side"), Some(dim));
        assert_eq!(color_of("2. garbage"), Some(Color32::from_rgb(170, 60, 60)));
        let normal = color_of("1. M83 - Midnight_City");
        assert!(normal == Some(color(colors.pl_text)) || normal == Some(color(colors.pl_current)));
    }

    #[test]
    fn the_title_line_keeps_an_origin_entrys_own_names() {
        let mut rig = Rig::new("origin-title", Vec::new(), |_| {});
        let id = rig.app.crates.shown_mut().add_waiting(
            "Nightcraft",
            "Glasshouse",
            None,
            Some(Origin {
                release: Some(123456),
                ..Default::default()
            }),
            "listed",
        );
        rig.app.apply(Action::PlayEntry(id), &rig.ctx.clone());
        // The fixture's tags say "M83 - Midnight_City".
        rig.app.set_audio(
            PLAYLIST,
            id,
            TrackRef::new(fixture("tone.flac").to_string_lossy()),
        );
        rig.until(
            |r| r.app.now_playing.is_some() && r.app.crates.shown().entries()[0].duration.is_some(),
            "playing, with its duration read",
        );
        assert_eq!(rig.app.now_playing.as_ref().unwrap().artist, "M83");
        let (artist, title, duration) = rig.app.now_playing_names().unwrap();
        assert_eq!(
            (artist.as_str(), title.as_str()),
            ("Nightcraft", "Glasshouse")
        );
        assert!(duration.is_some_and(|d| (d - 2.0).abs() < 0.05));
        let e = &rig.app.crates.shown().entries()[0];
        assert_eq!(e.display_name(), "Nightcraft - Glasshouse");
        assert!(
            e.duration.is_some_and(|d| (d - 2.0).abs() < 0.05),
            "takes the duration"
        );
    }

    // ---- title bar and crate menu --------------------------------------------------------

    #[test]
    fn the_title_bar_names_the_crate_in_the_skin_font() {
        let def = LoadedSkin::default_skin().def;
        assert_eq!(crate_title(&def, "Lowtide Tapes", 200.0), "LOWTIDE TAPES");
        assert_eq!(
            crate_title(&def, "Canción €5", 200.0),
            "CANCION 5",
            "folded, € skipped"
        );
        // 6 px per character, the last one without its gap: 29 px fit five, 28 px four.
        assert_eq!(crate_title(&def, "Keepers", 29.0), "KEEPE");
        assert_eq!(crate_title(&def, "Keepers", 28.0), "KEEP");
        let long = crate_title(&def, &"x".repeat(40), 275.0 - 90.0);
        assert!(
            long.len() < 40 && long.len() * 6 - 1 <= 185,
            "cut to fit: {long}"
        );
    }

    #[test]
    fn a_click_on_the_title_bar_opens_the_crate_menu_and_a_drag_moves_the_window() {
        let mut rig = Rig::new("titlebar", Vec::new(), |_| {});
        let at = rig.title_bar();
        rig.frame(vec![Event::PointerMoved(at)]);
        rig.press(at, PointerButton::Primary, true);
        let mut moved = false;
        for i in 1..=4 {
            let out = rig.frame(vec![Event::PointerMoved(at + vec2(8.0 * i as f32, 3.0))]);
            moved |= start_drag(&out);
        }
        rig.press(at + vec2(32.0, 3.0), PointerButton::Primary, false);
        let out = rig.frame(Vec::new());
        assert!(moved, "dragging the title bar moves the window");
        assert!(
            !rig.menu_open() && !shows(&out, "New crate…"),
            "and opens no menu"
        );

        let out = rig.click(at);
        assert!(!start_drag(&out));
        assert!(rig.menu_open(), "a click opens the crate menu");
        let out = rig.frame(Vec::new());
        for item in ["New crate…", "Rename crate…", "Delete crate…"] {
            assert!(shows(&out, item), "{item} in {:?}", text_list(&out));
        }
        assert!(shows(
            &out,
            &crate_menu_label("Playlist", true, true, false)
        ));
        let font = egui::FontId::proportional(14.0);
        assert!(
            rig.ctx.fonts_mut(|f| f.has_glyphs(&font, "•⏵")),
            "the menu's marks have glyphs"
        );
    }

    #[test]
    fn the_crate_menu_switches_creates_and_refuses_duplicate_names() {
        let mut rig = Rig::new("menu", Vec::new(), |_| {});
        let keepers = rig.crate_with("Keepers", &["tone.flac"]);
        rig.click(rig.title_bar());
        rig.click_text(&crate_menu_label("Keepers", false, false, false));
        assert_eq!(rig.app.crates.shown_id(), keepers);
        assert!(!rig.menu_open(), "choosing a crate closes the menu");

        // Marks: ✔ on the shown crate, ▶ on the playing one.
        rig.click(rig.title_bar());
        let out = rig.frame(Vec::new());
        assert!(shows(
            &out,
            &crate_menu_label("Keepers", true, false, false)
        ));
        assert!(shows(
            &out,
            &crate_menu_label("Playlist", false, true, false)
        ));

        rig.click_text("New crate…");
        rig.type_text("keepers");
        let out = rig.frame(Vec::new());
        assert!(
            shows(&out, "A crate named \"Keepers\" already exists"),
            "{:?}",
            text_list(&out)
        );
        assert_eq!(rig.app.crates.list().len(), 2, "no crate is created");
        rig.select_all_text();
        rig.type_text("Gig 12 Oct");
        let names: Vec<&str> = rig
            .app
            .crates
            .list()
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, ["Playlist", "Keepers", "Gig 12 Oct"]);
        let gig = rig.app.crates.list()[2].id;
        assert_eq!(rig.app.crates.shown_id(), gig, "the new crate is shown");
        assert!(rig.app.crates.shown().is_empty());
        assert!(rig.app.name_dialog.is_none());

        // Rename the shown crate.
        rig.click(rig.title_bar());
        rig.click_text("Rename crate…");
        rig.select_all_text();
        rig.type_text("Gig 13 Oct");
        assert_eq!(rig.app.crates.name(gig), "Gig 13 Oct");
    }

    #[test]
    fn deleting_a_crate_with_entries_asks_first_and_playlist_cannot_go() {
        let mut rig = Rig::new("delete", Vec::new(), |_| {});
        // Playlist: Rename and Delete are unavailable.
        rig.click(rig.title_bar());
        rig.click_text("Delete crate…");
        assert!(rig.app.confirm_delete.is_none());
        rig.click(rig.title_bar());
        rig.click_text("Rename crate…");
        assert!(rig.app.name_dialog.is_none());
        assert_eq!(rig.app.crates.list().len(), 1);

        let keepers = rig.crate_with("Keepers", &["tone.flac", "tone.wav"]);
        rig.app.apply(Action::ShowCrate(keepers), &rig.ctx.clone());
        rig.click(rig.title_bar());
        rig.click_text("Delete crate…");
        let out = rig.frame(Vec::new());
        assert!(
            shows(&out, "Delete crate \"Keepers\" (2 entries)?"),
            "{:?}",
            text_list(&out)
        );
        assert_eq!(rig.app.crates.list().len(), 2, "nothing deleted yet");
        rig.click_text("Cancel");
        assert_eq!(rig.app.crates.list().len(), 2, "cancelled");

        rig.click(rig.title_bar());
        rig.click_text("Delete crate…");
        rig.click_text("Delete");
        assert_eq!(rig.app.crates.list().len(), 1, "deleted after confirming");
        assert_eq!(rig.app.crates.shown_id(), PLAYLIST);

        // An empty crate goes without asking.
        let empty = rig.app.crates.create("Empty").unwrap();
        rig.app.apply(Action::ShowCrate(empty), &rig.ctx.clone());
        rig.click(rig.title_bar());
        rig.click_text("Delete crate…");
        assert_eq!(rig.app.crates.list().len(), 1);
    }

    #[test]
    fn send_to_crate_copies_the_selection_from_the_entry_menu() {
        let mut rig = Rig::new("send", Vec::new(), |_| {});
        rig.app.crates.shown_mut().add(
            ["tone.flac", "tone.wav", "tone.ogg"]
                .iter()
                .map(|f| TrackRef::new(fixture(f).to_string_lossy())),
        );
        let keepers = rig.crate_with("Keepers", &["tone.wav"]);
        rig.frame(Vec::new());
        // Select all three, then right-click one of them.
        rig.click(rig.row(0));
        rig.mods = Modifiers::SHIFT;
        rig.click(rig.row(2));
        rig.mods = Modifiers::NONE;
        assert_eq!(rig.app.crates.shown().selected_ids().len(), 3);
        let before = rig.app.crates.shown().to_saved();

        rig.click_with(rig.row(1), PointerButton::Secondary);
        rig.click_text("Send to crate");
        rig.click_text("Keepers");
        let files: Vec<String> = rig
            .app
            .crates
            .get(keepers)
            .unwrap()
            .entries()
            .iter()
            .map(|e| {
                Path::new(&e.track.0)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(
            files,
            ["tone.wav", "tone.flac", "tone.ogg"],
            "two appended, in order"
        );
        assert_eq!(
            rig.app.crates.shown().to_saved(),
            before,
            "the source is unchanged"
        );
        assert_eq!(
            rig.app.message.as_ref().map(|(m, _)| m.as_str()),
            Some("Sent 2 entries to Keepers (1 already there)")
        );

        // New crate… names a new crate for them.
        rig.click_with(rig.row(1), PointerButton::Secondary);
        rig.click_text("Send to crate");
        rig.click_text("New crate…");
        rig.type_text("Gig 12 Oct");
        let gig = rig.app.crates.list().last().unwrap().id;
        assert_eq!(rig.app.crates.name(gig), "Gig 12 Oct");
        assert_eq!(rig.app.crates.get(gig).unwrap().len(), 3);
        assert_eq!(
            rig.app.crates.shown_id(),
            PLAYLIST,
            "the source stays shown"
        );
    }
}
