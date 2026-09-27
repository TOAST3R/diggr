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

use crate::eqcurve;
use crate::files;
use crate::format;
use crate::fullscreen::{SceneFrame, VisualScene};
use crate::metadata::{MetaResult, MetaWorker};
use crate::playlist::{ClickMods, EntryId, Playlist, SavedPlaylist, play_order};
use crate::settings::{
    PLAYLIST_FILE, PRESETS_FILE, Repeat, SETTINGS_FILE, Settings, Store, VisMode,
};
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
    /// Files from the command line: replace the playlist and play.
    pub open: Vec<PathBuf>,
    pub scene: Box<dyn VisualScene>,
    pub startup: Startup,
    /// Music analysis ahead of the playhead (beats, sections) for the visuals.
    pub analysis: Option<analysis::AnalysisService>,
    /// Where annotation mode saves its JSON files.
    pub annotations_dir: Option<PathBuf>,
    /// Native-rate track overviews for the waveform section.
    pub overviews: Option<analysis::overview::OverviewService>,
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
    playlist: Playlist,
    /// Playlist entry ids in engine-queue order.
    queue: Vec<EntryId>,
    queue_dirty: bool,
    shuffle_seed: u64,
    engine: EngineSlot,
    pending_play: Option<Option<EntryId>>,
    tap: Option<TapReader>,
    analyzer: Analyzer,
    meta: Option<MetaWorker>,
    now_playing: Option<TrackInfo>,
    position: Position,
    seek_drag: Option<f32>,
    title_offset: usize,
    title_tick: f64,
    pl_scroll: usize,
    pl_drag_from: Option<usize>,
    pl_resize_acc: f32,
    preset_name: Option<String>,
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
    /// `WINAMP_FRAME_STATS=1`: per-second frame phase timings on stderr and in
    /// `$TMPDIR/winamp_frame_stats.log`.
    profile: Option<FrameProfile>,
    /// `WINAMP_AUTO_FULLSCREEN=1`: enter fullscreen once playback starts (for unattended runs).
    auto_fullscreen: bool,
    /// `WINAMP_AUTO_QUIT_SECS=n`: close the app after n seconds.
    auto_quit: Option<Instant>,
    settings_dirty: Option<Instant>,
    playlist_dirty: Option<Instant>,
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
        let store = ctx.store;
        let settings = store.as_ref().map(Store::load_settings).unwrap_or_default();
        let presets = store.as_ref().map(Store::load_presets).unwrap_or_default();
        let saved: SavedPlaylist = store
            .as_ref()
            .map(|s| s.load(PLAYLIST_FILE))
            .unwrap_or_default();
        let (mut playlist, mut pending_meta) = Playlist::from_saved(saved);

        // Open the audio device in the background: the window must not wait for it.
        let (tx, rx) = std::sync::mpsc::channel();
        let factory = ctx.engine;
        let egui_ctx = cc.egui_ctx.clone();
        std::thread::Builder::new()
            .name("engine-start".into())
            .spawn(move || {
                let _ = tx.send(factory());
                egui_ctx.request_repaint();
            })
            .expect("spawn engine starter");

        let wake_ctx = cc.egui_ctx.clone();
        let meta = MetaWorker::start(&*ctx.spawner, ctx.files.clone(), move || {
            wake_ctx.request_repaint()
        })
        .ok();

        let mut pending_play = None;
        if !ctx.open.is_empty() {
            playlist.clear();
            let added = playlist.add(
                files::expand(&ctx.open)
                    .into_iter()
                    .map(|p| TrackRef::new(p.to_string_lossy())),
            );
            pending_play = Some(added.first().map(|(id, _)| *id));
            pending_meta = added;
        }
        if let Some(m) = &meta {
            m.request(pending_meta);
        }
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);

        let skin = LoadedSkin::default_skin();
        Self {
            def: Arc::new(skin.def.clone()),
            skin,
            tex: None,
            store,
            analyzer: Analyzer::new(48_000),
            settings,
            presets,
            playlist,
            queue: Vec::new(),
            queue_dirty: true,
            shuffle_seed: seed,
            engine: EngineSlot::Starting(rx),
            pending_play,
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
            preset_name: None,
            fullscreen: None,
            scene: ctx.scene,
            scene_ready: false,
            render_state: cc.wgpu_render_state.clone(),
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
            playlist_dirty: None,
            last_size: None,
            message: None,
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

    fn mark_playlist(&mut self) {
        self.queue_dirty = true;
        self.playlist_dirty.get_or_insert_with(Instant::now);
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

    /// Rebuilds the engine queue from the playlist (and shuffle order).
    fn sync_queue(&mut self, first: Option<EntryId>) {
        let first_index = first
            .or(self.playlist.current())
            .and_then(|id| self.playlist.index_of(id));
        let order = play_order(
            self.playlist.len(),
            self.settings.shuffle,
            first_index,
            self.shuffle_seed,
        );
        let entries = self.playlist.entries();
        self.queue = order.iter().map(|&i| entries[i].id).collect();
        let tracks: Vec<TrackRef> = order.iter().map(|&i| entries[i].track.clone()).collect();
        self.queue_dirty = false;
        if let EngineSlot::Ready(e) = &mut self.engine {
            e.set_queue(tracks);
        }
    }

    fn play_entry(&mut self, id: EntryId) {
        if self.engine().is_none() {
            self.pending_play = Some(Some(id));
            return;
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
            self.playlist.set_current(Some(id));
            if let Some(e) = self.engine() {
                e.play_index(q);
            }
        }
    }

    fn play(&mut self) {
        if self.playlist.is_empty() {
            self.open_files_dialog(true);
            return;
        }
        let Some(e) = self.engine() else {
            self.pending_play = Some(None);
            return;
        };
        match e.state() {
            PlayState::Paused => e.resume(),
            PlayState::Playing => {
                if let Some(id) = self.playlist.current() {
                    self.play_entry(id);
                }
            }
            PlayState::Stopped => {
                let id = self
                    .playlist
                    .current()
                    .or_else(|| self.playlist.entries().first().map(|e| e.id));
                if let Some(id) = id {
                    self.play_entry(id);
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
                    Some(id) => self.play_entry(id),
                    None => self.play(),
                }
            } else {
                self.pending_play = Some(p);
            }
        }
        if let Some(meta) = &self.meta {
            let results = meta.poll();
            let got = !results.is_empty();
            for r in results {
                match r {
                    MetaResult::Info(id, i) => {
                        self.playlist
                            .set_info(id, i.title, i.artist, i.duration_secs)
                    }
                    MetaResult::Failed(id) => self.playlist.set_failed(id),
                }
            }
            if got {
                self.playlist_dirty.get_or_insert_with(Instant::now);
            }
        }
        let queue = self.queue.clone();
        let EngineSlot::Ready(engine) = &mut self.engine else {
            return;
        };
        for ev in engine.poll_events() {
            match ev {
                EngineEvent::TrackInfo { index, info, .. } => {
                    if let Some(&id) = queue.get(index) {
                        self.playlist.set_info(
                            id,
                            info.title.clone(),
                            info.artist.clone(),
                            info.duration_secs,
                        );
                    }
                }
                EngineEvent::TrackFailed { index, .. } => {
                    if let Some(&id) = queue.get(index) {
                        self.playlist.set_failed(id);
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
        if let Some(tap) = &mut self.tap {
            while let Some(chunk) = tap.pop() {
                self.analyzer.push(&chunk);
            }
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
        {
            self.playlist.set_current(Some(id));
        }
    }

    // ---- files -----------------------------------------------------------------------------

    fn add_paths(&mut self, paths: Vec<PathBuf>, play_first: bool) {
        let found = files::expand(&paths);
        if found.is_empty() {
            return;
        }
        let was_empty = self.playlist.is_empty();
        let added = self.playlist.add(
            found
                .into_iter()
                .map(|p| TrackRef::new(p.to_string_lossy())),
        );
        let first = added.first().map(|(id, _)| *id);
        if let Some(m) = &self.meta {
            m.request(added);
        }
        self.mark_playlist();
        if (play_first || was_empty)
            && let Some(id) = first
        {
            self.play_entry(id);
        }
    }

    fn open_files_dialog(&mut self, play: bool) {
        let exts: Vec<&str> = files::AUDIO_EXTENSIONS
            .iter()
            .copied()
            .chain(["m3u", "m3u8"])
            .collect();
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Audio", &exts)
            .pick_files()
        {
            self.add_paths(paths, play);
        }
    }

    fn open_folder_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.add_paths(vec![dir], false);
        }
    }

    fn export_m3u(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Playlist", &["m3u8", "m3u"])
            .set_file_name("playlist.m3u8")
            .save_file()
        else {
            return;
        };
        let entries: Vec<files::M3uEntry> = self
            .playlist
            .entries()
            .iter()
            .map(|e| files::M3uEntry {
                path: e.track.0.clone().into(),
                title: Some(e.display_name()),
                duration: e.duration,
            })
            .collect();
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
            Key::Delete | Key::Backspace => {
                if self.playlist.remove_selected() > 0 {
                    self.mark_playlist();
                }
            }
            Key::O if mods.command => self.open_files_dialog(false),
            Key::A if mods.command => self.playlist.select_all(),
            Key::Enter => {
                if let Some(&id) = self.playlist.selected_ids().first() {
                    self.play_entry(id);
                }
            }
            _ => {}
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
        let (artist, title, duration) = match &self.now_playing {
            Some(i) => (i.artist.clone(), i.title.clone(), i.duration_secs),
            None => (String::new(), "Nothing playing".into(), None),
        };
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
            let number = self.playlist.current_index().map_or(0, |i| i + 1);
            let title = match &info {
                Some(i) => format::title_line(number, &i.artist, &i.title, i.duration_secs),
                None => self
                    .playlist
                    .current()
                    .and_then(|id| self.playlist.get(id))
                    .map(|e| format::title_line(number, &e.artist, &e.title, e.duration))
                    .unwrap_or_else(|| engine_status(&self.engine)),
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
        let len = self.playlist.len();
        self.pl_scroll = self.pl_scroll.min(len.saturating_sub(rows));
        {
            let def = self.def.clone();
            let sk = self.skinned(&def, ui, origin);
            let scale = sk.scale;
            sk.sprite("pl_top", 0.0, 0.0);
            Self::titlebar_drag(ui, &sk, "pl_title", "pl_titlebar");
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
            for r in 0..rows {
                let idx = self.pl_scroll + r;
                let Some(e) = self.playlist.entries().get(idx) else {
                    break;
                };
                let rr = sk.rect(l.x as f32, top + r as f32 * row_h, l.w as f32, row_h);
                if self.playlist.is_selected(e.id) {
                    clip.rect_filled(rr, 0.0, color(d.colors.pl_selected_bg));
                }
                let current = self.playlist.current() == Some(e.id);
                let mut col = color(if current {
                    d.colors.pl_current
                } else {
                    d.colors.pl_text
                });
                if e.status == crate::playlist::EntryStatus::Failed {
                    col = Color32::from_rgb(170, 60, 60);
                }
                let dur = e.duration.map(format::clock).unwrap_or_default();
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
                if resp.double_clicked() {
                    actions.push(Action::PlayEntry(e.id));
                } else if resp.clicked() {
                    actions.push(Action::Select(idx, mods));
                }
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
                if ui.button("Clear playlist").clicked() {
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
            });

            // "selected/total" time, Winamp style.
            let (total, t_unknown) = self.playlist.total_duration();
            let (seltime, s_unknown) = self.playlist.selected_duration();
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

    fn preset_dialog(&mut self, ctx: &egui::Context) {
        let Some(name) = &mut self.preset_name else {
            return;
        };
        let mut done = None;
        egui::Window::new("Save EQ preset")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let r = ui.text_edit_singleline(name);
                r.request_focus();
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked()
                        || (r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    {
                        done = Some(true);
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        done = Some(false);
                    }
                });
            });
        match done {
            Some(true) => {
                let name = self.preset_name.take().unwrap_or_default();
                if !name.trim().is_empty() {
                    self.presets
                        .save(EqPreset::from_settings(name.trim(), &self.settings.eq));
                    self.save_presets();
                }
            }
            Some(false) => self.preset_name = None,
            None => {}
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
            Action::Prev => self.with_engine(|e| e.previous()),
            Action::Play => self.play(),
            Action::Pause => self.with_engine(|e| e.toggle_pause()),
            Action::Stop => self.with_engine(|e| e.stop()),
            Action::Next => self.with_engine(|e| e.next()),
            Action::Eject => self.open_files_dialog(true),
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
            Action::ToggleScale => {
                self.settings.scale = if self.settings.scale >= 2 { 1 } else { 2 };
                self.mark_settings();
            }
            Action::Rows(d) => {
                self.settings.playlist_rows =
                    (self.settings.playlist_rows as i32 + d).clamp(4, 60) as u16;
                self.mark_settings();
            }
            Action::SavePreset => self.preset_name = Some(String::new()),
            Action::DeletePreset(name) => {
                self.presets.remove(&name);
                self.save_presets();
            }
            Action::PlayEntry(id) => self.play_entry(id),
            Action::Select(i, m) => self.playlist.click(i, m),
            Action::Move(from, to) => {
                self.playlist.move_entry(from, to);
                self.mark_playlist();
            }
            Action::AddFiles => self.open_files_dialog(false),
            Action::AddFolder => self.open_folder_dialog(),
            Action::RemoveSelected => {
                if self.playlist.remove_selected() > 0 {
                    self.mark_playlist();
                }
            }
            Action::Clear => {
                self.playlist.clear();
                self.with_engine(|e| e.stop());
                self.mark_playlist();
            }
            Action::SelectAll => self.playlist.select_all(),
            Action::SelectNone => self.playlist.select_none(),
            Action::InvertSelection => self.playlist.invert_selection(),
            Action::ExportM3u => self.export_m3u(),
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
        if due(self.playlist_dirty) {
            self.playlist_dirty = None;
            if let Err(e) = store.save(PLAYLIST_FILE, &self.playlist.to_saved()) {
                self.notify(format!("Could not save playlist: {e}"));
            }
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
    Rows(i32),
    SavePreset,
    DeletePreset(String),
    PlayEntry(EntryId),
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
            self.add_paths(dropped, false);
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
        if self.settings_dirty.is_some() || self.playlist_dirty.is_some() {
            ctx.request_repaint_after(SAVE_DELAY);
        }
        self.save_now(false);
        if let Some(p) = &mut self.profile {
            p.logic_end();
        }
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
            if let Some((text, at)) = &self.message {
                if at.elapsed() < Duration::from_secs(4) {
                    egui::Tooltip::always_open(
                        ctx.clone(),
                        ui.layer_id(),
                        Id::new("msg"),
                        egui::PopupAnchor::Position(origin),
                    )
                    .show(|ui| ui.label(text.as_str()));
                } else {
                    self.message = None;
                }
            }
            self.preset_dialog(&ctx);
        }
        if self.help {
            crate::help::show(&ctx, &mut self.help);
        }
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
