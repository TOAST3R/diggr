//! The eframe application: main player, equalizer and playlist stacked in one borderless
//! window.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use audio::{Engine, EngineEvent, EqPreset, EqPresets, PlayState, Position, TapReader, TrackInfo};
use egui::{
    Color32, Id, Key, Modifiers, Pos2, Rect, Sense, TextureHandle, Ui, ViewportCommand, pos2, vec2,
};
use platform::{FileSource, Spawner, TrackRef};

use crate::columns::{Col, Dir, Field};
use crate::crates::{CrateId, Crates, MetaKey, PLAYLIST};
use crate::eqcurve;
use crate::files;
use crate::format;
use crate::metadata::{MetaResult, MetaWorker};
use crate::playlist::{AlbumKey, ClickMods, CursorMove, EntryId, EntryStatus, Facet};
use crate::records::{self, ListRow};
use crate::settings::{PRESETS_FILE, Repeat, SETTINGS_FILE, Settings, SideGroup, Store};
use crate::skin::LoadedSkin;
use crate::sync::speed_label;
use crate::widgets::{self, Skinned, SliderSprites, color};

#[cfg(not(target_arch = "wasm32"))]
mod bandcamp;
mod card;
#[cfg(not(target_arch = "wasm32"))]
mod covers;
#[cfg(not(target_arch = "wasm32"))]
mod digging;
#[cfg(not(target_arch = "wasm32"))]
mod fan;
#[cfg(not(target_arch = "wasm32"))]
mod friends;
#[cfg(not(target_arch = "wasm32"))]
mod mixes;
#[cfg(not(target_arch = "wasm32"))]
mod not_found;
#[cfg(not(target_arch = "wasm32"))]
mod sellers;
#[cfg(not(target_arch = "wasm32"))]
mod stores;
#[cfg(not(target_arch = "wasm32"))]
mod welcome;
#[cfg(not(target_arch = "wasm32"))]
pub use digging::{BridgeSetup, DigAction, DigSetup, SendMode};

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
    pub startup: Startup,
    /// Music analysis ahead of the playhead (beats, sections) for BPM, the waveform and the
    /// structure keys.
    pub analysis: Option<analysis::AnalysisService>,
    /// Native-rate track overviews for the waveform section.
    pub overviews: Option<analysis::overview::OverviewService>,
    /// Digging Discogs pages into crates of previews; `None` turns it off.
    #[cfg(not(target_arch = "wasm32"))]
    pub dig: Option<DigSetup>,
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

pub struct DiggrApp {
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
    meta: Option<MetaWorker<MetaKey>>,
    now_playing: Option<TrackInfo>,
    position: Position,
    /// Where the player's seek bar is being dragged (0..1), until the release seeks there.
    seek_drag: Option<f32>,
    /// A seek sent from the bar (where to, and when): the thumb stays there until the clock
    /// reports it, instead of jumping back to the old position meanwhile.
    seek_pending: Option<(f32, Instant)>,
    /// When SYNC last changed the playing track's speed (changes come at most every 30 ms).
    sync_applied: Instant,
    /// Scrolling over the pitch fader not yet worth a whole BPM, in skin pixels.
    sync_drag: f32,
    /// The turntable's platter angle and strobe drift, in radians.
    deck_turn: f32,
    deck_strobe: f32,
    /// The rim's lock flash, started when a record becomes fitted.
    deck_flash: Option<Instant>,
    /// The digits counting from a new track's own BPM to the target (from, to, since).
    deck_count: Option<(u16, u16, Instant)>,
    /// Who played and whether it was locked on the last frame, to see a record become fitted.
    deck_seen: (Option<(CrateId, crate::playlist::EntryId)>, bool),
    /// A different track began and hasn't been fitted yet (see `deck_section`).
    deck_new: bool,
    /// The LP knob, 0..=1 (1 is off); not remembered, so every launch starts with it off.
    lp_knob: f32,
    title_offset: usize,
    title_tick: f64,
    /// The first row in view (a record row counts as one).
    pl_scroll: usize,
    pl_drag_from: Option<usize>,
    /// The record card open in a grouped crate (see `app/card.rs`), and the card's rect last
    /// drawn (a click outside it closes it).
    card: Option<OpenCard>,
    card_rect: Option<Rect>,
    /// A record row being dragged: all its entries move.
    pl_drag_block: Option<Vec<EntryId>>,
    /// A track row of an open record being dragged: it moves within that record only.
    pl_drag_track: Option<AlbumKey>,
    /// The records open in each grouped crate (for the session).
    pl_open: HashMap<CrateId, HashSet<AlbumKey>>,
    /// Bumped when a record opens or closes.
    pl_open_rev: u64,
    /// The shown crate's rows, and what they were built from.
    pl_rows_cache: RefCell<Option<(RowsKey, Rc<Vec<ListRow>>)>>,
    /// The rest of the album of the entry whose menu is open (tinted; the selection stays).
    pl_tint: Vec<EntryId>,
    /// The empty crate's hint links as last drawn (label, where), for tests to click.
    pl_hint_links: Vec<(&'static str, Rect)>,
    /// The BPM range handle being dragged: 0 the low one, 1 the high one.
    bpm_drag: Option<u8>,
    /// The crate last clicked in the sidebar, until the list is clicked: Delete deletes it.
    side_crate: Option<CrateId>,
    /// The sidebar lists' first rows: the crates (or the whole sidebar when it scrolls as one),
    /// the open Discogs group, and the CRATES panel; with their trackpad remainders.
    side_scroll: [usize; 3],
    side_scroll_acc: [f32; 3],
    /// The shown crate last followed (none before the first frame), and whether the sidebar
    /// and the CRATES panel still have to scroll to it.
    side_revealed: Option<CrateId>,
    side_scroll_to: [bool; 2],
    /// When a drag at a list's edge last stepped it.
    side_edge_at: Option<Instant>,
    /// The CRATES panel's crate under the pointer while entries are dragged.
    /// Where the footer's BPM slider was last drawn: its track's ends and whether the range
    /// text fit (skin pixels, footer coordinates), for tests.
    #[cfg(test)]
    bpm_slider: Option<(f32, f32, bool)>,
    /// The shown crate's values per filter (style, artist, label) with their record counts,
    /// and what they were counted from.
    facet_cache: [Option<(CrateId, u64, FacetCounts)>; 5],
    /// What each filter's list is narrowed to.
    facet_search: [String; 5],
    /// "Records leave this crate with…" was said this session.
    discogs_hint_shown: bool,
    /// Where the footer's filters were last drawn, and how, for tests.
    #[cfg(test)]
    filters_drawn: Option<(f32, FooterFilters)>,
    /// The CART switch as last drawn (its label and where, in points), for tests.
    #[cfg(test)]
    cart_drawn: Option<(String, Pos2)>,
    /// The filter bar's time readout as last drawn (its text, x and width in skin pixels).
    #[cfg(test)]
    readout_drawn: Option<(String, f32, f32)>,
    /// The turntable as last drawn: its state line, its big digits and the meter needle's
    /// place (-1 to 1, `None` without one).
    #[cfg(test)]
    deck_drawn: Option<(String, String, Option<f32>)>,
    /// Where the seek bar's thumb was last drawn (0..1), `None` while it is inert.
    #[cfg(test)]
    seek_drawn: Option<f32>,
    /// The filter bar's search text as typed (the shown crate holds its folded words).
    pl_search: String,
    /// Cmd+F was pressed: the search field takes the keyboard on the next frame.
    search_focus: bool,
    /// The filter panel's tab.
    panel_tab: Facet,
    /// The year last clicked in the YEAR grid: a Shift-click picks the run from it.
    year_anchor: Option<u16>,
    /// Scrolling not yet enough for a whole row, in points.
    pl_scroll_acc: f32,
    /// The window's content size in skin pixels (the maximized layout follows it).
    win_px: egui::Vec2,
    /// The OS has reported the window maximized since the playlist was maximized (so a
    /// later "not maximized" means it was restored behind our back).
    /// Which side the arrow keys work on.
    focus: Focus,
    /// The shown crate's playing entry when the list last looked (see `follow_playing_entry`).
    pl_follow: Option<(CrateId, EntryId)>,
    /// The playlist width has been fitted to the monitor.
    name_dialog: Option<NameDialog>,
    /// A crate with entries waiting for "Delete crate…" to be confirmed.
    confirm_delete: Option<CrateId>,
    last_frame: Instant,
    startup: Startup,
    first_frame_done: bool,
    /// Removes the old visuals folder after the first frame (see [`Store::remove_old_visuals`]).
    cleanup: Option<(Arc<dyn Spawner>, Store)>,
    /// The shortcuts help panel (`H` / `F1`).
    help: bool,
    overviews: Option<analysis::overview::OverviewService>,
    /// The audible track's overview, as far as it is built.
    overview: Option<Arc<analysis::overview::Overview>>,
    nav: Nav,
    /// The spectrogram window (`S`) and whether it is open.
    spectro: crate::spectrogram::SpectrogramWindow,
    spectro_open: bool,
    /// The Insights window, and the collection's numbers it shows: for this crate at this
    /// revision.
    insights_open: bool,
    insights: Option<(CrateId, u64, Rc<crate::insights::Stats>)>,
    /// When the Insights window opened (egui time): its bars grow in from then.
    insights_opened: Option<f64>,
    /// `DIGGR_FRAME_STATS=1`: per-second frame phase timings on stderr and in
    /// `$TMPDIR/diggr_frame_stats.log`.
    profile: Option<FrameProfile>,
    /// `DIGGR_AUTO_QUIT_SECS=n`: close the app after n seconds.
    auto_quit: Option<Instant>,
    settings_dirty: Option<Instant>,
    message: Option<(String, Instant)>,
    /// A verdict just landed: the title line shows it for [`FLASH_SECS`] (WANTED, PASS, …).
    flash: Option<(String, Instant)>,
    analysis: Option<analysis::AnalysisService>,
    /// Engine track instances → files, from `TrackLoaded` events.
    track_refs: std::collections::HashMap<audio::TrackId, TrackRef>,
    score: Option<Arc<analysis::SongScore>>,
    /// The last playing track whose tempo was given to its entries.
    bpm_track: Option<TrackRef>,
    #[cfg(not(target_arch = "wasm32"))]
    dig: Option<digging::Dig>,
}

const SAVE_DELAY: Duration = Duration::from_millis(800);

impl DiggrApp {
    pub fn new(cc: &eframe::CreationContext<'_>, ctx: AppContext) -> Self {
        Self::build(cc.egui_ctx.clone(), ctx)
    }

    fn build(egui_ctx: egui::Context, ctx: AppContext) -> Self {
        let store = ctx.store;
        let cleanup = store.clone().map(|s| (ctx.spawner.clone(), s));
        let settings = store.as_ref().map(Store::load_settings).unwrap_or_default();
        let presets = store.as_ref().map(Store::load_presets).unwrap_or_default();
        let mut crates = store.as_ref().map_or_else(Crates::in_memory, Crates::open);

        // Cmd+= / Cmd+− / Cmd+0 are the size keys (1× or 2×). egui's own keyboard zoom would
        // scale the pixel skin by fractions and blur it.
        egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
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
        let scores = ctx.analysis.as_ref().and_then(|a| a.cache().cloned());
        let meta = MetaWorker::start(&*ctx.spawner, ctx.files.clone(), scores, move || {
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
        #[cfg(not(target_arch = "wasm32"))]
        let dig = ctx.dig.map(|setup| {
            let mut d = digging::Dig::new(
                setup,
                ctx.spawner.clone(),
                ctx.files.clone(),
                store.as_ref().map(|s| s.dir().to_path_buf()),
                egui_ctx.clone(),
            );
            sellers::seed_featured(&mut crates, &mut d);
            d
        });
        let skin = LoadedSkin::default_skin();
        Self {
            def: Arc::new(skin.def.clone()),
            skin,
            tex: None,
            store,
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
                speed: 1.0,
            },
            seek_drag: None,
            seek_pending: None,
            sync_applied: Instant::now(),
            sync_drag: 0.0,
            deck_turn: 0.0,
            deck_strobe: 0.0,
            deck_flash: None,
            deck_count: None,
            deck_seen: (None, false),
            deck_new: false,
            lp_knob: audio::filter::OFF,
            title_offset: 0,
            title_tick: 0.0,
            pl_scroll: 0,
            pl_drag_from: None,
            card: None,
            card_rect: None,
            pl_drag_block: None,
            pl_drag_track: None,
            pl_open: HashMap::new(),
            pl_open_rev: 0,
            pl_rows_cache: RefCell::new(None),
            pl_tint: Vec::new(),
            pl_hint_links: Vec::new(),
            bpm_drag: None,
            side_crate: None,
            side_scroll: [0; 3],
            side_scroll_acc: [0.0; 3],
            side_revealed: None,
            side_scroll_to: [false; 2],
            side_edge_at: None,
            #[cfg(test)]
            bpm_slider: None,
            facet_cache: Default::default(),
            facet_search: Default::default(),
            discogs_hint_shown: false,
            #[cfg(test)]
            filters_drawn: None,
            #[cfg(test)]
            cart_drawn: None,
            #[cfg(test)]
            readout_drawn: None,
            #[cfg(test)]
            deck_drawn: None,
            #[cfg(test)]
            seek_drawn: None,
            pl_search: String::new(),
            search_focus: false,
            panel_tab: Facet::Style,
            year_anchor: None,
            pl_scroll_acc: 0.0,
            win_px: egui::Vec2::ZERO,
            focus: Focus::Player,
            pl_follow: None,
            name_dialog: None,
            confirm_delete: None,
            last_frame: Instant::now(),
            startup: ctx.startup,
            first_frame_done: false,
            cleanup,
            profile: std::env::var_os("DIGGR_FRAME_STATS").map(|_| FrameProfile::new()),
            auto_quit: std::env::var("DIGGR_AUTO_QUIT_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .map(|s| Instant::now() + Duration::from_secs(s)),
            settings_dirty: None,
            message,
            flash: None,
            analysis: ctx.analysis,
            track_refs: std::collections::HashMap::new(),
            score: None,
            bpm_track: None,
            help: false,
            overviews: ctx.overviews,
            overview: None,
            nav: Nav::default(),
            spectro,
            spectro_open: false,
            insights_open: false,
            insights: None,
            insights_opened: None,
            #[cfg(not(target_arch = "wasm32"))]
            dig,
        }
    }

    /// The window's size in points when it isn't maximized: a playlist wide enough for its
    /// sidebar and columns, with 20 rows under the band.
    pub fn window_size(settings: &Settings) -> egui::Vec2 {
        vec2(1100.0, 600.0) * settings.scale as f32 / 2.0
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

    /// A tempo is known for this audio: every entry with the same audio (the same clip, or the
    /// same file), in every loaded crate, shows it.
    fn set_track_bpm(&mut self, track: &TrackRef, raw_bpm: f64) {
        let Some(bpm) = format::dj_bpm(raw_bpm) else {
            return;
        };
        let ids = self.crates.loaded_ids();
        let keys: std::collections::HashSet<_> = ids
            .iter()
            .filter_map(|&c| self.crates.get(c))
            .flat_map(|p| p.entries().iter().filter(|e| e.track == *track))
            .map(crate::playlist::Entry::duplicate_key)
            .collect();
        for c in ids {
            let changed = self
                .crates
                .get_mut(c)
                .is_some_and(|p| keys.iter().fold(false, |ch, k| p.set_bpm(k, bpm) | ch));
            // Saved soon. The play order changes only when a BPM filter decides what plays.
            if changed {
                let filtered = self.crates.get(c).is_some_and(|p| p.is_filtered());
                if filtered {
                    self.mark_crate(c);
                } else {
                    self.crates.touch(c);
                }
            }
        }
    }

    /// Once the playing track's score is whole (at once for a cached one), its entries show
    /// its tempo. Done once per track.
    fn show_playing_bpm(&mut self) {
        if self.position.state == PlayState::Stopped {
            return;
        }
        let Some(track) = self.track_refs.get(&self.position.track) else {
            return;
        };
        if self.bpm_track.as_ref() == Some(track) {
            return;
        }
        let Some(bpm) = self
            .score
            .as_ref()
            .filter(|s| s.complete)
            .map(|s| s.dominant_bpm())
        else {
            return;
        };
        let track = track.clone();
        self.bpm_track = Some(track.clone());
        if let Some(bpm) = bpm {
            self.set_track_bpm(&track, bpm);
        }
    }

    /// The playing entry's BPM, if known.
    fn playing_bpm(&self) -> Option<u16> {
        let playing = self.crates.playing();
        playing.current().and_then(|id| playing.get(id))?.bpm
    }

    /// SYNC: sets the speed of the playing track and its neighbours in the queue from their
    /// BPM (an entry started anywhere else gets its speed in [`Self::play_entry`]); all 1 when
    /// off. Returns whether a change to the playing track waits for its 30 ms.
    fn apply_sync(&mut self) -> bool {
        let sync = self.settings.sync;
        let (Some(p), EngineSlot::Ready(engine)) =
            (self.crates.get(self.queue_crate), &mut self.engine)
        else {
            return false;
        };
        let queue = &self.queue;
        let speed = |i: usize| {
            let bpm = queue.get(i).and_then(|&id| p.get(id)).and_then(|e| e.bpm);
            sync.speed(bpm).unwrap_or(1.0)
        };
        let Some(cur) = engine.current_index() else {
            return false;
        };
        // The next one before it starts (gapless at its speed), the first for REPEAT ALL, and
        // the one before for Z.
        for i in [cur + 1, 0, cur.wrapping_sub(1)] {
            if i < queue.len() && i != cur {
                engine.set_speed(i, speed(i));
            }
        }
        let s = speed(cur);
        if engine.speed(cur) == s {
            return false;
        }
        if self.sync_applied.elapsed() < Duration::from_millis(30) {
            return true;
        }
        engine.set_speed(cur, s);
        self.sync_applied = Instant::now();
        false
    }

    /// Turns SYNC on (at the playing track's tempo) or off. A playing track whose BPM isn't
    /// known yet is analyzed next, and SYNC waits for its BPM.
    fn toggle_sync(&mut self) {
        let sync = &mut self.settings.sync;
        sync.on = !sync.on;
        sync.pending = None;
        if sync.on {
            match self.playing_bpm() {
                Some(bpm) => self.settings.sync.target = bpm,
                // (A whole score without a tempo has nothing more to give.)
                None if self.position.state != PlayState::Stopped
                    && !self.score.as_ref().is_some_and(|s| s.complete) =>
                {
                    #[cfg(not(target_arch = "wasm32"))]
                    let wait = self.dig_analyze_playing();
                    #[cfg(target_arch = "wasm32")]
                    let wait = false;
                    if wait {
                        self.settings.sync.pending = Some(self.position.track);
                    }
                }
                None => {}
            }
        }
        let sync = self.settings.sync;
        let range = if sync.wide { 16 } else { 8 };
        self.notify(match (sync.on, sync.pending) {
            (false, _) => "SYNC off: every track plays at its own tempo".to_owned(),
            (true, Some(_)) => {
                "SYNC on: every track will play at this one's BPM once it's analyzed".to_owned()
            }
            (true, None) => format!(
                "SYNC on: every track in the list plays at {} BPM (within ±{range}%)",
                sync.target
            ),
        });
        self.mark_settings();
    }

    /// SYNC waiting for the playing track's BPM takes it once known, and stops waiting
    /// (keeping the target) when another track plays or playback stops.
    fn sync_wait(&mut self) {
        let sync = self.settings.sync;
        let Some(track) = sync.pending else { return };
        if !sync.on || self.position.state == PlayState::Stopped || self.position.track != track {
            self.settings.sync.pending = None;
        } else if let Some(bpm) = self.playing_bpm() {
            self.settings.sync.target = bpm;
            self.settings.sync.pending = None;
            self.mark_settings();
        }
    }

    /// While SYNC waits for a BPM its target stays put: says so.
    fn sync_waiting(&mut self) -> bool {
        let waiting = self.settings.sync.pending.is_some();
        if waiting {
            self.notify("Analyzing the BPM, wait a moment");
        }
        waiting
    }

    fn nudge_sync(&mut self, bpm: i32) {
        if self.sync_waiting() {
            return;
        }
        self.settings.sync.nudge(bpm);
        self.mark_settings();
    }

    fn mark_shown(&mut self) {
        self.mark_crate(self.crates.shown_id());
    }

    fn notify(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), Instant::now()));
    }

    /// Shows `verdict` on the title line for a moment, for `count` records.
    fn flash(&mut self, verdict: Verdict, count: usize) {
        if count > 0 {
            self.flash = Some((flash_text(verdict, count), Instant::now()));
        }
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
                engine.set_eq(s.eq);
                engine.set_repeat(s.repeat.to_engine());
                engine.set_av_offset_ms(s.av_offset_ms);
                self.tap = engine.take_tap();
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

    /// The next playable entry, skipping those waiting for their audio. When only waiting ones
    /// follow, the first is armed instead, and the current track plays on until it arrives.
    fn play_next(&mut self) {
        let p = self.crates.playing();
        let order = p.play_order(self.settings.shuffle, p.current(), self.shuffle_seed);
        let after = p
            .current()
            .and_then(|id| order.iter().position(|&o| o == id))
            .map_or(&[][..], |i| &order[i + 1..]);
        let status = |id: &EntryId| p.get(*id).map(|e| &e.status);
        let waiting = if after
            .iter()
            .any(|id| status(id).is_some_and(EntryStatus::is_playable))
        {
            None
        } else {
            after
                .iter()
                .copied()
                .find(|id| matches!(status(id), Some(EntryStatus::Waiting(_))))
        };
        if let Some(id) = waiting {
            self.armed = Some((self.crates.playing_id(), id));
        } else {
            self.armed = None;
            self.with_engine(|e| e.next());
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
                // Its preview isn't there: with the guided setup, while yt-dlp is missing, the
                // question opens.
                #[cfg(not(target_arch = "wasm32"))]
                if digging::needs_ytdlp(entry) {
                    self.dig_wants_ytdlp();
                }
                self.armed = Some((crate_id, id));
                return;
            }
            EntryStatus::Unavailable(_) => {
                // A clip that failed or a track not found may work now: try it again first.
                #[cfg(not(target_arch = "wasm32"))]
                if self.dig_retry_entry(crate_id, id) {
                    self.armed = Some((crate_id, id));
                    return;
                }
                let Some(entry) = self.crates.get(crate_id).and_then(|p| p.get(id)) else {
                    return;
                };
                let EntryStatus::Unavailable(reason) = &entry.status else {
                    return;
                };
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
            let bpm = self.crates.playing().get(id).and_then(|e| e.bpm);
            let speed = self.settings.sync.speed(bpm).unwrap_or(1.0);
            if let Some(e) = self.engine() {
                e.set_speed(q, speed);
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

    fn update_audio(&mut self) {
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
                    MetaResult::Info(key, _)
                    | MetaResult::Failed(key)
                    | MetaResult::Bpm(key, _) => key,
                };
                let Some(playlist) = self.crates.get_mut(c) else {
                    continue; // deleted meanwhile
                };
                match r {
                    MetaResult::Info(_, i) => playlist.set_info(id, i),
                    MetaResult::Failed(_) => {
                        if playlist.set_failed(id) {
                            self.mark_crate(c);
                        }
                    }
                    MetaResult::Bpm(_, bpm) => {
                        if let Some(key) = playlist.get(id).map(|e| e.duplicate_key())
                            && let Some(bpm) = format::dj_bpm(bpm)
                        {
                            playlist.set_bpm(&key, bpm);
                        }
                    }
                }
                // A tempo may decide what plays under a BPM filter.
                if self.crates.get(c).is_some_and(|p| p.is_filtered()) {
                    self.mark_crate(c);
                } else {
                    self.crates.touch(c);
                }
            }
        }
        let messages = self.crates.take_messages();
        if !messages.is_empty() {
            self.notify(messages.join("\n"));
        }
        self.check_armed();
        let queue = self.queue.clone();
        let queue_crate = self.queue_crate;
        let mut requeued = None;
        let EngineSlot::Ready(engine) = &mut self.engine else {
            return;
        };
        for ev in engine.poll_events() {
            match ev {
                EngineEvent::TrackInfo { index, info, .. } => {
                    if let (Some(&id), Some(p)) =
                        (queue.get(index), self.crates.get_mut(queue_crate))
                    {
                        p.set_info(id, info.clone());
                    }
                }
                EngineEvent::TrackFailed { index, .. } => {
                    if let (Some(&id), Some(p)) =
                        (queue.get(index), self.crates.get_mut(queue_crate))
                        && p.set_failed(id)
                    {
                        requeued = Some(id);
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
            // Drained even when nothing reads it, so the ring never fills.
            while let Some(chunk) = tap.pop() {
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
        // A track that was to play but whose preview is gone plays once downloaded again.
        if let Some(id) = requeued {
            self.armed = Some((queue_crate, id));
            self.mark_crate(queue_crate);
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
            // Not found or failed: the record's next track that may still play is armed, so
            // the search goes on down the record; removed, or none left, it is forgotten.
            self.armed = self.next_in_record(c, id).map(|next| (c, next));
            if let Some((c, next)) = self.armed {
                let title = self.crates.get(c).and_then(|p| p.get(id));
                if let Some(t) = title.map(|e| e.display_name()) {
                    self.notify(format!("{t}: no preview, trying the next track"));
                }
                self.check_armed_next(c, next);
            }
        }
    }

    /// An entry armed in place of one that won't play starts at once if it is ready.
    fn check_armed_next(&mut self, c: CrateId, id: EntryId) {
        let ready = self
            .crates
            .get(c)
            .and_then(|p| p.get(id))
            .is_some_and(|e| e.status.is_playable());
        if ready && matches!(self.engine, EngineSlot::Ready(_)) {
            self.play_entry(c, id);
        }
    }

    /// The entry after `id` in its crate, of the same record, that plays or may still play.
    fn next_in_record(&self, c: CrateId, id: EntryId) -> Option<EntryId> {
        let p = self.crates.get(c)?;
        let key = p.get(id)?.album_key()?;
        let at = p.index_of(id)?;
        p.entries()[at + 1..]
            .iter()
            .find(|e| e.album_key().as_ref() == Some(&key) && e.status.in_play_order())
            .map(|e| e.id)
    }

    /// "Waiting for ‹title› (downloading 40%)" while an entry is armed.
    /// Digging progress for the status line.
    fn dig_line(&self) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        return self.dig.as_ref().and_then(|d| d.line());
        #[cfg(target_arch = "wasm32")]
        None
    }

    /// The tracks being analyzed for their BPM and the share of each done.
    fn bpm_progress(&self) -> std::collections::HashMap<TrackRef, f32> {
        #[cfg(not(target_arch = "wasm32"))]
        return self
            .dig
            .as_ref()
            .map(|d| d.prepare_progress())
            .unwrap_or_default();
        #[cfg(target_arch = "wasm32")]
        Default::default()
    }

    /// The playing record's BPM is on its way: queued or being analyzed.
    fn playing_bpm_coming(&self) -> bool {
        let p = self.crates.playing();
        let progress = self.bpm_progress();
        self.position.state != PlayState::Stopped
            && p.current()
                .and_then(|id| p.get(id))
                .is_some_and(|e| self.bpm_wait(e, &progress).is_some())
    }

    /// Where an entry with no BPM stands in the background analysis; `None` once it has one
    /// (even while analyzed again), or when it isn't in the queue.
    fn bpm_wait(
        &self,
        e: &crate::playlist::Entry,
        progress: &std::collections::HashMap<TrackRef, f32>,
    ) -> Option<BpmWait> {
        if e.bpm.is_some() {
            return None;
        }
        if let Some(&share) = progress.get(&e.track) {
            return Some(BpmWait::Analyzing(share));
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(d) = &self.dig
            && !d.analyzed.contains(&e.track)
            && d.prepared_list.contains(&e.track)
        {
            return Some(BpmWait::Queued);
        }
        None
    }

    /// Kept, passed and wantlist-pending marks for a playlist row.
    /// "this pressing", or "another pressing (AF014, 2018)", when the entry's record is in
    /// the user's collection.
    /// What of the entry's record the user owns: "this pressing", "another pressing (…)".
    fn dig_owned(&self, e: &crate::playlist::Entry) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        return self.dig.as_ref()?.owned(e).map(|o| digging::owned_text(&o));
        #[cfg(target_arch = "wasm32")]
        {
            let _ = e;
            None
        }
    }

    /// Shown in the collection crate, or sent from a collection page: owned, whatever the
    /// synced collection says (and even without a token).
    fn in_collection_view(&self, e: &crate::playlist::Entry) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        let from_collection = e.origin.as_ref().is_some_and(|o| {
            ::dig::discogs::url::parse(&o.page)
                .is_ok_and(|p| matches!(p.kind, ::dig::discogs::url::PageKind::Collection(_)))
        });
        #[cfg(target_arch = "wasm32")]
        let from_collection = {
            let _ = e;
            false
        };
        from_collection || self.crates.is_collection(self.crates.shown_id())
    }

    /// Whether the user owns the entry's record, and which pressing ("this pressing",
    /// "another pressing (AF001R, 2019)", or "" in the collection view). Unlike the OWNED
    /// badge, this holds in the collection crate too: it decides whether a row may buy.
    fn owns(&self, e: &crate::playlist::Entry) -> Option<String> {
        if self.in_collection_view(e) {
            return Some(String::new());
        }
        self.dig_owned(e)
    }

    /// An entry's dig marks in the shown crate. In the collection's own crate (or entries
    /// sent from the collection page) everything is owned, so no OWNED badge there.
    fn dig_marks(&self, e: &crate::playlist::Entry) -> format::DigMarks {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(d) = &self.dig else {
                return format::DigMarks::default();
            };
            let m = d.marks(e);
            let badge = !self.in_collection_view(e);
            format::DigMarks {
                wanted: m.wanted,
                passed: m.passed,
                wantlist_pending: m.wantlist_pending,
                wantlist_failed: m.wantlist_failed,
                collection_failed: m.collection_failed,
                discard_pending: m.discard_pending,
                discard_failed: m.discard_failed,
                owned: if badge { self.dig_owned(e) } else { None },
                cart: e
                    .origin
                    .as_ref()
                    .and_then(|o| o.release)
                    .and_then(|r| d.cart.for_release(r))
                    .map(|i| format!("from {}, {}", i.seller, i.price)),
                copies: e.origin.as_ref().and_then(|o| o.release).and_then(|r| {
                    let copies = self.crates.shown().copies_of(r);
                    let text: Vec<String> = copies
                        .iter()
                        .map(|c| {
                            let sold = if c.sold { " (sold)" } else { "" };
                            format!("{} {}{sold}", format::price(c.cents, &c.currency), c.grades)
                        })
                        .collect();
                    (!text.is_empty()).then(|| text.join(", "))
                }),
                stores: self.store_notes(e),
                searching: e.status == EntryStatus::Waiting(crate::playlist::WaitKind::Search)
                    && e.origin
                        .as_ref()
                        .is_some_and(|o| d.search_now.as_deref() == Some(o.search_key.as_str())),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = e;
            format::DigMarks::default()
        }
    }

    fn armed_line(&self) -> Option<String> {
        let (c, id) = self.armed?;
        let e = self.crates.get(c)?.get(id)?;
        // Waiting on a paused source: say why, since the pause's own message is hidden.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(why) = self.waiting_why(e) {
            return Some(format!("Waiting for {}. {why}", e.display_name()));
        }
        Some(match e.status.note() {
            Some(note) => format!("Waiting for {} ({note})", e.display_name()),
            None => format!("Waiting for {}", e.display_name()),
        })
    }

    // ---- files -----------------------------------------------------------------------------

    /// Adds music to the shown crate (ADD, drag-and-drop, Cmd+O, M3U import). Adding to an
    /// empty crate plays it if nothing else is playing.
    fn add_paths(&mut self, paths: Vec<PathBuf>, open: Open) {
        let found: Vec<TrackRef> = files::expand(&paths)
            .into_iter()
            .map(|p| TrackRef::new(p.to_string_lossy()))
            .collect();
        if found.is_empty() {
            return;
        }
        if self.refuse_discogs_insert(self.crates.shown_id()) {
            return;
        }
        let crate_id = self.crates.shown_id();
        let was_empty = self.crates.shown().is_empty();
        let added = self.crates.shown_mut().add(found);
        let added: Vec<_> = added.into_iter().map(|(e, t)| ((crate_id, e), t)).collect();
        self.mark_shown();
        let idle = self.position.state == PlayState::Stopped;
        let play = open == Open::AddAndPlay || (was_empty && idle);
        let first = added.first().map(|((_, id), _)| *id);
        if let Some(m) = &self.meta {
            m.request(added);
        }
        if play && let Some(id) = first {
            self.play_entry(crate_id, id);
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

    /// Export M3U… for crate `id`, from the list menu (the shown crate) or the crate's own menu
    /// (any crate): it isn't shown, and playback and the selection are left alone.
    fn export_m3u(&mut self, id: CrateId) {
        if !self.crates.load(id) {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Playlist", &["m3u8", "m3u"])
            .set_file_name(format!("{}.m3u8", self.crates.name(id)))
            .save_file()
        else {
            return;
        };
        let Some(m3u) = self.m3u_of(id) else { return };
        match std::fs::write(&path, m3u) {
            Ok(()) => self.notify(format!("Saved {}", path.display())),
            Err(e) => self.notify(format!("Could not save: {e}")),
        }
    }

    /// Crate `id` as M3U8, loading it if needed (whole, whatever the filter shows); `None` when
    /// it can't be read.
    fn m3u_of(&mut self, id: CrateId) -> Option<String> {
        if !self.crates.load(id) {
            return None;
        }
        let entries = files::m3u_entries(self.crates.get(id)?);
        Some(files::write_m3u(&entries))
    }

    // ---- keyboard ----------------------------------------------------------------------------

    fn handle_keys(&mut self, ctx: &egui::Context) {
        // Tab switches between the player and the playlist. egui would otherwise move its
        // widget focus, and a focused widget holds the keyboard until Esc.
        if !ctx.text_edit_focused() && ctx.input(|i| i.key_pressed(Key::Tab) && !i.modifiers.any())
        {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            self.focus = self.focus.other();
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        // A question is open (keeping a record already owned): its own keys only.
        #[cfg(not(target_arch = "wasm32"))]
        if self.dig_asking() {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        for text in ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Paste(t) => Some(t.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        }) {
            self.dig_paste(&text);
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
        // Space is play/pause: taken before a focused button could read it as a click.
        ctx.input_mut(|i| {
            i.consume_key(Modifiers::NONE, Key::Space);
            i.consume_key(Modifiers::SHIFT, Key::Space);
        });
        for key in pressed {
            // Cmd+F (Ctrl+F) finds in the shown crate.
            if key == Key::F && mods.command {
                self.search_focus = true;
                continue;
            }
            // The help panel takes its own close keys first.
            if matches!(key, Key::H | Key::F1) && !mods.command {
                self.help = !self.help;
                continue;
            }
            if self.help && key == Key::Escape {
                self.help = false;
                continue;
            }
            // While a record card is open, Esc closes it and ← → step through the records (in
            // place of seeking, or opening and closing records).
            if self.card.is_some() && !mods.command && !mods.shift {
                match key {
                    Key::Escape => {
                        self.apply(Action::CloseCard, ctx);
                        continue;
                    }
                    Key::ArrowLeft | Key::ArrowRight => {
                        let step = if key == Key::ArrowLeft { -1 } else { 1 };
                        self.apply(Action::CardStep(step), ctx);
                        continue;
                    }
                    _ => {}
                }
            }
            if !self.transport_key(key, mods) {
                self.window_key(ctx, key, mods);
            }
        }
    }

    /// Playback, volume, structure and digging keys.
    fn transport_key(&mut self, key: Key, mods: Modifiers) -> bool {
        if mods.command {
            return false;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.dig_key(key) {
            return true;
        }
        let secs = self.position.seconds();
        match key {
            Key::Space if mods.shift => self.with_engine(|e| e.stop()),
            Key::Space if self.position.state == PlayState::Playing => {
                self.with_engine(|e| e.toggle_pause())
            }
            Key::Space => self.play(),
            // In a grouped playlist with the keyboard, the arrows open and close records.
            Key::ArrowLeft | Key::ArrowRight
                if self.arrows_move_cursor() && self.crates.shown().is_grouped() =>
            {
                return false;
            }
            Key::Z => self.with_engine(|e| e.previous()),
            Key::X => self.play(),
            Key::C => self.with_engine(|e| e.toggle_pause()),
            Key::V => self.with_engine(|e| e.stop()),
            Key::B => self.play_next(),
            Key::ArrowLeft => self.with_engine(|e| e.seek((secs - 5.0).max(0.0))),
            Key::ArrowRight => self.with_engine(|e| e.seek(secs + 5.0)),
            // With the playlist focused, the arrows move its cursor instead (`window_key`).
            Key::ArrowUp | Key::ArrowDown if self.arrows_move_cursor() => return false,
            Key::ArrowUp if mods.shift => self.nudge_sync(1),
            Key::ArrowDown if mods.shift => self.nudge_sync(-1),
            Key::ArrowUp => self.set_volume(self.settings.volume + 0.05),
            Key::ArrowDown => self.set_volume(self.settings.volume - 0.05),
            Key::CloseBracket if mods.shift => self.structure_jump(crate::navigation::next_rise),
            Key::CloseBracket => self.structure_jump(crate::navigation::next_section),
            Key::OpenBracket => self.structure_jump(crate::navigation::previous_section),
            Key::L if mods.shift => self.bar_loop(),
            Key::L => self.section_loop(),
            _ => return false,
        }
        true
    }

    fn window_key(&mut self, ctx: &egui::Context, key: Key, mods: Modifiers) {
        match key {
            Key::S if !mods.command => self.spectro_open = !self.spectro_open,
            // The size, on the keys every app zooms with: "+" has its own key on many
            // European layouts and "=" is shifted, so both mean double size.
            Key::Equals | Key::Plus if mods.command => self.apply(Action::SetScale(2), ctx),
            Key::Minus | Key::Num0 if mods.command => self.apply(Action::SetScale(1), ctx),
            // After a click on a crate in the sidebar, Delete deletes that crate (asking first
            // when it has entries); otherwise it removes the selected entries.
            Key::Delete | Key::Backspace => match self.side_crate.take() {
                Some(PLAYLIST) => self.notify("The Playlist crate can't be deleted"),
                Some(c) if self.pl_sidebar() => self.apply(Action::DeleteCrate(c), ctx),
                _ => self.remove_selected(),
            },
            Key::O if mods.command => self.open_files_dialog(Open::Add),
            Key::A if mods.command => self.crates.shown_mut().select_all(),
            Key::Enter => {
                // On a closed record row, the record plays.
                if let Some(ListRow::Record(rec)) = self.pl_cursor_row()
                    && !rec.single
                    && !rec.open
                {
                    let shown = self.crates.shown();
                    let ids = rec.members.iter().map(|&i| shown.entries()[i].id).collect();
                    return self.apply(Action::PlayRecord(ids), ctx);
                }
                let shown = self.crates.shown();
                if let Some(id) = shown
                    .cursor()
                    .or_else(|| shown.selected_ids().first().copied())
                {
                    self.play_entry(self.crates.shown_id(), id);
                }
            }
            // → opens the record under the cursor (grouped); ← closes it, from its record row
            // or one of its tracks, which leaves the cursor on the record row.
            Key::ArrowRight => {
                if let Some(ListRow::Record(rec)) = self.pl_cursor_row()
                    && !rec.single
                    && !rec.open
                {
                    self.apply(Action::ToggleRecord(rec.key), ctx);
                }
            }
            Key::ArrowLeft => match self.pl_cursor_row() {
                Some(ListRow::Record(rec)) if !rec.single && rec.open => {
                    self.apply(Action::ToggleRecord(rec.key), ctx);
                }
                Some(ListRow::Entry { idx, track: true }) => {
                    if let Some(key) = self.crates.shown().entries()[idx].album_key() {
                        self.apply(Action::ToggleRecord(key), ctx);
                    }
                }
                _ => {}
            },
            Key::P if !mods.command => self.show_playing_entry(),
            _ if !mods.command && self.arrows_move_cursor() => {
                let mv = match key {
                    Key::ArrowUp => CursorMove::Up,
                    Key::ArrowDown => CursorMove::Down,
                    Key::PageUp => CursorMove::PageUp,
                    Key::PageDown => CursorMove::PageDown,
                    Key::Home => CursorMove::Home,
                    Key::End => CursorMove::End,
                    _ => return,
                };
                let page = self.pl_visible_rows();
                let start = (self.crates.shown_id() == self.crates.playing_id()
                    && self.position.state != PlayState::Stopped)
                    .then(|| self.crates.shown().current())
                    .flatten();
                self.side_crate = None;
                if self.crates.shown().is_grouped() {
                    return self.move_row_cursor(mv, mods.shift, start);
                }
                let p = self.crates.shown_mut();
                p.move_cursor(mv, mods.shift, page, start);
                if let Some(i) = p.cursor_index() {
                    self.scroll_into_view(i);
                }
            }
            _ => {}
        }
    }

    /// The row the keyboard cursor is on (grouped: a closed record's row for any of its
    /// entries).
    fn pl_cursor_row(&self) -> Option<ListRow> {
        let i = self.crates.shown().cursor_index()?;
        let rows = self.pl_list();
        let r = records::row_of(&rows, i);
        rows.get(r)
            .filter(|row| row.indices().contains(&i))
            .cloned()
    }

    /// Grouped, the arrows move over the rows shown: a record row is one step, and selecting
    /// it selects its entries. Shift extends from the anchor by whole rows.
    fn move_row_cursor(&mut self, mv: CursorMove, extend: bool, start: Option<EntryId>) {
        let rows = self.pl_list();
        if rows.is_empty() {
            return;
        }
        let units = self.pl_visible_rows();
        let last = rows.len() - 1;
        let p = self.crates.shown();
        let row_of_id = |id: EntryId| p.index_of(id).map(|i| records::row_of(&rows, i));
        let at = p.cursor().and_then(row_of_id);
        let page = records::fit(&rows, self.pl_scroll, units).max(1);
        let to = match (at, mv) {
            (None, CursorMove::Up | CursorMove::Down) => start.and_then(row_of_id).unwrap_or(0),
            (at, _) => {
                let at = at.or_else(|| start.and_then(row_of_id)).unwrap_or(0);
                match mv {
                    CursorMove::Up => at.saturating_sub(1),
                    CursorMove::Down => (at + 1).min(last),
                    CursorMove::PageUp => at.saturating_sub(page),
                    CursorMove::PageDown => (at + page).min(last),
                    CursorMove::Home => 0,
                    CursorMove::End => last,
                }
            }
        };
        // Copy rows (and Add next) hold no audio: the cursor steps over them the way it was
        // going.
        let down = matches!(
            mv,
            CursorMove::Down | CursorMove::PageDown | CursorMove::End
        );
        let mut to = to;
        while matches!(rows[to], ListRow::Copy { .. } | ListRow::More { .. }) {
            match (down, to) {
                (true, t) if t < last => to += 1,
                (_, t) if t > 0 => to -= 1,
                _ => return,
            }
        }
        let ids: Vec<EntryId> = rows[to]
            .indices()
            .iter()
            .map(|&i| p.entries()[i].id)
            .collect();
        let first = rows[to].first();
        let last_idx = rows[to].indices().iter().copied().max().unwrap_or(first);
        let p = self.crates.shown_mut();
        if extend {
            p.extend_to(last_idx, &ids, ids[0]);
        } else {
            p.select_only(&ids, ids[0]);
        }
        self.scroll_into_view(first);
    }

    /// ↑/↓ move the playlist cursor while the playlist has the keyboard.
    fn arrows_move_cursor(&self) -> bool {
        self.focus == Focus::Playlist
    }

    /// The shown crate's rows (see [`records`]), rebuilt only when the crate, its filter, its
    /// grouping or its open records change.
    fn pl_list(&self) -> Rc<Vec<ListRow>> {
        let id = self.crates.shown_id();
        let p = self.crates.shown();
        let more = self.more_to_add(id);
        let key = (id, p.rev(), p.is_grouped(), self.pl_open_rev, more);
        if let Some((k, rows)) = &*self.pl_rows_cache.borrow()
            && *k == key
        {
            return rows.clone();
        }
        let empty = HashSet::new();
        let mut rows = records::build(p, self.pl_open.get(&id).unwrap_or(&empty));
        if let (Some((next, left)), Some(last)) = (more, rows.last()) {
            let first = last.first();
            rows.push(ListRow::More { next, left, first });
        }
        let rows = Rc::new(rows);
        *self.pl_rows_cache.borrow_mut() = Some((key, rows.clone()));
        rows
    }

    /// What Add next would bring into crate `id`, a seller crate dug 1,000 at a time: the next
    /// copies and all those left.
    fn more_to_add(&self, id: CrateId) -> Option<(usize, usize)> {
        #[cfg(not(target_arch = "wasm32"))]
        return self
            .seller_more_left(id)
            .map(|left| (left.min(dig::discogs::seller::LIMIT), left));
        #[cfg(target_arch = "wasm32")]
        {
            let _ = id;
            None
        }
    }

    /// Scrolls the playlist by the least amount that shows entry `index` (a crate index): its
    /// row, its closed record's row, or under a BPM filter where it would be.
    fn scroll_into_view(&mut self, index: usize) {
        let rows = self.pl_list();
        let row = records::row_of(&rows, index);
        self.pl_scroll = records::scroll_to(&rows, self.pl_scroll, row, self.pl_visible_rows());
    }

    /// The list row of crate index `index` (see [`Self::scroll_into_view`]).
    fn pl_row_of(&self, index: usize) -> usize {
        records::row_of(&self.pl_list(), index)
    }

    /// Whether list row `row` is in view.
    fn pl_row_in_view(&self, row: usize) -> bool {
        let rows = self.pl_list();
        row >= self.pl_scroll
            && row < self.pl_scroll + records::fit(&rows, self.pl_scroll, self.pl_visible_rows())
    }

    /// `P`: the playing crate, scrolled to its playing entry, with the cursor on it.
    /// A style, artist or label filter changed: saved soon, the play order follows, and the
    /// list keeps a row in view.
    fn filter_changed(&mut self) {
        self.mark_shown();
        let rows = self.pl_list().len();
        self.pl_scroll = self.pl_scroll.min(rows.saturating_sub(1));
    }

    fn show_playing_entry(&mut self) {
        let playing = self.crates.playing_id();
        let Some(id) = self.crates.playing().current() else {
            return;
        };
        if self.crates.shown_id() != playing {
            self.show_crate(playing);
        }
        // Filters that hide it are turned off.
        let p = self.crates.shown_mut();
        if p.get(id).is_some_and(|e| !p.shows(e)) {
            p.clear_filters();
            self.mark_shown();
        }
        let p = self.crates.shown_mut();
        p.set_cursor(Some(id));
        if let Some(i) = p.index_of(id) {
            self.scroll_into_view(i);
        }
    }

    fn remove_selected(&mut self) {
        let ids = self.crates.shown().selected_ids();
        self.remove_entries(&ids);
    }

    /// Removes entries of the shown crate (Remove, Remove album, Delete), except from the
    /// Discogs crates, which records leave only through the Discogs items, and label crates.
    fn remove_entries(&mut self, ids: &[EntryId]) {
        if self.refuse_discogs_edit(self.crates.shown_id()) {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.dig_before_remove(ids);
        #[cfg(not(target_arch = "wasm32"))]
        let clips = digging::clips_of(self.crates.shown(), |e| ids.contains(&e.id));
        if self.crates.shown_mut().remove_ids(ids) > 0 {
            self.mark_shown();
            #[cfg(not(target_arch = "wasm32"))]
            self.dig_forget_clips(clips);
        }
    }

    /// The crates Send to crate offers: every other crate but the ones the app fills itself
    /// (Discogs, label, seller and friend crates).
    fn send_targets(&self) -> Vec<(CrateId, String)> {
        #[cfg(not(target_arch = "wasm32"))]
        let takes = |c| self.takes_sends(c);
        #[cfg(target_arch = "wasm32")]
        let takes = |c| !self.crates.is_locked(c);
        self.crates
            .list()
            .iter()
            .filter(|c| c.id != self.crates.shown_id() && takes(c.id))
            .map(|c| (c.id, c.name.clone()))
            .collect()
    }

    /// Whether crate `c` is a Discogs crate, which only the Discogs items fill, or a label
    /// crate, which only its label fills; says how.
    fn refuse_discogs_insert(&mut self, c: CrateId) -> bool {
        if !self.crates.is_locked(c) {
            return false;
        }
        self.notify(if self.crates.is_label(c) {
            LABEL_CRATE_HINT
        } else if self.crates.is_wantlist(c) {
            "Records come into this crate with Add to wantlist (Y)"
        } else {
            "Records come into this crate with Add to collection"
        });
        true
    }

    /// Whether crate `c` is a Discogs crate or a label crate, which take no hand removals; the
    /// first refusal of the session says how records leave a Discogs crate, and every one says
    /// how a label crate works.
    fn refuse_discogs_edit(&mut self, c: CrateId) -> bool {
        if !self.crates.is_locked(c) {
            return false;
        }
        if self.crates.is_label(c) {
            self.notify(LABEL_CRATE_HINT);
        } else if !self.discogs_hint_shown {
            self.discogs_hint_shown = true;
            self.notify(if self.crates.is_wantlist(c) {
                "Records leave this crate with Remove from wantlist (Y)"
            } else {
                "Records leave this crate with Remove from collection…"
            });
        }
        true
    }

    fn set_volume(&mut self, v: f32) {
        self.settings.volume = v.clamp(0.0, 1.0);
        let v = self.settings.volume;
        self.with_engine(|e| e.set_volume(v));
        self.mark_settings();
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

    /// The player's title line: `N. (catno) Artist: Title (N BPM) (m:ss)`.
    fn now_playing_line(&self) -> Option<String> {
        let (artist, title, duration) = self.now_playing_names()?;
        let playing = self.crates.playing();
        let number = playing.current_index().map_or(0, |i| i + 1);
        let entry = playing.current().and_then(|id| playing.get(id));
        let origin = entry.and_then(|e| e.origin.as_ref());
        let catno = origin.map_or("", |o| o.catno.as_str());
        let name = format::entry_name(catno, &artist, &title, "", entry.and_then(|e| e.bpm));
        Some(format::title_line(number, &name, duration))
    }

    /// The player's second line: an entry from a catalogue page gives its label, side, year and
    /// for-sale summary; a local file its album tag. `None` while nothing plays or it has none.
    fn now_playing_detail(&self) -> Option<String> {
        if self.position.state == PlayState::Stopped {
            return None;
        }
        let playing = self.crates.playing();
        let entry = playing.current().and_then(|id| playing.get(id))?;
        match &entry.origin {
            Some(o) => Some(format::origin_line(o)).filter(|l| !l.is_empty()),
            None => Some(entry.album.trim().to_owned()).filter(|a| !a.is_empty()),
        }
    }

    /// The player's seek bar, under its button row across its width: a groove, the played
    /// part, and a thumb at the playing position. A click or a drag seeks on release, and the
    /// tooltip names the time under the pointer. Inert while nothing plays or the track's
    /// length isn't known.
    fn seek_bar(&mut self, ui: &mut Ui, sk: &Skinned, w: f32, actions: &mut Vec<Action>) {
        let h = crate::skin::generate::SEEK_H as f32;
        let bar_w = (w - 8.0).max(4.0);
        sk.sprite("seek_track_l", 4.0, SEEK_Y);
        sk.sprite_in("seek_track_m", sk.rect(6.0, SEEK_Y, bar_w - 4.0, h));
        sk.sprite("seek_track_r", 4.0 + bar_w - 2.0, SEEK_Y);
        let Some(d) = self
            .now_playing
            .as_ref()
            .and_then(|i| i.duration_secs)
            .filter(|d| *d > 0.0 && self.position.state != PlayState::Stopped)
        else {
            self.seek_drag = None;
            self.seek_pending = None;
            #[cfg(test)]
            {
                self.seek_drawn = None;
            }
            return;
        };
        let thumb_w = sk.def.sprite("seek_thumb").w as f32;
        let travel = (bar_w - thumb_w).max(1.0);
        // A little taller than the groove, so it's easy to hit, but under the transport.
        let bar = sk.rect(4.0, SEEK_Y - 2.0, bar_w, h + 3.0);
        let resp = ui.interact(bar, Id::new("mini_seek"), Sense::click_and_drag());
        let fraction =
            |p: Pos2| (((p.x - bar.left()) / sk.scale - thumb_w / 2.0) / travel).clamp(0.0, 1.0);
        if let Some(p) = resp.interact_pointer_pos() {
            self.seek_drag = Some(fraction(p));
        }
        // A seek on its way holds the thumb where it was sent, until the clock gets there (or
        // half a second goes by).
        let playing_at = (self.position.seconds() / d) as f32;
        if let Some((to, at)) = self.seek_pending
            && ((playing_at - to).abs() < 0.02 || at.elapsed() > SEEK_SETTLE)
        {
            self.seek_pending = None;
        }
        if self.seek_pending.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        let v = self
            .seek_drag
            .or(self.seek_pending.map(|(to, _)| to))
            .unwrap_or(playing_at)
            .clamp(0.0, 1.0);
        let x = 4.0 + (v * travel).round();
        sk.sprite_in(
            "seek_fill",
            sk.rect(6.0, SEEK_Y, (x + thumb_w / 2.0 - 6.0).max(0.0), h),
        );
        let name = if resp.is_pointer_button_down_on() {
            "seek_thumb_p"
        } else {
            "seek_thumb"
        };
        sk.sprite(name, x, SEEK_Y);
        #[cfg(test)]
        {
            self.seek_drawn = Some(v);
        }
        if (resp.drag_stopped() || resp.clicked())
            && let Some(v) = self.seek_drag.take()
        {
            self.seek_pending = Some((v, Instant::now()));
            actions.push(Action::Seek(v as f64 * d));
        }
        let at = self.seek_drag.or_else(|| resp.hover_pos().map(fraction));
        if let Some(f) = at {
            resp.on_hover_text(format::clock(f as f64 * d));
        }
    }

    /// The turntable, the band's middle panel `w` skin pixels wide, always there. The pitch
    /// fader sets the set's tempo against the playing record (its centre is the record's own
    /// tempo) once SYNC is on, and then shows how far each record is pushed; with SYNC off the
    /// deck is dimmed and only SYNC answers. The platter turns with the speed heard; its
    /// strobe dots stand still while the record is locked to the tempo, as on a real deck.
    fn deck_section(&mut self, ui: &mut Ui, origin: Pos2, w: f32) {
        use crate::sync::Fit;
        use std::f32::consts::TAU;
        let sync = self.settings.sync;
        let bpm = self.playing_bpm();
        let state = self.position.state;
        let playing = state != PlayState::Stopped;
        let analyzing = self.playing_bpm_coming();
        let fitted = sync.speed(bpm);
        let fit = sync.fit(bpm, analyzing, playing);
        let now = Instant::now();
        // A record becoming fitted (SYNC on over a fitted record, a fitted track starting, a
        // tempo read at last) flashes the rim; a new track fitted at another tempo than its own
        // makes the digits count to the target.
        let who = {
            let p = self.crates.playing();
            p.current().map(|e| (self.crates.playing_id(), e))
        };
        let locked = matches!(fit, Fit::Locked(_));
        let (was_who, was_locked) = self.deck_seen;
        // A new track waits for its fit (a tempo being read, a clock still starting) before
        // the rim flashes and the digits count; one that doesn't fit ends the wait.
        if who != was_who {
            self.deck_new = true;
        }
        if playing {
            match fit {
                Fit::Locked(speed) => {
                    if !was_locked || self.deck_new {
                        self.deck_flash = Some(now);
                    }
                    if self.deck_new {
                        let from = (sync.target as f64 / speed).round() as u16;
                        self.deck_count = (from != sync.target).then_some((from, sync.target, now));
                    }
                    self.deck_new = false;
                }
                Fit::Analyzing => {}
                _ => self.deck_new = false,
            }
        }
        self.deck_seen = (who, locked);
        if state == PlayState::Playing {
            let dt = ui.input(|i| i.stable_dt).min(0.1);
            let rps = 100.0 / 3.0 / 60.0 * self.position.speed as f32;
            self.deck_turn = (self.deck_turn + dt * TAU * rps) % TAU;
            let drift = match (fit, bpm) {
                (Fit::Analyzing, _) => 0.8,
                (Fit::Locked(_), _) | (Fit::Off, _) | (Fit::Idle, _) => 0.0,
                (_, Some(b)) => b as f32 / sync.target as f32 - 1.0,
                (_, None) => 0.05,
            };
            self.deck_strobe = (self.deck_strobe + dt * TAU * drift) % TAU;
        }
        let mut actions = Vec::new();
        {
            let def = self.def.clone();
            let h = crate::layout::BAND_H as f32;
            let panel = self.skinned(&def, ui, origin);
            let ox = ((w - crate::skin::generate::DECK_W as f32) / 2.0)
                .floor()
                .max(0.0);
            let sk = Skinned {
                origin: panel.origin + vec2(ox * panel.scale, 0.0),
                ..self.skinned(&def, ui, origin)
            };
            panel.sprite_in("deck_fill", panel.rect(0.0, 0.0, w, h));
            sk.sprite("deck_bg", 0.0, 0.0);
            panel.sprite_in("deck_edge_l", panel.rect(0.0, 0.0, 1.0, h));
            panel.sprite_in("deck_edge_r", panel.rect(w - 1.0, 0.0, 1.0, h));
            let lcd = color(sk.def.colors.lcd);
            let red = Color32::from_rgb(235, 72, 60);

            // The platter: strobe dots on its rim, a mark on its label.
            let p = sk.def.at("deck_platter");
            let (cx, cy) = (p.x as f32 + p.w as f32 / 2.0, p.y as f32 + p.h as f32 / 2.0);
            let px = |r: f32, a: f32, size: f32| {
                let (x, y) = (cx + r * a.cos() - size / 2.0, cy + r * a.sin() - size / 2.0);
                sk.rect(x.round(), y.round(), size, size)
            };
            for k in 0..STROBE_DOTS {
                let a = self.deck_strobe + k as f32 * TAU / STROBE_DOTS as f32;
                // Analyzing: the lit dot runs round the rim, with a tail.
                let dot = if fit == Fit::Analyzing {
                    let phase =
                        (k as f32 / STROBE_DOTS as f32 - self.deck_strobe / TAU).rem_euclid(1.0);
                    lcd.gamma_multiply((1.0 - phase).powi(2).max(0.12))
                } else {
                    lcd
                };
                sk.fill(px(STROBE_R, a, 2.0), dot);
            }
            let white = color(sk.def.colors.pl_current);
            for r in [3.0, 5.0, 7.0] {
                sk.fill(px(r, self.deck_turn, 1.0), white);
            }
            // The lock flash: the whole rim lights once and fades.
            if let Some(at) = self.deck_flash {
                let t = at.elapsed().as_secs_f32() / FLASH_RIM_SECS;
                if t >= 1.0 {
                    self.deck_flash = None;
                } else {
                    let glow = lcd.gamma_multiply((1.0 - t).powi(2));
                    for k in 0..RIM_DOTS {
                        sk.fill(
                            px(STROBE_R + 1.5, k as f32 * TAU / RIM_DOTS as f32, 2.0),
                            glow,
                        );
                    }
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                }
            }

            // The tempo in big digits: the target (counting to it on a new track), or while
            // SYNC is off the record's own BPM; then how far this record is pushed, and its
            // own tempo.
            let t = sk.def.at("deck_target");
            let digits = if !sync.on {
                match (bpm, analyzing) {
                    (_, true) => "  ?".to_owned(),
                    (Some(b), false) => format!("{b:>3}"),
                    (None, false) => "   ".to_owned(),
                }
            } else if sync.pending.is_some() || analyzing {
                "  ?".to_owned()
            } else if let Some((from, to, at)) = self.deck_count {
                let k = at.elapsed().as_secs_f32() / COUNT_SECS;
                if k >= 1.0 {
                    self.deck_count = None;
                    format!("{to:>3}")
                } else {
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                    let v = from as f32 + (to as f32 - from as f32) * k;
                    format!("{:>3}", v.round() as u16)
                }
            } else {
                format!("{:>3}", sync.target)
            };
            for (i, c) in digits.chars().enumerate() {
                let name = match c {
                    ' ' => "digit_blank".to_owned(),
                    '?' => "digit_q".to_owned(),
                    c => format!("digit_{c}"),
                };
                sk.sprite(&name, t.x as f32 + 10.0 * i as f32, t.y as f32);
            }
            let r = sk.def.at("deck_push");
            if let (Some(_), true, true) = (bpm, playing, sync.on) {
                let push = speed_label(fitted.unwrap_or(1.0)).unwrap_or_else(|| "0.0%".into());
                sk.text(r.x as f32, r.y as f32, &push, lcd);
            }
            let r = sk.def.at("deck_record");
            let rec = match (bpm, sync.pending.is_some() || analyzing) {
                (Some(b), false) => format!("REC {b}"),
                (_, true) => "REC ?".to_owned(),
                (None, false) => "REC ---".to_owned(),
            };
            sk.text(r.x as f32, r.y as f32, &rec, lcd);

            // The pitch meter: a needle at the push, pinned and red when the fit is out of range.
            let m = sk.def.at("deck_meter");
            let needle = match fit {
                Fit::Locked(s) => Some((((s - 1.0) / sync.range()).clamp(-1.0, 1.0) as f32, lcd)),
                Fit::Out(s) => Some((if s > 1.0 { 1.0 } else { -1.0 }, red)),
                _ => None,
            };
            if let Some((frac, col)) = needle {
                let half = m.w as f32 / 2.0 - 1.0;
                let x = m.x as f32 + m.w as f32 / 2.0 + frac * half;
                sk.fill(sk.rect(x.round() - 1.0, m.y as f32, 2.0, m.h as f32), col);
            }

            // The state line: a lamp and what SYNC is doing.
            let r = sk.def.at("deck_state");
            let (lamp, text) = match fit {
                Fit::Locked(_) => (lcd, lcd),
                Fit::Out(_) | Fit::NoBpm => (red, red),
                Fit::Analyzing => (lcd, lcd),
                Fit::Off | Fit::Idle => (lcd.gamma_multiply(0.35), lcd.gamma_multiply(0.35)),
            };
            #[cfg(test)]
            {
                self.deck_drawn = Some((
                    fit.line(sync.range()),
                    digits.clone(),
                    needle.map(|(f, _)| f),
                ));
            }
            sk.fill(sk.rect(r.x as f32 + 3.0, r.y as f32 + 3.0, 3.0, 6.0), lamp);
            sk.text(
                r.x as f32 + 9.0,
                r.y as f32 + 3.0,
                &fit.line(sync.range()),
                text,
            );

            let range = if sync.wide {
                "deck_range_16"
            } else {
                "deck_range_8"
            };
            let range_resp = widgets::button(ui, &sk, "deck_range", "deck_range", range)
                .on_hover_text("Pitch range: ±8% or ±16%");
            let reset_resp = widgets::button(ui, &sk, "deck_reset", "deck_reset", "deck_reset")
                .on_hover_text("The tempo back to this record's own");
            if sync.on {
                if range_resp.clicked() {
                    actions.push(Action::SyncRange);
                }
                if reset_resp.clicked() {
                    actions.push(Action::SyncTakeBpm);
                }
            }

            // The pitch fader: up is faster. Its thumb shows this record's push.
            let r = sk.def.at("deck_pitch");
            let travel = sk.rect(r.x as f32, r.y as f32, r.w as f32, r.h as f32);
            let range = sync.range() as f32;
            let pitch = fitted.map_or(0.0, |s| s as f32 - 1.0);
            let resp = ui
                .interact(
                    travel.expand(5.0 * sk.scale),
                    Id::new("deck_pitch"),
                    Sense::click_and_drag(),
                )
                .on_hover_text(
                    "Your set's tempo: up faster, down slower (the centre is this record's own)",
                );
            if let (true, Some(b), Some(pos), true) = (
                sync.on,
                bpm,
                resp.interact_pointer_pos(),
                resp.is_pointer_button_down_on(),
            ) {
                let at = ((pos.y - travel.top()) / travel.height()).clamp(0.0, 1.0);
                // Whole BPM, kept inside the range so the record stays locked.
                let b = b as f32;
                let target = (b * (1.0 + range * (1.0 - 2.0 * at)))
                    .round()
                    .clamp((b * (1.0 - range)).ceil(), (b * (1.0 + range)).floor())
                    as u16;
                if target != sync.target {
                    actions.push(Action::SyncTarget(target));
                }
            }
            // Waiting for the record's BPM: the fader does nothing, and says why.
            if sync.on && sync.pending.is_some() && resp.is_pointer_button_down_on() {
                actions.push(Action::SyncTarget(sync.target));
            }
            // Scrolling over it: 1 BPM per 4 skin pixels.
            if sync.on && resp.hovered() {
                self.sync_drag += ui.input(|i| i.smooth_scroll_delta.y) / sk.scale;
                let n = (self.sync_drag / 4.0).trunc();
                if n != 0.0 {
                    self.sync_drag -= n * 4.0;
                    actions.push(Action::SyncNudge(n as i32));
                }
            }
            let thumb = sk.def.sprite("pitch_thumb");
            let at = (0.5 - pitch / range / 2.0).clamp(0.0, 1.0);
            let name = if sync.on && resp.is_pointer_button_down_on() {
                "pitch_thumb_p"
            } else {
                "pitch_thumb"
            };
            sk.sprite(
                name,
                r.x as f32 + (r.w as f32 - thumb.w as f32) / 2.0,
                (r.y as f32 + at * r.h as f32 - thumb.h as f32 / 2.0).round(),
            );

            // SYNC off: the deck is dimmed, and only the SYNC button is lit.
            if !sync.on {
                sk.fill(
                    sk.rect(0.0, 1.0, crate::skin::generate::DECK_W as f32, h - 2.0),
                    Color32::from_black_alpha(DIM_ALPHA),
                );
            }
            let btn = if sync.on { "sync_on" } else { "sync_off" };
            let rect = sk.at("deck_sync");
            if sprite_button(ui, &sk, "deck_sync_button", rect, btn)
                .on_hover_text("SYNC: every track at one tempo, set on the turntable")
                .clicked()
            {
                actions.push(Action::Sync);
            }
        }
        for a in actions {
            self.apply(a, ui.ctx());
        }
    }

    fn eq_section(&mut self, ui: &mut Ui, origin: Pos2) {
        let mut eq = self.settings.eq;
        let mut lp = self.lp_knob;
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
            // The LP knob: drag up or down (100 skin pixels for the whole turn), double-click
            // to turn it off.
            let kr = sk.at("eq_lp");
            let (resp, delta) = widgets::drag_area(ui, &sk, "eq_lp", kr);
            if resp.double_clicked() {
                lp = audio::filter::OFF;
            } else if delta.y != 0.0 {
                lp = (lp - delta.y / 100.0).clamp(0.0, audio::filter::OFF);
            }
            sk.sprite_in("eq_lp_knob", kr);
            let angle = (-135.0 + 270.0 * lp).to_radians();
            let c = kr.center();
            let tip = c + vec2(angle.sin(), -angle.cos()) * 4.5 * sk.scale;
            let on = lp < audio::filter::OFF;
            sk.painter.line_segment(
                [c + (tip - c) * 0.2, tip],
                egui::Stroke::new(
                    1.5 * sk.scale,
                    if on {
                        color(sk.def.colors.lcd)
                    } else {
                        Color32::from_rgb(26, 26, 38)
                    },
                ),
            );
            resp.on_hover_text(format::lp_label(lp));
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
        if lp != self.lp_knob {
            self.lp_knob = lp;
            self.with_engine(|e| e.set_filter(lp));
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

    /// The Options menu on a right-click (or Control-click on a Mac) in `rect`, which sits
    /// under the section's controls so they keep their own clicks.
    fn options_menu(&self, ui: &mut Ui, rect: Rect, id: &str, actions: &mut Vec<Action>) {
        if !self.has_options() {
            return;
        }
        let resp = ui
            .interact(rect, Id::new(id), Sense::click())
            .on_hover_text("Right-click for options");
        let open = opens_context_menu(
            resp.secondary_clicked(),
            resp.clicked(),
            ui.input(|i| i.modifiers),
        );
        let command = if open {
            Some(egui::SetOpenCommand::Bool(true))
        } else if resp.clicked() {
            Some(egui::SetOpenCommand::Bool(false))
        } else {
            None
        };
        egui::Popup::context_menu(&resp)
            .open_memory(command)
            .show(|ui| self.options_items(ui, actions));
    }

    /// Whether the Options menu has anything to offer: only digging (accounts, the browser,
    /// setup) puts items in it, and an empty menu doesn't open.
    fn has_options(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.dig.is_some();
        #[cfg(target_arch = "wasm32")]
        false
    }

    /// The Options menu's items, for the right-click and the footer's gear alike: accounts and
    /// setup. The size is on Cmd+= / Cmd+− / Cmd+0 and the spectrogram on the player's button.
    fn options_items(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        #[cfg(target_arch = "wasm32")]
        let _ = (ui, actions);
        #[cfg(not(target_arch = "wasm32"))]
        if self.dig.is_some() {
            if ui.button("Discogs…").clicked() {
                actions.push(Action::Dig(DigAction::OpenDialog));
                ui.close();
            }
            if ui.button("Browser…").clicked() {
                actions.push(Action::Dig(DigAction::OpenBrowserDialog));
                ui.close();
            }
            if ui.button(self.setup_label()).clicked() {
                actions.push(Action::Dig(DigAction::OpenWelcome));
                ui.close();
            }
        }
    }

    /// The setup links an empty crate shows: with the setup panel, the steps still open;
    /// without it, the extension's as before.
    fn setup_links(&self) -> SetupLinks {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(d) = self.dig.as_ref().filter(|d| d.setup.welcome) {
            use welcome::{State, Step};
            return SetupLinks {
                discogs: d.step(Step::Discogs) == Some(State::Open),
                extension: d.step(Step::Browser) == Some(State::Open),
            };
        }
        SetupLinks {
            discogs: false,
            extension: true,
        }
    }

    /// Whether the setup panel is part of this app (it opens from links and menus).
    #[cfg(not(target_arch = "wasm32"))]
    fn welcome_on(&self) -> bool {
        self.dig.as_ref().is_some_and(|d| d.setup.welcome)
    }

    /// The playlist's width and rows now: what the window leaves under the header and the band.
    fn pl_geometry(&self) -> (u16, usize) {
        let d = &self.skin.def;
        let (w, rows) =
            crate::layout::playlist_fit(d, crate::layout::TOP_H, self.win_px.x, self.win_px.y);
        (w, rows as usize)
    }

    /// Rows the playlist shows now.
    fn pl_rows(&self) -> usize {
        self.pl_geometry().1
    }

    /// The playlist is wide enough for columns (under a header row).
    fn pl_columns(&self) -> bool {
        self.pl_geometry().0 >= crate::columns::COLUMNS_FROM_WIDTH
    }

    /// The column header shows: wide enough for columns, and not grouped into record rows
    /// (they span the whole width, so the header's labels would name nothing). A grouped crate
    /// without albums has no record rows, so it keeps its header.
    fn pl_header(&self) -> bool {
        let shown = self.crates.shown();
        self.pl_columns() && (!shown.is_grouped() || !shown.has_albums())
    }

    /// Rows of entries on screen: the playlist's rows, less the column header.
    fn pl_visible_rows(&self) -> usize {
        (self.pl_rows() - self.pl_header() as usize).max(1)
    }

    /// The playlist is wide enough for the crate sidebar (whether or not it is on).
    fn pl_sidebar_fits(&self) -> bool {
        self.pl_geometry().0 >= SIDEBAR_FROM_WIDTH
    }

    /// The seller crates for the sidebar (crate, dug, refreshing), in Top Sellers order.
    fn sidebar_sellers(&self) -> Vec<(CrateId, bool, bool)> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.seller_rows()
                .into_iter()
                .map(|r| (r.crate_id, r.dug, r.refreshing))
                .collect()
        }
        #[cfg(target_arch = "wasm32")]
        Vec::new()
    }

    /// The friend crates for the sidebar (crate, dug, private, refreshing), in Discogs'
    /// order.
    fn sidebar_friends(&self) -> Vec<(CrateId, bool, bool, bool)> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.friend_rows()
                .into_iter()
                .map(|r| (r.crate_id, r.dug, r.private, r.busy))
                .collect()
        }
        #[cfg(target_arch = "wasm32")]
        Vec::new()
    }

    /// The crate sidebar shows exactly when it fits.
    fn pl_sidebar(&self) -> bool {
        self.pl_sidebar_fits()
    }

    /// The crate sidebar in `area` (skin pixels: x, y, w, h): CRATES with the user's crates
    /// and "+ New crate" from the top, and the Discogs group anchored at the bottom: DISCOGS
    /// with the wantlist and collection pinned, then LABELS, TOP SELLERS and FRIENDS, of which
    /// at most one is open. The user's crates and the open group scroll when they don't fit
    /// (see `layout::side_layout`). Returns the crate under the pointer while entries are
    /// dragged (not the shown one), which is highlighted.
    fn crate_sidebar(
        &mut self,
        ui: &mut Ui,
        sk: &Skinned,
        area: (f32, f32, f32, f32),
        row_h: f32,
        font: &egui::FontId,
        actions: &mut Vec<Action>,
    ) -> Option<CrateId> {
        let (x, y, w, h) = area;
        let colors = sk.def.colors.clone();
        let scale = sk.scale;
        sk.fill(
            sk.rect(x, y, w, h),
            lerp_color(colors.pl_bg, colors.pl_text, 0.06),
        );
        sk.fill(
            sk.rect(x + w - 1.0, y, 1.0, h),
            lerp_color(colors.pl_bg, colors.pl_text, 0.3),
        );
        let painter = sk.painter.with_clip_rect(sk.rect(x, y, w - 1.0, h));
        let rows = ((h / row_h).floor() as usize).max(1);
        let sellers = self.sidebar_sellers();
        let friends = self.sidebar_friends();
        let friend_of: HashMap<CrateId, (bool, bool, bool)> = friends
            .iter()
            .map(|&(id, dug, private, busy)| (id, (dug, private, busy)))
            .collect();
        let seller_of: HashMap<CrateId, (bool, bool)> = sellers
            .iter()
            .map(|&(id, dug, busy)| (id, (dug, busy)))
            .collect();
        let labels: Vec<_> = self.crates.labels().cloned().collect();
        let (mine, collection) = self.side_crates(&seller_of, &friend_of);
        let shown = self.crates.shown_id();
        let reveal = std::mem::take(&mut self.side_scroll_to[0]);
        let open = self.settings.side_open;
        let crates_folded = self.settings.crates_folded;

        // The rows: CRATES and its list (a), "+ New crate", then the Discogs group's headings
        // and pinned crates around the open group's list (b).
        let a: Vec<SideRow> = if crates_folded {
            Vec::new()
        } else {
            mine.iter()
                .map(|c| SideRow::Crate(c.clone(), None))
                .collect()
        };
        let mut before: Vec<SideRow> = Vec::new();
        let mut b: Vec<SideRow> = Vec::new();
        let mut after: Vec<SideRow> = Vec::new();
        if !collection.is_empty()
            || !labels.is_empty()
            || !sellers.is_empty()
            || !friends.is_empty()
        {
            before.push(SideRow::Discogs);
        }
        before.extend(collection.into_iter().map(|c| SideRow::Crate(c, None)));
        let groups = [
            (
                SideGroup::Labels,
                labels.iter().map(|c| c.id).collect(),
                false,
            ),
            (
                SideGroup::Sellers,
                sellers.iter().map(|s| s.0).collect::<Vec<_>>(),
                sellers.iter().any(|s| s.2),
            ),
            (
                SideGroup::Friends,
                friends.iter().map(|f| f.0).collect(),
                friends.iter().any(|f| f.3),
            ),
        ];
        let mut past_open = false;
        for (g, ids, busy) in groups {
            if ids.is_empty() {
                continue;
            }
            let heading = SideRow::Group(g, ids.len(), open == Some(g), busy);
            if open == Some(g) {
                before.push(heading);
                b = ids
                    .iter()
                    .filter_map(|id| self.crates.info(*id))
                    .map(|c| SideRow::Crate(c.clone(), seller_of.get(&c.id).copied()))
                    .collect();
                past_open = true;
            } else if past_open {
                after.push(heading);
            } else {
                before.push(heading);
            }
        }
        let fixed = 2 + before.len() + after.len();
        let lay = crate::layout::side_layout(rows, fixed, a.len(), b.len());
        let crates_head = SideRow::Crates(mine.len(), crates_folded);

        let row_px = row_h * scale;
        let dragging = self.pl_drag_from.is_some();
        if !dragging {
            self.side_edge_at = None;
        }
        let pane = |first: usize, n: usize| {
            sk.rect(x, y + first as f32 * row_h, w - 1.0, n as f32 * row_h)
        };
        // (row on screen, row, scrolled pane): the bar takes the count's room.
        let mut placed: Vec<(usize, SideRow, bool)> = Vec::new();
        // (pane rect, length, rows shown, offset, which).
        let mut panes: Vec<(Rect, usize, usize, usize, usize)> = Vec::new();
        if lay.single {
            let mut all = vec![crates_head];
            all.extend(a);
            all.push(SideRow::New);
            all.extend(before);
            all.extend(b);
            all.extend(after);
            if reveal && let Some(i) = all.iter().position(|r| r.is(shown)) {
                self.side_scroll[0] = reveal_row(self.side_scroll[0], i, rows);
            }
            let first = self.pane_scroll(ui, pane(0, rows), 0, all.len(), rows, row_px);
            let scrolled = all.len() > rows;
            panes.push((pane(0, rows), all.len(), rows, first, 0));
            placed.extend(
                all.into_iter()
                    .skip(first)
                    .take(rows)
                    .enumerate()
                    .map(|(i, r)| (i, r, scrolled)),
            );
        } else {
            let (ar, br) = (lay.crates_rows, lay.group_rows);
            if reveal {
                if let Some(i) = a.iter().position(|r| r.is(shown)) {
                    self.side_scroll[0] = reveal_row(self.side_scroll[0], i, ar);
                } else if let Some(i) = b.iter().position(|r| r.is(shown)) {
                    self.side_scroll[1] = reveal_row(self.side_scroll[1], i, br);
                }
            }
            let bottom = rows - (before.len() + br + after.len());
            let b_top = bottom + before.len();
            let (a_len, b_len) = (a.len(), b.len());
            let fa = self.pane_scroll(ui, pane(1, ar), 0, a_len, ar, row_px);
            let fb = self.pane_scroll(ui, pane(b_top, br), 1, b_len, br, row_px);
            panes.push((pane(1, ar), a_len, ar, fa, 0));
            panes.push((pane(b_top, br), b_len, br, fb, 1));
            placed.push((0, crates_head, false));
            placed.extend(
                a.into_iter()
                    .skip(fa)
                    .take(ar)
                    .enumerate()
                    .map(|(i, r)| (1 + i, r, a_len > ar)),
            );
            placed.push((1 + ar, SideRow::New, false));
            let rest = before
                .into_iter()
                .map(|r| (r, false))
                .chain(b.into_iter().skip(fb).take(br).map(|r| (r, b_len > br)))
                .chain(after.into_iter().map(|r| (r, false)));
            placed.extend(rest.enumerate().map(|(i, (r, s))| (bottom + i, r, s)));
        }

        let heading = |row: usize, text: &str| {
            let dr = sk.rect(x, y + row as f32 * row_h, w - 1.0, row_h);
            let width = painter
                .text(
                    pos2(dr.left() + 3.0 * scale, dr.center().y),
                    egui::Align2::LEFT_CENTER,
                    text,
                    egui::FontId::proportional(font.size * 0.75),
                    color(colors.pl_owned),
                )
                .width();
            painter.hline(
                dr.left() + width + 6.0 * scale..=dr.right() - 3.0 * scale,
                dr.center().y,
                egui::Stroke::new(1.0, lerp_color(colors.pl_owned, colors.pl_bg, 0.5)),
            );
            dr
        };
        let mark = |open: bool| if open { "⏷" } else { "⏵" };
        let mut target = None;
        for (r, row, scrolled) in placed {
            match row {
                SideRow::Discogs => {
                    heading(r, "DISCOGS");
                }
                SideRow::Crates(n, folded) => {
                    let dr = heading(r, &format!("{} CRATES ({n})", mark(!folded)));
                    if ui
                        .interact(dr, Id::new("pl_side_crates"), Sense::click())
                        .on_hover_text("Your crates: click to fold")
                        .clicked()
                    {
                        actions.push(Action::ToggleSideGroup(None));
                    }
                }
                SideRow::Group(g, n, is_open, busy) => {
                    let name = match g {
                        SideGroup::Labels => "LABELS",
                        SideGroup::Sellers => "TOP SELLERS",
                        SideGroup::Friends => "FRIENDS",
                    };
                    // A dig or refresh inside a closed group still shows.
                    let busy = busy && !is_open;
                    let dots = if busy { " …" } else { "" };
                    let dr = heading(r, &format!("{} {name} ({n}){dots}", mark(is_open)));
                    let resp = ui.interact(dr, Id::new(("pl_side_group", name)), Sense::click());
                    let mut tip = match g {
                        SideGroup::Labels => {
                            "The labels you follow: click to open or close".to_owned()
                        }
                        SideGroup::Sellers => {
                            "Your Top Sellers: click to open or close, right-click to add one"
                                .to_owned()
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        SideGroup::Friends => self.friends_tip(),
                        #[cfg(target_arch = "wasm32")]
                        SideGroup::Friends => String::new(),
                    };
                    if busy {
                        tip += match g {
                            SideGroup::Friends => "\nRefreshing…",
                            _ => "\nDigging…",
                        };
                    }
                    let resp = resp.on_hover_text(tip);
                    #[cfg(not(target_arch = "wasm32"))]
                    if g == SideGroup::Sellers {
                        use crate::app::sellers::SellerAction;
                        let menu_click = opens_context_menu(
                            resp.secondary_clicked(),
                            resp.clicked(),
                            ui.input(|i| i.modifiers),
                        );
                        if resp.clicked() && !menu_click {
                            actions.push(Action::ToggleSideGroup(Some(g)));
                        }
                        let open = menu_click.then_some(egui::SetOpenCommand::Bool(true));
                        egui::Popup::context_menu(&resp)
                            .open_memory(open)
                            .show(|ui| {
                                if ui.button("Add seller…").clicked() {
                                    actions.push(Action::Dig(DigAction::Seller(SellerAction::Add)));
                                    ui.close();
                                }
                            });
                        continue;
                    }
                    if resp.clicked() {
                        actions.push(Action::ToggleSideGroup(Some(g)));
                    }
                }
                SideRow::New => {
                    let nr = sk.rect(x, y + r as f32 * row_h, w - 1.0, row_h);
                    painter.text(
                        pos2(nr.left() + 3.0 * scale, nr.center().y),
                        egui::Align2::LEFT_CENTER,
                        "+ New crate",
                        font.clone(),
                        lerp_color(colors.pl_text, colors.pl_bg, 0.3),
                    );
                    if ui
                        .interact(nr, Id::new("pl_side_new"), Sense::click())
                        .clicked()
                    {
                        actions.push(Action::NewCrate);
                    }
                }
                SideRow::Crate(c, seller) => {
                    let rr = sk.rect(x, y + r as f32 * row_h, w - 1.0, row_h);
                    let friend = friend_of.get(&c.id).copied();
                    let pad = if scrolled { SIDE_BAR_W + 1.0 } else { 0.0 };
                    if self.crate_row(
                        ui, sk, &painter, "pl_side", rr, &c, seller, friend, pad, font, actions,
                    ) {
                        target = Some(c.id);
                    }
                }
            }
        }
        for (rect, len, shown_rows, first, which) in panes {
            self.pane_bar(ui, sk, rect, which, len, shown_rows, first);
        }
        target
    }

    /// When the shown crate changes, by any means, its sidebar group opens (or CRATES unfolds),
    /// and the sidebar and the CRATES panel scroll to it when next drawn. At launch it is
    /// only scrolled to.
    fn follow_shown(&mut self) {
        let shown = self.crates.shown_id();
        if self.side_revealed == Some(shown) {
            return;
        }
        let launch = self.side_revealed.is_none();
        self.side_revealed = Some(shown);
        self.side_scroll_to = [true; 2];
        if launch {
            return;
        }
        let group = if self.crates.is_label(shown) {
            Some(SideGroup::Labels)
        } else if self.sidebar_sellers().iter().any(|s| s.0 == shown) {
            Some(SideGroup::Sellers)
        } else if self.sidebar_friends().iter().any(|f| f.0 == shown) {
            Some(SideGroup::Friends)
        } else {
            None
        };
        match group {
            Some(g) if self.settings.side_open != Some(g) => {
                self.settings.side_open = Some(g);
                self.side_scroll[1] = 0;
                self.mark_settings();
            }
            None if self.settings.crates_folded
                && self.crates.info(shown).is_some_and(|c| !c.discogs()) =>
            {
                self.settings.crates_folded = false;
                self.mark_settings();
            }
            _ => {}
        }
    }

    /// The crates the sidebar lists under CRATES (the user's own: no Discogs, label, seller or
    /// friend crate) in creation order, and the user's Discogs crates, the wantlist first.
    fn side_crates(
        &self,
        seller_of: &HashMap<CrateId, (bool, bool)>,
        friend_of: &HashMap<CrateId, (bool, bool, bool)>,
    ) -> (Vec<crate::crates::CrateInfo>, Vec<crate::crates::CrateInfo>) {
        let (mine, mut collection): (Vec<_>, Vec<_>) = self
            .crates
            .list()
            .iter()
            .filter(|c| {
                !seller_of.contains_key(&c.id) && !friend_of.contains_key(&c.id) && !c.is_label()
            })
            .cloned()
            .partition(|c| !c.discogs());
        collection.sort_by_key(|c| !c.wantlist);
        (mine, collection)
    }

    /// A sidebar list's first row this frame (`which`: 0 the crates or the whole sidebar, 1 the
    /// open group, 2 the CRATES panel): the wheel or trackpad over it, and while entries are
    /// dragged, its first or last row steps it a row every `SIDE_EDGE_STEP`.
    fn pane_scroll(
        &mut self,
        ui: &Ui,
        rect: Rect,
        which: usize,
        len: usize,
        rows: usize,
        row_px: f32,
    ) -> usize {
        let max = len.saturating_sub(rows);
        let first = self.side_scroll[which].min(max);
        let acc = &mut self.side_scroll_acc[which];
        // An open record card takes the pointer: nothing under it scrolls.
        let mut first = if ui.rect_contains_pointer(rect) && self.card.is_none() {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            scroll_rows(acc, first, max, dy, row_px)
        } else {
            *acc = 0.0;
            first
        };
        if self.pl_drag_from.is_some()
            && max > 0
            && let Some(p) = ui.ctx().pointer_latest_pos()
            && rect.contains(p)
        {
            let step: isize = if p.y < rect.top() + row_px {
                -1
            } else if p.y >= rect.bottom() - row_px {
                1
            } else {
                0
            };
            let moved = first.saturating_add_signed(step).min(max);
            if moved != first {
                let now = Instant::now();
                if self
                    .side_edge_at
                    .is_none_or(|t| now.duration_since(t) >= SIDE_EDGE_STEP)
                {
                    self.side_edge_at = Some(now);
                    first = moved;
                }
                ui.ctx().request_repaint_after(SIDE_EDGE_STEP);
            }
        }
        self.side_scroll[which] = first;
        first
    }

    /// A sidebar list's thin scrollbar, when it holds more than it shows: click or drag it to
    /// move.
    #[allow(clippy::too_many_arguments)]
    fn pane_bar(
        &mut self,
        ui: &Ui,
        sk: &Skinned,
        rect: Rect,
        which: usize,
        len: usize,
        rows: usize,
        first: usize,
    ) {
        if len <= rows || rows == 0 {
            return;
        }
        let colors = &sk.def.colors;
        let max = len - rows;
        let bar = Rect::from_min_max(
            pos2(rect.right() - SIDE_BAR_W * sk.scale, rect.top()),
            rect.max,
        );
        sk.painter
            .rect_filled(bar, 0.0, lerp_color(colors.pl_bg, colors.pl_text, 0.12));
        let thumb_h = (bar.height() * rows as f32 / len as f32).max(bar.height() / rows as f32);
        let travel = bar.height() - thumb_h;
        let top = bar.top() + travel * first as f32 / max as f32;
        sk.painter.rect_filled(
            Rect::from_min_size(pos2(bar.left(), top), vec2(bar.width(), thumb_h)),
            0.0,
            color(colors.pl_owned),
        );
        let resp = ui.interact(
            bar,
            Id::new(("pl_side_bar", which)),
            Sense::click_and_drag(),
        );
        if let Some(p) = resp.interact_pointer_pos()
            && travel > 0.0
        {
            let f = ((p.y - bar.top() - thumb_h / 2.0) / travel).clamp(0.0, 1.0);
            self.side_scroll[which] = (f * max as f32).round() as usize;
        }
    }

    /// One crate in the sidebar or the CRATES panel (`place`, which keeps their widgets apart),
    /// in `rr`: its marks, name and count (`pad` skin pixels in from the right, for a
    /// scrollbar), its tooltip, click, menu and Delete key. Returns whether it is under the pointer while entries are dragged (not the shown
    /// crate): the drop target.
    #[allow(clippy::too_many_arguments)]
    fn crate_row(
        &mut self,
        ui: &mut Ui,
        sk: &Skinned,
        painter: &egui::Painter,
        place: &str,
        rr: Rect,
        c: &crate::crates::CrateInfo,
        seller: Option<(bool, bool)>,
        friend: Option<(bool, bool, bool)>,
        pad: f32,
        font: &egui::FontId,
        actions: &mut Vec<Action>,
    ) -> bool {
        let colors = sk.def.colors.clone();
        let scale = sk.scale;
        let (shown, playing) = (self.crates.shown_id(), self.crates.playing_id());
        let state = self.position.state;
        let dragging = self.pl_drag_from.is_some();
        let readable = !self.crates.is_unreadable(c.id);
        // A seller crate never dug, or a friend's private collection, is dimmed and shows
        // no count.
        let undug =
            seller.is_some_and(|(dug, _)| !dug) || friend.is_some_and(|(_, private, _)| private);
        let over = dragging && readable && c.id != shown && ui.rect_contains_pointer(rr);
        // A Discogs crate isn't lit as a target: a drop there only says how to fill it.
        if over && !self.crates.is_locked(c.id) && seller.is_none() {
            painter.rect_filled(rr, 0.0, color(colors.pl_selected_bg));
        } else if c.id == shown {
            painter.rect_filled(
                rr,
                0.0,
                lerp_color(colors.pl_selected_bg, colors.pl_bg, 0.5),
            );
        }
        let base = if c.discogs() {
            colors.pl_owned
        } else if c.id == shown {
            colors.pl_current
        } else {
            colors.pl_text
        };
        let col = if readable && !undug {
            color(base)
        } else {
            lerp_color(base, colors.pl_bg, 0.55)
        };
        // The left slot: ▶ / ‖ while this crate plays or is paused, a record for the
        // collection, a dot for the shown crate.
        let live = (c.id == playing)
            .then_some(state)
            .filter(|s| *s != PlayState::Stopped);
        let slot = pos2(rr.left() + 2.0 * scale, rr.center().y);
        match live {
            Some(s) => {
                let name = if s == PlayState::Playing {
                    "status_play"
                } else {
                    "status_pause"
                };
                let r = sk.def.sprite(name);
                sk.sprite_in(
                    name,
                    Rect::from_min_size(
                        slot - vec2(0.0, r.h as f32 * scale / 2.0),
                        vec2(r.w as f32, r.h as f32) * scale,
                    ),
                );
            }
            None if c.discogs() => {
                let centre = slot + vec2(4.0 * scale, 0.0);
                painter.circle_stroke(centre, 3.5 * scale, egui::Stroke::new(scale, col));
                painter.circle_filled(centre, scale, col);
            }
            None if c.id == shown => {
                painter.text(slot, egui::Align2::LEFT_CENTER, "•", font.clone(), col);
            }
            None => {}
        }
        // Every crate counts records, grouped or flat.
        let shown_count = self.crates.record_count(c.id);
        let count_text = if undug {
            String::new()
        } else {
            shown_count.to_string()
        };
        let count = painter
            .text(
                pos2(rr.right() - (3.0 + pad) * scale, rr.center().y),
                egui::Align2::RIGHT_CENTER,
                count_text,
                font.clone(),
                col,
            )
            .width();
        painter
            .with_clip_rect(Rect::from_min_max(
                rr.min,
                pos2(rr.right() - count - 6.0 * scale, rr.max.y),
            ))
            .text(
                pos2(rr.left() + 13.0 * scale, rr.center().y),
                egui::Align2::LEFT_CENTER,
                side_name(c, seller.is_some() || friend.is_some()),
                font.clone(),
                col,
            );
        let mut resp = ui.interact(rr, Id::new((place, c.id)), Sense::click());
        if resp.hovered() && !dragging {
            let tracks = entries_label(self.crates.entry_count(c.id));
            let records = match shown_count {
                1 => "1 record".to_owned(),
                n => format!("{n} records"),
            };
            let mut tip = format!("{} ({records}, {tracks})", c.name);
            if c.collection {
                tip += "\nYour Discogs collection";
            } else if c.wantlist {
                tip += "\nYour Discogs wantlist";
            } else if c.is_label() {
                tip += "\nA label you follow: it fills from the label's page";
            } else if friend.is_some_and(|(_, private, _)| private) {
                tip += "\nPrivate collection";
            } else if friend.is_some_and(|(dug, ..)| !dug) {
                tip += "\nA Discogs friend: double-click to dig their collection";
            } else if friend.is_some() {
                tip += "\nA Discogs friend's collection: right-click to refresh it";
            } else if undug {
                tip += "\nTop Sellers: double-click to dig it";
            } else if seller.is_some() {
                tip += "\nTop Sellers: double-click to refresh it (once a day)";
            }
            match live {
                Some(PlayState::Playing) => tip += "\nPlaying from this crate",
                Some(_) => tip += "\nPaused in this crate",
                None => {}
            }
            resp = resp.on_hover_text(tip);
        }
        // A right-click, or a Control-click on a Mac, opens the crate's menu.
        let menu_click = opens_context_menu(
            resp.secondary_clicked(),
            resp.clicked(),
            ui.input(|i| i.modifiers),
        );
        if resp.clicked() && !menu_click && readable {
            actions.push(Action::ShowCrate(c.id));
        }
        #[cfg(not(target_arch = "wasm32"))]
        if resp.double_clicked() && seller.is_some() && readable {
            actions.push(Action::Dig(DigAction::Seller(
                crate::app::sellers::SellerAction::Dig(c.id),
            )));
        }
        #[cfg(not(target_arch = "wasm32"))]
        if resp.double_clicked() && friend.is_some() && readable {
            actions.push(Action::Dig(DigAction::Friend(
                crate::app::friends::FriendAction::Dig(c.id),
            )));
        }
        if resp.clicked() || menu_click {
            self.side_crate = Some(c.id);
        }
        // Move to Labels needs the crate's entries: a right-click loads it.
        if menu_click && readable {
            self.crates.load(c.id);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let movable = self.label_to_move(c.id);
        let open = if menu_click {
            Some(egui::SetOpenCommand::Bool(true))
        } else if resp.clicked() {
            Some(egui::SetOpenCommand::Bool(false))
        } else {
            None
        };
        let entries = self.crates.entry_count(c.id);
        egui::Popup::context_menu(&resp)
            .open_memory(open)
            .show(|ui| {
                if c.id == PLAYLIST {
                    // The scratch crate stays (command-line files fill it): it clears.
                    if ui
                        .add_enabled(entries > 0, egui::Button::new("Clear crate"))
                        .clicked()
                    {
                        actions.push(Action::ClearCrate(c.id));
                        ui.close();
                    }
                    self.export_item(ui, c.id, actions);
                    ui.label(
                        egui::RichText::new(
                            "Playlist can't be renamed or deleted:\ncommand-line files use it",
                        )
                        .weak()
                        .small(),
                    );
                    return;
                }
                if c.is_label() {
                    self.label_menu(ui, c.id, actions);
                    return;
                }
                #[cfg(not(target_arch = "wasm32"))]
                if c.discogs() {
                    self.dig_refresh_item(ui, c, actions);
                }
                if c.collection && ui.button("Open Insights").clicked() {
                    actions.push(Action::OpenInsights);
                    ui.close();
                }
                #[cfg(not(target_arch = "wasm32"))]
                if let Some((dug, busy)) = seller {
                    use crate::app::sellers::SellerAction;
                    if dug {
                        let label = if busy {
                            "Refreshing…"
                        } else {
                            "Refresh seller"
                        };
                        if ui
                            .add_enabled(!busy, egui::Button::new(label))
                            .on_hover_text("Read this seller's copies for sale again")
                            .clicked()
                        {
                            actions
                                .push(Action::Dig(DigAction::Seller(SellerAction::Refresh(c.id))));
                            ui.close();
                        }
                        if ui.button("Narrow down…").clicked() {
                            actions
                                .push(Action::Dig(DigAction::Seller(SellerAction::Narrow(c.id))));
                            ui.close();
                        }
                        if ui
                            .button("New search…")
                            .on_hover_text("Empty this crate and dig the seller with a new search")
                            .clicked()
                        {
                            actions.push(Action::Dig(DigAction::Seller(SellerAction::NewSearch(
                                c.id,
                            ))));
                            ui.close();
                        }
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                if let Some((true, false, busy)) = friend {
                    let label = if busy {
                        "Refreshing…"
                    } else {
                        "Refresh friend"
                    };
                    if ui
                        .add_enabled(!busy, egui::Button::new(label))
                        .on_hover_text("Read this friend's collection again")
                        .clicked()
                    {
                        actions.push(Action::Dig(DigAction::Friend(
                            crate::app::friends::FriendAction::Refresh(c.id),
                        )));
                        ui.close();
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(label) = movable
                    && ui
                        .button("Move to Labels")
                        .on_hover_text(
                            "Follow this crate's label: it moves under LABELS and fills \
                             only from the label from then on",
                        )
                        .clicked()
                {
                    actions.push(Action::MoveToLabels(c.id, label));
                    ui.close();
                }
                self.export_item(ui, c.id, actions);
                // The wantlist and collection crates are the account's: never renamed or
                // deleted here.
                if c.discogs() {
                    return;
                }
                if ui
                    .add_enabled(readable, egui::Button::new("Rename crate…"))
                    .clicked()
                {
                    actions.push(Action::RenameCrate(c.id));
                    ui.close();
                }
                #[cfg(not(target_arch = "wasm32"))]
                if seller.is_some() {
                    if ui.button("Remove seller…").clicked() {
                        actions.push(Action::Dig(DigAction::Seller(
                            crate::app::sellers::SellerAction::Remove(c.id),
                        )));
                        ui.close();
                    }
                    return;
                }
                if ui.button("Delete crate…").clicked() {
                    actions.push(Action::DeleteCrate(c.id));
                    ui.close();
                }
            });
        over
    }

    /// The filter bar's BPM control, from `x` to `x_end` (skin pixels, in the bar at `y`, `h`
    /// tall): "BPM", a two-handle range over the crate's tempos and the range as text (the
    /// bar's × clears it). Nothing when the crate has fewer than two different tempos.
    /// Returns where it ends (`x` when nothing is drawn).
    fn bpm_control(
        &mut self,
        ui: &mut Ui,
        sk: &Skinned,
        (x, y, x_end, h): (f32, f32, f32, f32),
        actions: &mut Vec<Action>,
    ) -> f32 {
        let shown = self.crates.shown();
        let Some((lo, hi)) = shown.tempo_span() else {
            return x;
        };
        let filter = shown.bpm_filter();
        let (a, b) = filter.unwrap_or((lo, hi));
        let colors = &sk.def.colors;
        let lcd = color(colors.lcd);
        let ty = y + ((h - sk.def.font.glyph_h as f32) / 2.0).round();

        // A short slider of a fixed width, then the range: "BPM" goes in front when there's
        // room for it too. The slider starts at a fixed place, so it never moves under a
        // dragged handle.
        let (label, core) = bpm_widths(sk);
        let with_label = x_end - x >= label + core;
        if with_label {
            sk.text(x, ty, "BPM", lcd);
        }
        let x0 = if with_label { x + label } else { x };
        let x1 = x0 + BPM_SLIDER_W;
        let with_text = x_end - x >= core;
        if x_end - x0 < BPM_SLIDER_W {
            return x;
        }
        #[cfg(test)]
        {
            self.bpm_slider = Some((x0, x1, with_text));
        }
        let range = format!("{a}-{b}");
        let mut end = x1;
        if with_text {
            sk.text(x1 + 3.0, ty, &range, lcd);
            end = x1 + 3.0 + sk.text_width(&range);
        }

        // The track, its selected part, and the two handles.
        let span = (hi - lo) as f32;
        let at = |v: u16| x0 + (v - lo) as f32 / span * (x1 - x0);
        let mid = y + (h / 2.0).floor();
        sk.fill(
            sk.rect(x0, mid, x1 - x0, 1.0),
            lerp_color(colors.pl_text, colors.pl_bg, 0.7),
        );
        sk.fill(sk.rect(at(a), mid - 1.0, at(b) - at(a), 3.0), lcd);
        let handle = sk.def.sprite("bpm_handle");
        let (hw, hh) = (handle.w as f32, handle.h as f32);
        for v in [a, b] {
            sk.sprite(
                "bpm_handle",
                (at(v) - hw / 2.0).round(),
                y + ((h - hh) / 2.0).round(),
            );
        }
        let resp = ui.interact(
            sk.rect(x0 - hw / 2.0, y, x1 - x0 + hw, h),
            Id::new("bpm_range"),
            Sense::click_and_drag(),
        );
        if resp.double_clicked() {
            actions.push(Action::SetBpmFilter(None));
        } else if let Some(p) = resp.interact_pointer_pos() {
            let px = (p.x - sk.origin.x) / sk.scale;
            let v = lo + ((px - x0) / (x1 - x0) * span).round().clamp(0.0, span) as u16;
            // The handle nearest the pointer when the drag starts is the one that moves.
            let which = *self
                .bpm_drag
                .get_or_insert(if (px - at(a)).abs() <= (px - at(b)).abs() {
                    0
                } else {
                    1
                });
            let (na, nb) = if which == 0 {
                (v.min(b), b)
            } else {
                (a, v.max(a))
            };
            if (na, nb) != (a, b) {
                actions.push(Action::SetBpmFilter(Some((na, nb))));
            }
        } else {
            self.bpm_drag = None;
        }
        let mut tip = format!("{range} BPM: drag a handle to keep only these tempos");
        if filter.is_some() {
            tip += "; double-click to show all";
        }
        let missing = shown.without_bpm();
        if missing > 0 {
            tip += &format!(
                " ({} without a BPM {} hidden while a range is set)",
                entries_label(missing),
                if missing == 1 { "is" } else { "are" }
            );
        }
        resp.on_hover_text(tip);
        end
    }

    /// The shown crate's values for `facet` with their record counts, counted again only when
    /// the crate changes.
    fn facet_counts(&mut self, facet: Facet) -> FacetCounts {
        let c = self.crates.shown_id();
        let rev = self.crates.shown().rev();
        let slot = &mut self.facet_cache[facet as usize];
        match slot {
            Some((sc, sr, list)) if (*sc, *sr) == (c, rev) => list.clone(),
            _ => {
                let list = Rc::new(self.crates.shown().counts(facet));
                *slot = Some((c, rev, list.clone()));
                list
            }
        }
    }

    /// The filters the shown crate offers: those its records have at least two values of (a
    /// local file has none, so a crate of them offers none).
    fn offered_facets(&mut self) -> Vec<Facet> {
        Facet::ALL
            .into_iter()
            .filter(|&f| self.facet_counts(f).len() >= 2)
            .collect()
    }

    /// Whether the shown crate offers `facet` (see [`Self::offered_facets`]), from the counts
    /// the filter bar keeps when they are current.
    fn offers(&self, facet: Facet) -> bool {
        let shown = self.crates.shown();
        match &self.facet_cache[facet as usize] {
            Some((c, rev, list)) if (*c, *rev) == (self.crates.shown_id(), shown.rev()) => {
                list.len() >= 2
            }
            _ => shown.counts(facet).len() >= 2,
        }
    }

    /// The widths the filter buttons want: with style chips (when the styles are few enough)
    /// and as buttons only; `None` when the crate offers no filter.
    fn filter_widths(&mut self, sk: &Skinned) -> Option<(Option<f32>, f32)> {
        let facets = self.offered_facets();
        if facets.is_empty() {
            return None;
        }
        let counts: Vec<FacetCounts> = facets.iter().map(|&f| self.facet_counts(f)).collect();
        let shown = self.crates.shown();
        let width = |items: &[f32]| {
            items.iter().sum::<f32>() + STYLE_GAP * items.len().saturating_sub(1) as f32
        };
        let buttons: Vec<f32> = facets
            .iter()
            .map(|&f| sk.text_width(&facet_button_label(shown, f)))
            .collect();
        let chips: Option<Vec<f32>> = facets
            .iter()
            .position(|&f| f == Facet::Style)
            .filter(|&i| counts[i].len() <= STYLE_CHIPS_MAX)
            .map(|i| {
                counts[i]
                    .iter()
                    .map(|(s, _)| sk.text_width(&s.to_uppercase()))
                    .chain(
                        facets
                            .iter()
                            .zip(&buttons)
                            .filter(|(f, _)| **f != Facet::Style)
                            .map(|(_, w)| *w),
                    )
                    .collect()
            });
        Some((chips.map(|c| width(&c)), width(&buttons)))
    }

    /// The filter bar's style, artist, label and format filters, from `x` to `x_end` (skin
    /// pixels, like [`Self::bpm_control`]): the style chips (at most [`STYLE_CHIPS_MAX`]) then
    /// the other buttons when all of it fits, else one button per filter when they fit, else
    /// one FILTERS button. Each opens the filter panel. Returns where they end.
    fn filter_controls(
        &mut self,
        ui: &mut Ui,
        sk: &Skinned,
        (x, y, x_end, h): (f32, f32, f32, f32),
        actions: &mut Vec<Action>,
    ) -> f32 {
        // With no filter but a CART switch that doesn't fit, FILTERS still opens the panel.
        let cart_w = self
            .cart_label()
            .map(|l| 2.0 * STYLE_GAP + sk.text_width(&l));
        let (chips_w, buttons_w) = match self.filter_widths(sk) {
            Some(w) => w,
            None if cart_w.is_some_and(|w| x + w > x_end) => (None, f32::INFINITY),
            None => return x,
        };
        let facets = self.offered_facets();
        let counts: Vec<FacetCounts> = facets.iter().map(|&f| self.facet_counts(f)).collect();
        let shown = self.crates.shown();
        let lcd = color(sk.def.colors.lcd);
        let dim = lerp_color(sk.def.colors.lcd, sk.def.colors.pl_bg, 0.6);
        let ty = y + ((h - sk.def.font.glyph_h as f32) / 2.0).round();
        let x = x + STYLE_GAP;
        let room = x_end - x;
        let set = facets
            .iter()
            .filter(|&&f| shown.filter(f).is_some())
            .count()
            + shown.cart_only() as usize;
        let folded = if set > 0 {
            format!("FILTERS {set}")
        } else {
            "FILTERS".to_owned()
        };
        let tier = if chips_w.is_some_and(|w| w <= room) {
            FooterFilters::Chips
        } else if buttons_w <= room {
            FooterFilters::Buttons
        } else if sk.text_width(&folded) <= room {
            FooterFilters::Folded
        } else {
            FooterFilters::None
        };
        #[cfg(test)]
        {
            self.filters_drawn = Some((x, tier));
        }
        if tier == FooterFilters::None {
            return x;
        }
        if tier == FooterFilters::Folded {
            let w = sk.text_width(&folded);
            sk.text(x, ty, &folded, if set > 0 { lcd } else { dim });
            let button = ui.interact(
                sk.rect(x - 1.0, y, w + 2.0, h),
                Id::new("filters_button"),
                Sense::click(),
            );
            if button.clicked() {
                // On the first tab with a set filter, or the first tab.
                self.panel_tab = facets
                    .iter()
                    .copied()
                    .find(|&f| self.crates.shown().filter(f).is_some())
                    .or(facets.first().copied())
                    .unwrap_or(self.panel_tab);
            }
            egui::Popup::from_toggle_button_response(&button)
                .id(filters_popup_id())
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| self.filter_panel(ui, actions));
            button.on_hover_text("Filters: click to choose which records show");
            return x + w;
        }
        // What to draw, worked out before the lists borrow the app.
        let texts: Vec<(String, bool)> = facets
            .iter()
            .map(|&f| (facet_button_label(shown, f), shown.filter(f).is_some()))
            .collect();
        let styles_on: Vec<bool> = facets
            .iter()
            .position(|&f| f == Facet::Style)
            .map(|i| {
                counts[i]
                    .iter()
                    .map(|(s, _)| shown.picked(Facet::Style, s))
                    .collect()
            })
            .unwrap_or_default();
        let mut cx = x;
        for ((&f, _), (text, on)) in facets.iter().zip(&counts).zip(texts) {
            if f == Facet::Style && tier == FooterFilters::Chips {
                let list = &counts[facets.iter().position(|&g| g == f).unwrap_or(0)];
                for ((style, n), &on) in list.iter().zip(&styles_on) {
                    let name = style.to_uppercase();
                    let w = sk.text_width(&name);
                    sk.text(cx, ty, &name, if on { lcd } else { dim });
                    let r = ui.interact(
                        sk.rect(cx - 1.0, y, w + 2.0, h),
                        Id::new(("style_chip", style.as_str())),
                        Sense::click(),
                    );
                    if r.double_clicked() {
                        actions.push(Action::ClearPicks(Some(f)));
                    } else if r.clicked() {
                        actions.push(Action::TogglePick(f, style.clone()));
                    }
                    r.on_hover_text(format!(
                        "{style}: {} — click to {} it; double-click shows every style",
                        records_label(*n),
                        if on { "unselect" } else { "select" }
                    ));
                    cx += w + STYLE_GAP;
                }
                continue;
            }
            let w = sk.text_width(&text);
            sk.text(cx, ty, &text, if on { lcd } else { dim });
            let button = ui.interact(
                sk.rect(cx - 1.0, y, w + 2.0, h),
                Id::new(("facet_button", f.name())),
                Sense::click(),
            );
            if button.double_clicked() {
                actions.push(Action::ClearPicks(Some(f)));
            } else if button.clicked() {
                self.panel_tab = f;
            }
            let n = self.facet_counts(f).len();
            let popup = egui::Popup::from_toggle_button_response(&button)
                .id(facet_popup_id(f))
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
            popup.show(|ui| self.filter_panel(ui, actions));
            button.on_hover_text(format!(
                "{n} {}s: click to choose which records show; double-click shows them all",
                f.name()
            ));
            cx += w + STYLE_GAP;
        }
        cx - STYLE_GAP
    }

    /// The filter panel: a tab per filter the crate offers (its list: search, checkboxes with
    /// record counts, Clear) and, in a seller crate with a cart, the CART switch.
    fn filter_panel(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let facets = self.offered_facets();
        let tab = if facets.contains(&self.panel_tab) {
            Some(self.panel_tab)
        } else {
            facets.first().copied()
        };
        let cart = self.cart_label();
        let mut chosen = None;
        ui.horizontal(|ui| {
            for &f in &facets {
                if ui
                    .selectable_label(tab == Some(f), f.name().to_uppercase())
                    .clicked()
                {
                    chosen = Some(f);
                }
            }
            if let Some(label) = &cart {
                let mut on = self.crates.shown().cart_only();
                if ui.checkbox(&mut on, label.as_str()).clicked() {
                    actions.push(Action::ToggleCartOnly);
                }
            }
        });
        if let Some(f) = chosen {
            self.panel_tab = f;
        }
        if let Some(f) = chosen.or(tab) {
            ui.separator();
            let list = self.facet_counts(f);
            if f == Facet::Year {
                self.year_grid(ui, &list, actions);
            } else {
                self.facet_list(ui, f, &list, actions);
            }
        }
    }

    /// The YEAR tab: every year from the crate's first to its last, five a row, each with its
    /// number of records (years without any are dimmed). A click picks or unpicks a year; a
    /// Shift-click picks the run from the year clicked before.
    fn year_grid(&mut self, ui: &mut Ui, list: &FacetCounts, actions: &mut Vec<Action>) {
        ui.set_min_width(220.0);
        let counts: BTreeMap<u16, usize> = list
            .iter()
            .filter_map(|(y, n)| Some((y.parse().ok()?, *n)))
            .collect();
        let (Some(&lo), Some(&hi)) = (counts.keys().next(), counts.keys().next_back()) else {
            return;
        };
        let shown = self.crates.shown();
        let shift = ui.input(|i| i.modifiers.shift);
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                egui::Grid::new("year_grid").num_columns(5).show(ui, |ui| {
                    // Rows start on a year ending in 0 or 5, so decades line up.
                    for y in lo - lo % 5..=hi {
                        let n = counts.get(&y).copied().unwrap_or(0);
                        let text =
                            format!("{y}\n{}", if n > 0 { n.to_string() } else { "·".into() });
                        let on = shown.picked(Facet::Year, &y.to_string());
                        let cell =
                            ui.add_enabled(n > 0 && y >= lo, egui::Button::selectable(on, text));
                        if cell.clicked() {
                            let run = match self.year_anchor.filter(|_| shift) {
                                Some(a) => a.min(y)..=a.max(y),
                                None => y..=y,
                            };
                            for r in run.filter(|r| counts.contains_key(r)) {
                                // A Shift-click only adds; a plain click toggles.
                                if !shift || !shown.picked(Facet::Year, &r.to_string()) {
                                    actions.push(Action::TogglePick(Facet::Year, r.to_string()));
                                }
                            }
                            self.year_anchor = Some(y);
                        }
                        if (y + 1) % 5 == 0 {
                            ui.end_row();
                        }
                    }
                });
            });
        ui.separator();
        if ui
            .add_enabled(
                shown.filter(Facet::Year).is_some(),
                egui::Button::new("Clear"),
            )
            .clicked()
        {
            actions.push(Action::ClearPicks(Some(Facet::Year)));
        }
    }

    /// The filter bar's CART switch, from `x` (skin pixels, like [`Self::filter_controls`]),
    /// in a seller crate with copies in the cart: "CART 3 · €41.20", lit while on. Nothing
    /// when it doesn't fit (the filter panel holds it too).
    fn cart_switch(
        &mut self,
        ui: &mut Ui,
        sk: &Skinned,
        (x, y, x_end, h): (f32, f32, f32, f32),
        actions: &mut Vec<Action>,
    ) {
        let on = self.crates.shown().cart_only();
        #[cfg(test)]
        {
            self.cart_drawn = None;
        }
        let Some(label) = self.cart_label() else {
            return;
        };
        let x = x + STYLE_GAP;
        let w = sk.text_width(&label);
        if x + w > x_end {
            return;
        }
        let lcd = color(sk.def.colors.lcd);
        let dim = lerp_color(sk.def.colors.lcd, sk.def.colors.pl_bg, 0.6);
        let ty = y + ((h - sk.def.font.glyph_h as f32) / 2.0).round();
        sk.text(x, ty, &label, if on { lcd } else { dim });
        let r = ui
            .interact(
                sk.rect(x - 1.0, y, w + 2.0, h),
                Id::new("cart_switch"),
                Sense::click(),
            )
            .on_hover_text(if on {
                "Only what's in your cart: click to show every record"
            } else {
                "Click to show and play only the records in your cart"
            });
        #[cfg(test)]
        {
            self.cart_drawn = Some((label, r.rect.center()));
        }
        if r.clicked() {
            actions.push(Action::ToggleCartOnly);
        }
    }

    /// The shown crate's time readout, widest first: "selected/total", then the total only,
    /// each with `+` while an entry's duration isn't known. It counts the whole crate.
    fn time_readout(&self) -> [String; 2] {
        let shown = self.crates.shown();
        let (total, t_unknown) = shown.total_duration();
        let (sel, s_unknown) = shown.selected_duration();
        let plus = |unknown: bool| if unknown { "+" } else { "" };
        let total = format!("{}{}", format::clock(total), plus(t_unknown));
        [
            format!("{}{}/{total}", format::clock(sel), plus(s_unknown)),
            total,
        ]
    }

    /// "CART 3 · €41.20": the shown seller crate's part of the cart, when it has one (or the
    /// switch is on).
    fn cart_label(&self) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let seller = self.crates.seller_of(self.crates.shown_id())?;
            let d = self.dig.as_ref()?;
            let part = d
                .cart
                .sellers
                .iter()
                .find(|(s, _)| s.eq_ignore_ascii_case(seller))
                .map(|(_, c)| c.clone())
                .unwrap_or_default();
            if part.count == 0 && !self.crates.shown().cart_only() {
                return None;
            }
            Some(if part.subtotal.is_empty() {
                format!("CART {}", part.count)
            } else {
                format!("CART {} · {}", part.count, part.subtotal)
            })
        }
        #[cfg(target_arch = "wasm32")]
        None
    }

    /// A filter's list: a search field, one checkbox per value with its number of records,
    /// and Clear.
    fn facet_list(&mut self, ui: &mut Ui, f: Facet, list: &FacetCounts, actions: &mut Vec<Action>) {
        ui.set_min_width(220.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.facet_search[f as usize])
                .hint_text(format!("Filter {}s…", f.name()))
                .desired_width(f32::INFINITY),
        );
        let needle = self.facet_search[f as usize].trim().to_lowercase();
        let shown = self.crates.shown();
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                for (value, n) in list.iter() {
                    if !needle.is_empty() && !value.to_lowercase().contains(&needle) {
                        continue;
                    }
                    let mut on = shown.picked(f, value);
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut on, value.as_str()).clicked() {
                            actions.push(Action::TogglePick(f, value.clone()));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.weak(n.to_string())
                        });
                    });
                }
            });
        ui.separator();
        if ui
            .add_enabled(shown.filter(f).is_some(), egui::Button::new("Clear"))
            .clicked()
        {
            actions.push(Action::ClearPicks(Some(f)));
        }
    }

    /// The column header: names (with the sort arrow), a click sorts, dragging a divider
    /// resizes the column to its left, and a right-click shows or hides columns.
    fn column_header(
        &mut self,
        ui: &mut Ui,
        sk: &Skinned,
        head: Rect,
        cols: &[(Col, f32, f32)],
        font: &egui::FontId,
        actions: &mut Vec<Action>,
    ) {
        let d = sk.def;
        let s = sk.scale;
        let painter = sk.painter.with_clip_rect(head);
        painter.rect_filled(
            head,
            0.0,
            lerp_color(d.colors.pl_text, d.colors.pl_bg, 0.85),
        );
        let label_col = lerp_color(d.colors.pl_text, d.colors.pl_bg, 0.3);
        let sorted = self.crates.shown().sorted();
        let cell = |x: f32, w: f32| {
            Rect::from_min_size(
                pos2(head.left() + x * s, head.top()),
                vec2(w * s, head.height()),
            )
        };
        for &(c, x, w) in cols {
            let name = match c {
                Col::Number => "#",
                Col::Field(f) => f.label(),
            };
            let r = cell(x, w);
            let cell_painter = painter.with_clip_rect(r.shrink2(vec2(1.5 * s, 0.0)));
            let text = cell_painter.text(
                pos2(r.left() + 3.0 * s, r.center().y),
                egui::Align2::LEFT_CENTER,
                name,
                font.clone(),
                label_col,
            );
            // The sort direction, drawn as a small triangle (the font has no arrows).
            if let (Col::Field(f), Some((sf, dir))) = (c, sorted)
                && sf == f
            {
                let (cx, cy, h) = (text.right() + 5.0 * s, r.center().y, 2.5 * s);
                let points = if dir == Dir::Asc {
                    vec![pos2(cx - h, cy + h), pos2(cx + h, cy + h), pos2(cx, cy - h)]
                } else {
                    vec![pos2(cx - h, cy - h), pos2(cx + h, cy - h), pos2(cx, cy + h)]
                };
                cell_painter.add(egui::Shape::convex_polygon(
                    points,
                    label_col,
                    egui::Stroke::NONE,
                ));
            }
        }
        let resp = ui.interact(head, Id::new("pl_head"), Sense::click());
        if resp.clicked()
            && let Some(p) = resp.interact_pointer_pos()
            && let Some(&(Col::Field(f), _, _)) =
                cols.iter().find(|(_, x, w)| cell(*x, *w).contains(p))
        {
            actions.push(Action::Sort(f));
        }
        egui::Popup::context_menu(&resp)
            .id(Id::new("pl_head_menu"))
            .show(|ui| {
                for f in Field::ALL.into_iter().filter(|f| f.hideable()) {
                    let mut on = self.settings.columns.shows(f);
                    if ui.checkbox(&mut on, f.label()).clicked() {
                        actions.push(Action::ToggleColumn(f));
                    }
                }
            });
        // Dividers on each column's right edge (none after the last). Dragging the title's
        // edge resizes the column after it instead, since the title takes what's left.
        for (i, &(c, x, w)) in cols.iter().enumerate().take(cols.len().saturating_sub(1)) {
            let Col::Field(f) = c else { continue };
            let edge = head.left() + (x + w) * s;
            let grip =
                Rect::from_center_size(pos2(edge, head.center().y), vec2(4.0 * s, head.height()));
            painter.vline(
                edge,
                head.y_range(),
                egui::Stroke::new(1.0, label_col.gamma_multiply(0.5)),
            );
            let g = ui
                .interact(grip, Id::new(("pl_col_edge", i)), Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeColumn);
            let dx = g.drag_delta().x / s;
            if dx != 0.0 {
                let share = dx / head.width() * s;
                match (f, cols.get(i + 1)) {
                    (Field::Title, Some(&(Col::Field(next), _, _))) => {
                        actions.push(Action::ResizeColumn(next, -share))
                    }
                    _ => actions.push(Action::ResizeColumn(f, share)),
                }
            }
        }
    }

    /// When the playing entry of the shown crate changes and the previous one was on screen,
    /// the list follows the new one; otherwise the user's scroll is left alone.
    fn follow_playing_entry(&mut self) {
        let shown_id = self.crates.shown_id();
        let now = (shown_id == self.crates.playing_id())
            .then(|| self.crates.shown().current())
            .flatten()
            .map(|id| (shown_id, id));
        if now == self.pl_follow {
            return;
        }
        let shown = self.crates.shown();
        let target = match (self.pl_follow, now) {
            (Some((c0, prev)), Some((c1, new))) if c0 == c1 => shown
                .index_of(prev)
                .map(|i| self.pl_row_of(i))
                .filter(|&r| self.pl_row_in_view(r))
                .and_then(|_| shown.index_of(new)),
            _ => None,
        };
        self.pl_follow = now;
        if let Some(i) = target {
            self.scroll_into_view(i);
        }
    }

    /// The entry menu (right-click) for `e`, or for a record row whose entries are `record`
    /// (selected, so the selection items act on all of them).
    fn entry_menu(
        &self,
        ui: &mut Ui,
        e: &crate::playlist::Entry,
        record: Option<&[EntryId]>,
        actions: &mut Vec<Action>,
    ) {
        let shown = self.crates.shown();
        let waiting = matches!(e.status, EntryStatus::Waiting(_));
        let playable = match record {
            Some(ids) => ids
                .iter()
                .any(|&id| shown.get(id).is_some_and(|x| x.status.in_play_order())),
            None => e.status.in_play_order(),
        };
        if ui
            .add_enabled(
                playable,
                egui::Button::new(if waiting { "Arm" } else { "Play" }),
            )
            .clicked()
        {
            actions.push(match record {
                Some(ids) => Action::PlayRecord(ids.to_vec()),
                None => Action::PlayEntry(e.id),
            });
            ui.close();
        }
        // The Discogs crates mirror the account: records leave them with Remove from wantlist
        // or Remove from collection, never one track at a time; a label crate keeps them all.
        let discogs = self.crates.is_locked(self.crates.shown_id());
        if !discogs && ui.button("Remove").clicked() {
            actions.push(Action::RemoveEntry(e.id));
            ui.close();
        }
        // A record row already is its album.
        let album = shown.album_of(e.id).len();
        if album > 0 && record.is_none() {
            let more = album > 1;
            let tracks = if album == 1 { "track" } else { "tracks" };
            if !discogs
                && ui
                    .add_enabled(
                        more,
                        egui::Button::new(format!("Remove album ({album} {tracks})")),
                    )
                    .clicked()
            {
                actions.push(Action::RemoveAlbum(e.id));
                ui.close();
            }
            if ui
                .add_enabled(more, egui::Button::new("Select album"))
                .clicked()
            {
                actions.push(Action::SelectAlbum(e.id));
                ui.close();
            }
        }
        self.filter_items(ui, e, actions);
        ui.separator();
        ui.menu_button("Send to crate", |ui| {
            for (id, name) in self.send_targets() {
                let readable = !self.crates.is_unreadable(id);
                if ui.add_enabled(readable, egui::Button::new(name)).clicked() {
                    actions.push(Action::SendTo(e.id, Some(id)));
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("New crate…").clicked() {
                actions.push(Action::SendTo(e.id, None));
                ui.close();
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        self.dig_entry_menu(ui, e, actions);
        ui.separator();
        ui.menu_button("Crate", |ui| self.crate_items(ui, actions));
    }

    /// The shown crate's own actions, from a row's Crate ▸ or a right-click below the rows.
    fn crate_items(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        if ui.button("Select all").clicked() {
            actions.push(Action::SelectAll);
        }
        if ui.button("Select none").clicked() {
            actions.push(Action::SelectNone);
        }
        if ui.button("Invert selection").clicked() {
            actions.push(Action::InvertSelection);
        }
        // The Discogs crates lose records only through the Discogs items.
        if !self.crates.is_locked(self.crates.shown_id()) {
            ui.separator();
            if ui.button("Remove selected").clicked() {
                actions.push(Action::RemoveSelected);
            }
            if ui.button("Clear crate").clicked() {
                actions.push(Action::Clear);
            }
        }
        ui.separator();
        ui.menu_button("Sort", |ui| {
            for f in Field::ALL {
                if ui.button(f.label()).clicked() {
                    actions.push(Action::Sort(f));
                    ui.close();
                }
            }
        });
        let mut grouped = self.crates.shown().is_grouped();
        if ui
            .checkbox(&mut grouped, "Group by record")
            .on_hover_text("One row per record, opening to its tracks (Shift+G)")
            .clicked()
        {
            actions.push(Action::ToggleGrouped);
            ui.close();
        }
        // Adding music, where the crate can take it, and its M3U out of every crate.
        if !self.crates.is_locked(self.crates.shown_id()) {
            ui.separator();
            if ui
                .button(ADD_FILES)
                .on_hover_text("Audio files, or M3U/M3U8 playlists")
                .clicked()
            {
                actions.push(Action::AddFiles);
            }
            if ui.button("Add folder…").clicked() {
                actions.push(Action::AddFolder);
            }
        }
        ui.separator();
        if ui.button("Export M3U…").clicked() {
            actions.push(Action::ExportM3u(self.crates.shown_id()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let pages = self.cart_pages();
        #[cfg(not(target_arch = "wasm32"))]
        if !pages.is_empty() {
            ui.separator();
        }
        #[cfg(not(target_arch = "wasm32"))]
        for page in pages {
            if ui
                .button(format!("Open cart on {}", site_of(page)))
                .clicked()
            {
                actions.push(Action::Dig(DigAction::Seller(
                    crate::app::sellers::SellerAction::OpenCart(page),
                )));
                ui.close();
            }
        }
    }

    /// The carts the list menu opens: Discogs' in a seller crate or while it holds a copy,
    /// and each store basket that holds an item.
    #[cfg(not(target_arch = "wasm32"))]
    fn cart_pages(&self) -> Vec<&'static str> {
        use ::dig::stores::Store;
        let Some(d) = &self.dig else {
            return Vec::new();
        };
        let mut pages = Vec::new();
        if self.crates.seller_of(self.crates.shown_id()).is_some() || !d.cart.items.is_empty() {
            pages.push(CART_PAGE);
        }
        let carts = &d.stores.carts;
        for (store, basket) in [(Store::Juno, &carts.juno), (Store::Bleep, &carts.bleep)] {
            if !basket.is_empty() {
                pages.extend(::dig::carts::cart_page(store));
            }
        }
        pages
    }

    /// The entry menu's filter shortcuts: keep only this record's artist, label or a style,
    /// where the crate offers that filter; where it offers no label filter (a label crate),
    /// search its label or catalogue number.
    fn filter_items(&self, ui: &mut Ui, e: &crate::playlist::Entry, actions: &mut Vec<Action>) {
        let Some(o) = e.origin.as_ref() else {
            return;
        };
        let mut items = Vec::new();
        for (f, what) in [(Facet::Artist, "artist"), (Facet::Label, "label")] {
            if let Some(v) = e.values(f).next().filter(|_| self.offers(f)) {
                items.push((
                    format!("Only this {what}"),
                    Action::OnlyPick(f, v.to_owned()),
                ));
            }
        }
        if !self.offers(Facet::Label) {
            for text in [o.label.trim(), o.catno.trim()] {
                if !text.is_empty() {
                    items.push((format!("Search {text}"), Action::SearchFor(text.to_owned())));
                }
            }
        }
        let styles: Vec<&str> = if self.offers(Facet::Style) {
            e.values(Facet::Style).collect()
        } else {
            Vec::new()
        };
        if !items.is_empty() || !styles.is_empty() {
            ui.separator();
        }
        for (label, a) in items {
            if ui.button(label).clicked() {
                actions.push(a);
                ui.close();
            }
        }
        if !styles.is_empty() {
            ui.menu_button("Only this style", |ui| {
                for st in styles {
                    if ui.button(st).clicked() {
                        actions.push(Action::OnlyPick(Facet::Style, st.to_owned()));
                        ui.close();
                    }
                }
            });
        }
    }

    /// Whether a listing is in the user's Discogs cart.
    fn copy_in_cart(&self, listing: u64) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self
            .dig
            .as_ref()
            .is_some_and(|d| d.cart.has_listing(listing));
        #[cfg(target_arch = "wasm32")]
        {
            let _ = listing;
            false
        }
    }

    /// A copy for sale under its open record: its cart pill (or SOLD), price, grades and
    /// country. It holds no audio; a double-click opens its listing, and the pill the cart.
    #[allow(clippy::too_many_arguments)]
    fn copy_row(
        &self,
        ui: &mut Ui,
        clip: &egui::Painter,
        rr: Rect,
        (listing, first): (u64, usize),
        font: &egui::FontId,
        colors: &crate::skin::Colors,
        scale: f32,
        actions: &mut Vec<Action>,
    ) {
        let shown = self.crates.shown();
        let Some(c) = shown.copy(listing) else {
            return;
        };
        let in_cart = self.copy_in_cart(listing);
        let owned = shown.entries().get(first).and_then(|e| self.owns(e));
        let col = if c.sold {
            lerp_color(colors.pl_text, colors.pl_bg, 0.5)
        } else {
            color(colors.pl_text)
        };
        let row_clip = clip.with_clip_rect(rr);
        let at = pos2(rr.left() + 3.0 * scale, rr.center().y);
        // An unsold copy carries its cart pill (a record the user owns only to take it out
        // of the cart); a sold one only SOLD.
        let pill = (!c.sold && (in_cart || owned.is_none())).then(|| Pill {
            in_cart,
            pointer: self.pill_pointer(ui, rr),
        });
        let marks = Badges {
            slot: false,
            owned: false,
            cart: false,
            sold: c.sold,
            pill,
            checkout: checkout_page(pill, false, None).map(|page| Checkout {
                page,
                pointer: self.pill_pointer(ui, rr),
            }),
        };
        let (w, hits) = entry_badges(&row_clip, at, marks, None, font, colors, scale);
        row_clip.text(
            at + vec2(w, 0.0),
            egui::Align2::LEFT_CENTER,
            format::copy_line(c),
            font.clone(),
            col,
        );
        let mut tip = String::new();
        if let Some(day) = c.posted.get(..10) {
            tip += &format!("Listed {day}");
        }
        if let Some(was) = c.was_cents {
            tip += &format!("\nWas {}", format::price(was, &c.currency));
        }
        if !c.comments.trim().is_empty() {
            tip += &format!("\n{}", c.comments.trim());
        }
        tip += "\nDouble-click to open it on discogs.com";
        let resp = ui
            .interact(rr, Id::new(("pl_copy", listing)), Sense::click())
            .on_hover_text(tip.trim_start());
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::app::sellers::SellerAction;
            if resp.double_clicked() {
                actions.push(Action::Dig(DigAction::Seller(SellerAction::OpenCopy(
                    listing,
                ))));
            }
            egui::Popup::context_menu(&resp).show(|ui| {
                if ui.button("Open on discogs.com").clicked() {
                    actions.push(Action::Dig(DigAction::Seller(SellerAction::OpenCopy(
                        listing,
                    ))));
                    ui.close();
                }
            });
        }
        #[cfg(target_arch = "wasm32")]
        let _ = resp;
        if let Some(rect) = hits.pill {
            self.cart_pill_click(ui, rect, rr, c, in_cart, owned.as_deref(), actions);
        }
        if let (Some(rect), Some(k)) = (hits.checkout, marks.checkout) {
            self.checkout_click(ui, rect, rr, k.page, listing, actions);
        }
    }

    /// The last row of a seller crate dug 1,000 at a time: a click adds the next 1,000.
    #[allow(clippy::too_many_arguments)]
    fn more_row(
        &self,
        ui: &mut Ui,
        clip: &egui::Painter,
        rr: Rect,
        (next, left): (usize, usize),
        font: &egui::FontId,
        colors: &crate::skin::Colors,
        actions: &mut Vec<Action>,
    ) {
        let resp = ui
            .interact(rr, Id::new("pl_more"), Sense::click())
            .on_hover_text("Read the next copies from Discogs into this crate");
        let col = if resp.hovered() {
            colors.pl_current
        } else {
            colors.pl_text
        };
        let n = |v: usize| format::thousands(v);
        let text = if left > next {
            format!("+ ADD NEXT {} · {} MORE", n(next), n(left))
        } else {
            format!("+ ADD THE LAST {}", n(left))
        };
        clip.with_clip_rect(rr).text(
            pos2(rr.left() + rr.height() / 2.0, rr.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            font.clone(),
            color(col),
        );
        #[cfg(not(target_arch = "wasm32"))]
        if resp.clicked() {
            use crate::app::sellers::SellerAction;
            let id = self.crates.shown_id();
            actions.push(Action::Dig(DigAction::Seller(SellerAction::More(id))));
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (resp, actions);
    }

    /// A store entry's cart pill; in a cart and owned, its tooltip says it is owned first.
    fn store_pill(&self, e: &crate::playlist::Entry) -> Option<StorePill> {
        #[cfg(not(target_arch = "wasm32"))]
        return DiggrApp::store_pill_of(self, e).map(|mut p| {
            if let Some(o) = self.owns(e).filter(|_| p.in_cart) {
                p.tip = format::owned_warning(&o) + &p.tip;
            }
            p
        });
        #[cfg(target_arch = "wasm32")]
        {
            let _ = e;
            None
        }
    }

    /// A store pill's click over the band `rr` of its row, registered after the row's: adds
    /// the entry's store item, takes it out, or (Bleep, several formats) opens their menu.
    fn store_pill_click(
        &self,
        ui: &mut Ui,
        pill: Rect,
        rr: Rect,
        id: EntryId,
        p: &StorePill,
        actions: &mut Vec<Action>,
    ) {
        let hit = Rect::from_x_y_ranges(pill.x_range(), rr.y_range());
        let resp = ui
            .interact(hit, Id::new(("pl_store_pill", id)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(&p.tip);
        #[cfg(not(target_arch = "wasm32"))]
        {
            if p.buy {
                if resp.clicked() {
                    actions.push(Action::Dig(DigAction::Buy(id)));
                }
            } else if p.choose.is_empty() {
                if resp.clicked() {
                    actions.push(Action::Dig(DigAction::StoreCart(id, None)));
                }
            } else {
                egui::Popup::menu(&resp).show(|ui| {
                    for (format, label) in &p.choose {
                        if ui.button(label).clicked() {
                            let a = DigAction::StoreCart(id, Some(format.clone()));
                            actions.push(Action::Dig(a));
                            ui.close();
                        }
                    }
                });
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (resp, actions);
    }

    /// The pointer for a cart pill on the band `rr` of its row, when it is over it and nothing
    /// covers it.
    fn pill_pointer(&self, ui: &Ui, rr: Rect) -> Option<Pos2> {
        ui.rect_contains_pointer(rr)
            .then(|| ui.ctx().pointer_interact_pos())
            .flatten()
    }

    /// A cart pill's click over the band `rr` of its row, registered after the row's so it
    /// takes its own clicks (the row isn't selected, played or opened): + CART adds the copy,
    /// IN CART takes it out. `owned`: which pressing the user owns (see `owns`).
    #[allow(clippy::too_many_arguments)]
    fn cart_pill_click(
        &self,
        ui: &mut Ui,
        pill: Rect,
        rr: Rect,
        copy: &crate::playlist::SaleCopy,
        in_cart: bool,
        owned: Option<&str>,
        actions: &mut Vec<Action>,
    ) {
        let hit = Rect::from_x_y_ranges(pill.x_range(), rr.y_range());
        let what = format!(
            "{} · {}",
            format::price(copy.cents, &copy.currency),
            copy.grades
        );
        let tip = if in_cart {
            format!(
                "{}Remove {what} from your cart",
                owned.map(format::owned_warning).unwrap_or_default()
            )
        } else {
            format!("Add {what} to your cart")
        };
        let resp = ui
            .interact(hit, Id::new(("pl_cart_pill", copy.listing)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tip);
        #[cfg(not(target_arch = "wasm32"))]
        if resp.clicked() {
            use crate::app::sellers::SellerAction;
            actions.push(Action::Dig(DigAction::Seller(if in_cart {
                SellerAction::CartRemove(copy.listing)
            } else {
                SellerAction::CartAdd(vec![copy.listing])
            })));
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (resp, actions);
    }

    /// The » pill's click over the band `rr` of its row, registered after the row's as the
    /// cart pill's is: opens the cart `page` to pay there.
    fn checkout_click(
        &self,
        ui: &mut Ui,
        pill: Rect,
        rr: Rect,
        page: &'static str,
        salt: impl std::hash::Hash + std::fmt::Debug,
        actions: &mut Vec<Action>,
    ) {
        let hit = Rect::from_x_y_ranges(pill.x_range(), rr.y_range());
        let resp = ui
            .interact(hit, Id::new(("pl_checkout", salt)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(format!("Check out on {}", site_of(page)));
        #[cfg(not(target_arch = "wasm32"))]
        if resp.clicked() {
            use crate::app::sellers::SellerAction;
            actions.push(Action::Dig(DigAction::Seller(SellerAction::OpenCart(page))));
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (resp, actions);
    }

    /// The open card's record row and the frame's rows around it: (row index, row, whether a
    /// record comes before it, whether one comes after).
    fn card_record(&self) -> Option<(usize, records::RecordRow, bool, bool)> {
        let c = self.card.as_ref()?;
        let rows = self.pl_list();
        let at = rows
            .iter()
            .position(|r| matches!(r, ListRow::Record(rec) if rec.key == c.key))?;
        let ListRow::Record(rec) = &rows[at] else {
            return None;
        };
        let is_record = |r: &ListRow| matches!(r, ListRow::Record(_));
        let prev = rows[..at].iter().any(is_record);
        let next = rows[at + 1..].iter().any(is_record);
        Some((at, rec.clone(), prev, next))
    }

    /// The card's text for record `rec`, from what's in memory.
    fn card_details(&self, rec: &records::RecordRow) -> format::RecordCard {
        let shown = self.crates.shown();
        let members: Vec<&crate::playlist::Entry> =
            rec.members.iter().map(|&i| &shown.entries()[i]).collect();
        let copies = match rec.key {
            AlbumKey::Release(r) => shown.copies_of(r),
            _ => Vec::new(),
        };
        let playing = shown
            .current()
            .filter(|_| self.crates.shown_id() == self.crates.playing_id())
            .filter(|_| self.position.state != PlayState::Stopped)
            .map(|id| (id, self.position.state == PlayState::Paused));
        let armed = self
            .armed
            .filter(|(c, _)| *c == self.crates.shown_id())
            .map(|(_, id)| id);
        format::record_details(
            &members,
            &self.dig_marks(members[0]),
            &copies,
            playing,
            armed,
            unix_now(),
        )
    }

    /// Draws the open card centred over the window (which it dims and keeps from the pointer:
    /// only its × and Esc close it).
    fn show_card(&mut self, ctx: &egui::Context, scale: f32, actions: &mut Vec<Action>) {
        let now = ctx.input(|i| i.time);
        self.card_follow(now);
        let Some((_, rec, prev, next)) = self.card_record() else {
            self.card_rect = None;
            return;
        };
        let card = self.card_details(&rec);
        let (pills, mut pill_actions): (Vec<card::CardPill>, Vec<Option<Action>>) =
            self.card_pills(&rec).into_iter().unzip();
        // Only what's in memory: the cover worker reads files and fetches.
        #[cfg(not(target_arch = "wasm32"))]
        let (thumb, large) = {
            let first = self.crates.shown().entries()[rec.members[0]].clone();
            self.dig
                .as_mut()
                .map_or((None, None), |d| d.covers.card_slot(ctx, &first))
        };
        #[cfg(target_arch = "wasm32")]
        let (thumb, large) = (None, None);
        // The bar follows the record's track that plays (or is paused).
        let shown = self.crates.shown();
        let progress = shown
            .current()
            .filter(|_| self.crates.shown_id() == self.crates.playing_id())
            .filter(|_| self.position.state != PlayState::Stopped)
            .filter(|id| rec.members.iter().any(|&i| shown.entries()[i].id == *id))
            .and_then(|id| {
                let total = self
                    .now_playing_names()
                    .and_then(|(_, _, d)| d)
                    .or_else(|| shown.get(id).and_then(|e| e.duration))?;
                Some((self.position.seconds(), total))
            });
        let slide = self
            .card
            .as_ref()
            .and_then(|c| c.moved)
            .map(|(at, dir)| ((now - at) as f32, dir))
            .filter(|(t, _)| *t < card::SLIDE);
        let view = card::Card {
            card: &card,
            image: card::pick(thumb, large),
            prev,
            next,
            pills,
            progress,
            slide,
        };
        let (rect, clicked) = card::show(ctx, &view, &self.def.colors, scale);
        self.card_rect = Some(rect);
        for a in clicked {
            let action = match a {
                card::CardAction::Play(id) => Some(Action::PlayEntry(id)),
                card::CardAction::Resume => Some(Action::Play),
                card::CardAction::Pause => Some(Action::Pause),
                card::CardAction::Step(step) => Some(Action::CardStep(step)),
                card::CardAction::Close => Some(Action::CloseCard),
                card::CardAction::Seek(t) => Some(Action::Seek(t)),
                card::CardAction::Pill(i) => pill_actions.get_mut(i).and_then(Option::take),
            };
            actions.extend(action);
        }
    }

    /// The record row's actions, as the card's pills, each with what its click does (`None`: a
    /// badge). The same choices the record row makes, so the two never disagree.
    #[cfg(not(target_arch = "wasm32"))]
    fn card_pills(&self, rec: &records::RecordRow) -> Vec<(card::CardPill, Option<Action>)> {
        use crate::app::sellers::SellerAction;
        let mut out = Vec::new();
        if self.dig.is_none() {
            return out;
        }
        let shown = self.crates.shown();
        let first = &shown.entries()[rec.members[0]];
        let ids: Vec<EntryId> = rec.members.iter().map(|&i| shown.entries()[i].id).collect();
        let marks = self.dig_marks(first);
        let pill = |label: &str, hover: Option<&str>, tip: String, lit: bool, button: bool| {
            card::CardPill {
                label: label.to_owned(),
                hover_label: hover.map(str::to_owned),
                tip,
                lit,
                button,
            }
        };
        // The wantlist, for a Discogs record.
        let discogs = first
            .origin
            .as_ref()
            .is_some_and(|o| o.release.is_some() || o.master.is_some());
        if discogs {
            out.push(if marks.wanted {
                (
                    pill(
                        "WANTED",
                        Some("UNWANT"),
                        "On your wantlist: click to take it off".into(),
                        true,
                        true,
                    ),
                    Some(Action::Dig(DigAction::Unwant(ids.clone()))),
                )
            } else {
                (
                    pill(
                        "+ WANT",
                        None,
                        "Add it to your Discogs wantlist".into(),
                        false,
                        true,
                    ),
                    Some(Action::Dig(DigAction::Want(ids.clone()))),
                )
            });
        }
        // The cart: a seller's copy (one, or several to pick from), else a store's or Buy.
        let copies = match rec.key {
            AlbumKey::Release(r) => shown.copies_of(r),
            _ => Vec::new(),
        };
        let unsold: Vec<_> = copies.iter().filter(|c| !c.sold).collect();
        let owned = self.owns(first);
        let store = (unsold.is_empty())
            .then(|| self.store_pill(first))
            .flatten()
            .filter(|p| !(p.buy && marks.cart.is_some()));
        let mut in_cart_pill = None;
        match unsold.as_slice() {
            [c] => {
                let in_cart = self.copy_in_cart(c.listing);
                if in_cart || owned.is_none() {
                    in_cart_pill = Some(in_cart);
                    let what = format!("{} · {}", format::price(c.cents, &c.currency), c.grades);
                    out.push(if in_cart {
                        (
                            pill(
                                "IN CART",
                                Some("REMOVE"),
                                format!(
                                    "{}Remove {what} from your cart",
                                    owned
                                        .as_deref()
                                        .map(format::owned_warning)
                                        .unwrap_or_default()
                                ),
                                true,
                                true,
                            ),
                            Some(Action::Dig(DigAction::Seller(SellerAction::CartRemove(
                                c.listing,
                            )))),
                        )
                    } else {
                        (
                            pill(
                                "+ CART",
                                None,
                                format!("Add {what} to your cart"),
                                false,
                                true,
                            ),
                            Some(Action::Dig(DigAction::Seller(SellerAction::CartAdd(vec![
                                c.listing,
                            ])))),
                        )
                    });
                }
            }
            [] => {
                if let Some(p) = store.as_ref().filter(|p| p.in_cart || owned.is_none()) {
                    in_cart_pill = Some(p.in_cart);
                    let action = if p.buy {
                        DigAction::Buy(first.id)
                    } else {
                        DigAction::StoreCart(first.id, p.choose.first().map(|(f, _)| f.clone()))
                    };
                    let (label, hover) = if p.in_cart {
                        ("IN CART", Some("REMOVE"))
                    } else {
                        ("+ CART", None)
                    };
                    out.push((
                        pill(label, hover, p.tip.clone(), p.in_cart, true),
                        Some(Action::Dig(action)),
                    ));
                }
            }
            several => {
                let tip = if rec.open {
                    "Its copies are listed below the record".to_owned()
                } else {
                    "Open the record in the list to pick a copy".to_owned()
                };
                out.push((
                    pill(
                        &format!("{} COPIES", several.len()),
                        None,
                        tip,
                        marks.cart.is_some(),
                        true,
                    ),
                    (!rec.open).then(|| Action::ToggleRecord(rec.key.clone())),
                ));
            }
        }
        let pill_state = in_cart_pill.map(|in_cart| Pill {
            in_cart,
            pointer: None,
        });
        if let Some(page) = checkout_page(pill_state, marks.cart.is_some(), store.as_ref()) {
            out.push((
                pill(
                    "»",
                    None,
                    format!("Check out on {}", site_of(page)),
                    false,
                    true,
                ),
                Some(Action::Dig(DigAction::Seller(SellerAction::OpenCart(page)))),
            ));
        }
        if let Some(o) = &marks.owned {
            out.push((
                pill("OWNED", None, format!("You own {o}"), false, false),
                None,
            ));
        }
        let sold = (!copies.is_empty() && unsold.is_empty()) || store.is_some_and(|p| p.sold);
        if sold {
            out.push((
                pill("SOLD", None, "Every copy is sold".into(), false, false),
                None,
            ));
        }
        out
    }

    #[cfg(target_arch = "wasm32")]
    fn card_pills(&self, _: &records::RecordRow) -> Vec<(card::CardPill, Option<Action>)> {
        Vec::new()
    }

    /// Steps the card to the previous (-1) or next (1) record: selects it, moves the cursor to
    /// it and scrolls the list the least that shows it. What plays doesn't change. `now` (egui
    /// time) starts its slide.
    fn card_step(&mut self, step: i8, now: f64) {
        let Some((at, _, _, _)) = self.card_record() else {
            return;
        };
        let rows = self.pl_list();
        let is_record = |r: &ListRow| matches!(r, ListRow::Record(_));
        let to = if step < 0 {
            rows[..at].iter().rposition(is_record)
        } else {
            rows[at + 1..]
                .iter()
                .position(is_record)
                .map(|i| at + 1 + i)
        };
        if let Some(to) = to {
            self.card_move_to(to, step, now);
        }
    }

    /// Moves the card to the record row `to` of the frame's rows, coming from direction `dir`.
    fn card_move_to(&mut self, to: usize, dir: i8, now: f64) {
        let rows = self.pl_list();
        let Some(ListRow::Record(rec)) = rows.get(to) else {
            return;
        };
        let p = self.crates.shown_mut();
        let ids: Vec<EntryId> = rec.members.iter().map(|&i| p.entries()[i].id).collect();
        p.select_only(&ids, ids[0]);
        self.scroll_into_view(rec.members[0]);
        if let Some(c) = &mut self.card {
            c.key = rec.key.clone();
            c.following = None;
            c.moved = Some((now, dir));
        }
    }

    /// When the track playing last frame was the card's record's and playback has moved on to
    /// another record (the record ended), the card goes with it.
    fn card_follow(&mut self, now: f64) {
        let Some((at, rec, _, _)) = self.card_record() else {
            return;
        };
        let shown = self.crates.shown();
        let playing = shown
            .current()
            .filter(|_| self.crates.shown_id() == self.crates.playing_id())
            .filter(|_| self.position.state != PlayState::Stopped);
        let in_record = |id: EntryId| rec.members.iter().any(|&i| shown.entries()[i].id == id);
        let following = self.card.as_ref().and_then(|c| c.following);
        let moved_on = playing.filter(|&id| following.is_some_and(|f| f != id) && !in_record(id));
        let target = moved_on.and_then(|id| {
            let i = shown.index_of(id)?;
            let rows = self.pl_list();
            let r = records::row_of(&rows, i);
            matches!(rows.get(r), Some(ListRow::Record(_))).then_some(r)
        });
        if let Some(to) = target {
            let dir = if to > at { 1 } else { -1 };
            self.card_move_to(to, dir, now);
        }
        let shown = self.crates.shown();
        let still = playing.filter(|&id| {
            self.card_record()
                .is_some_and(|(_, r, _, _)| r.members.iter().any(|&i| shown.entries()[i].id == id))
        });
        if let Some(c) = &mut self.card {
            c.following = still;
        }
    }

    /// A record row: its cover, ⏵/⏷, "Artist – Album" with its marks, and a dimmed line with
    /// the pressing, its tracks and what's for sale (or the track playing in it). A click
    /// selects the record, a double-click plays it, ⏵ opens it, and its menu acts on it all; a
    /// record of one entry acts as that entry. Returns the row's response (drag, hover).
    #[allow(clippy::too_many_arguments)]
    fn record_row(
        &self,
        ui: &mut Ui,
        sk: &Skinned,
        clip: &egui::Painter,
        rr: Rect,
        rec: &records::RecordRow,
        cover: Option<CoverSlot>,
        look: &RecordLook,
        actions: &mut Vec<Action>,
    ) -> egui::Response {
        let shown = self.crates.shown();
        let colors = look.colors;
        let scale = sk.scale;
        let font = look.font;
        let first = &shown.entries()[rec.members[0]];
        let ids: Vec<EntryId> = rec.members.iter().map(|&i| shown.entries()[i].id).collect();
        let playing_here = shown
            .current()
            .filter(|c| ids.contains(c))
            .and_then(|c| shown.get(c))
            .filter(|_| self.crates.shown_id() == self.crates.playing_id());
        if ids.iter().all(|&id| shown.is_selected(id)) {
            clip.rect_filled(rr, 0.0, color(colors.pl_selected_bg));
        }
        if self.focus == Focus::Playlist && shown.cursor().is_some_and(|c| ids.contains(&c)) {
            clip.rect_stroke(
                rr,
                0.0,
                egui::Stroke::new(scale, color(colors.pl_text)),
                egui::StrokeKind::Inside,
            );
        }
        let copies = match rec.key {
            AlbumKey::Release(r) => shown.copies_of(r),
            _ => Vec::new(),
        };
        let all_sold = !copies.is_empty() && copies.iter().all(|c| c.sold);
        let (col, _) = row_look(first, playing_here.is_some(), colors);
        let col = if playing_here.is_some() {
            color(colors.pl_current)
        } else if all_sold {
            // Every copy sold: the record stays (it still plays), dimmed.
            lerp_color(
                col.to_array()[..3].try_into().unwrap_or([0; 3]),
                colors.pl_bg,
                0.45,
            )
        } else {
            col
        };
        let dim = lerp_color(
            col.to_array()[..3].try_into().unwrap_or([0; 3]),
            colors.pl_bg,
            0.4,
        );
        // The cover: a square of the row's height.
        let side = rr.height() - 2.0 * scale;
        let cr = Rect::from_min_size(rr.min + vec2(scale, scale), vec2(side, side));
        match cover {
            Some(CoverSlot::Loaded(tex, size)) => {
                let fit = size * (side / size.x.max(size.y).max(1.0));
                clip.image(
                    tex,
                    Rect::from_center_size(cr.center(), fit),
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Some(CoverSlot::Waiting) => {
                clip.rect_stroke(
                    cr,
                    0.0,
                    egui::Stroke::new(scale, dim),
                    egui::StrokeKind::Inside,
                );
            }
            None => {
                clip.circle_stroke(cr.center(), side * 0.4, egui::Stroke::new(scale, dim));
                clip.circle_filled(cr.center(), side * 0.08, dim);
            }
        }
        // The row's area first, so ▸ (added after) takes its own clicks.
        let resp = ui.interact(
            rr,
            Id::new(("pl_record", first.id)),
            Sense::click_and_drag(),
        );
        let x0 = cr.right() + 3.0 * scale;
        let line1 = rr.top() + rr.height() * 0.27;
        let line2 = rr.top() + rr.height() * 0.73;
        let text_clip = clip.with_clip_rect(Rect::from_min_max(pos2(x0, rr.top()), rr.max));
        let mut at = pos2(x0, line1);
        if !rec.single {
            let mark = if rec.open { "⏷" } else { "⏵" };
            let w = text_clip
                .text(at, egui::Align2::LEFT_CENTER, mark, font.clone(), col)
                .width();
            at.x += w + 3.0 * scale;
            let tri = Rect::from_min_max(
                pos2(x0 - 2.0 * scale, rr.top()),
                pos2(at.x, line1 * 2.0 - rr.top()),
            );
            if ui
                .interact(tri, Id::new(("pl_record_open", first.id)), Sense::click())
                .on_hover_text(if rec.open {
                    "Close the record"
                } else {
                    "Open the record"
                })
                .clicked()
            {
                actions.push(Action::ToggleRecord(rec.key.clone()));
            }
        }
        let marks = self.dig_marks(first);
        // A record with one copy left for sale carries that copy's cart pill; with several,
        // CART only says one is in the cart (the copy rows have the pills).
        let mut unsold = copies.iter().filter(|c| !c.sold);
        // The pill sits on the first line: it takes the clicks of that line's half only.
        let band = Rect::from_min_max(rr.min, pos2(rr.right(), rr.center().y));
        let lone = match (unsold.next(), unsold.next()) {
            (Some(c), None) => Some((*c, self.copy_in_cart(c.listing))),
            _ => None,
        };
        // A Discogs copy's pill comes first; with several copies, one that opens the record
        // to its copies' pills; a store's or buy pill otherwise.
        let several = lone.is_none() && copies.iter().filter(|c| !c.sold).count() > 1;
        let store_pill = (lone.is_none() && !several)
            .then(|| self.store_pill(first))
            .flatten()
            .filter(|p| !(p.buy && marks.cart.is_some()));
        // A record the user owns keeps a pill only to take it out of a cart.
        let owned = self.owns(first);
        let pill = lone
            .map(|(_, in_cart)| in_cart)
            .or(several.then_some(marks.cart.is_some()))
            .or(store_pill.as_ref().map(|p| p.in_cart))
            .filter(|&in_cart| in_cart || owned.is_none())
            .map(|in_cart| Pill {
                in_cart,
                // It only opens the record: no REMOVE on hover.
                pointer: self
                    .pill_pointer(ui, band)
                    .filter(|_| !(several && in_cart)),
            });
        let badges = Badges {
            slot: false,
            owned: marks.owned.is_some(),
            cart: marks.cart.is_some(),
            sold: all_sold || store_pill.as_ref().is_some_and(|p| p.sold),
            pill,
            checkout: checkout_page(pill, marks.cart.is_some(), store_pill.as_ref()).map(|page| {
                Checkout {
                    page,
                    pointer: self.pill_pointer(ui, band),
                }
            }),
        };
        let (w, hits) = entry_badges(
            &text_clip,
            at,
            badges,
            first.format_mark(),
            font,
            colors,
            scale,
        );
        at.x += w;
        if let (Some(rect), Some((c, in_cart))) = (hits.pill, lone) {
            self.cart_pill_click(ui, rect, band, c, in_cart, owned.as_deref(), actions);
        }
        if let (Some(rect), Some(p)) = (hits.pill, &store_pill) {
            self.store_pill_click(ui, rect, band, first.id, p, actions);
        }
        if let (Some(rect), Some(k)) = (hits.checkout, badges.checkout) {
            self.checkout_click(ui, rect, band, k.page, first.id, actions);
        }
        if let Some(rect) = hits.pill.filter(|_| several) {
            let hit = Rect::from_x_y_ranges(rect.x_range(), band.y_range());
            let tip = if rec.open {
                "Its copies are listed below"
            } else {
                "Open the record to pick a copy"
            };
            if ui
                .interact(hit, Id::new(("pl_copies_pill", first.id)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(tip)
                .clicked()
                && !rec.open
            {
                actions.push(Action::ToggleRecord(rec.key.clone()));
            }
        }
        let album = if first.album().is_empty() {
            first.title.as_str()
        } else {
            first.album()
        };
        // The record's own credit ("Various" for a compilation), else its first track's.
        let artist = match first.origin.as_ref().map(|o| o.artist.as_str()) {
            Some(a) if !a.is_empty() => a,
            _ => first.artist.as_str(),
        };
        let name = if artist.is_empty() {
            album.to_owned()
        } else {
            format!("{artist} – {album}")
        };
        let failed = marks.wantlist_failed.is_some()
            || marks.collection_failed.is_some()
            || marks.discard_failed.is_some();
        let name = format!(
            "{}{}{name}",
            if failed { "⚑ " } else { "" },
            if marks.wanted { "★ " } else { "" }
        );
        // The record's styles at the right end of its first line; the name stops short of them.
        let styles = first.origin.as_ref().map_or("", |o| o.styles.as_str());
        let mut name_right = rr.right() - 3.0 * scale;
        if !styles.is_empty() {
            let small = egui::FontId::proportional(font.size * 0.9);
            let room = (rr.right() - at.x) * 0.45;
            let style_clip = clip.with_clip_rect(Rect::from_min_max(
                pos2(rr.right() - 3.0 * scale - room, rr.top()),
                rr.max,
            ));
            let w = style_clip
                .text(
                    pos2(rr.right() - 3.0 * scale, line1),
                    egui::Align2::RIGHT_CENTER,
                    styles,
                    small,
                    dim,
                )
                .width()
                .min(room);
            name_right -= w + 8.0 * scale;
        }
        let name_clip = text_clip.with_clip_rect(Rect::from_min_max(
            pos2(x0, rr.top()),
            pos2(name_right.max(at.x), rr.bottom()),
        ));
        let hl = color(colors.pl_current);
        text_marked(&name_clip, at, name, font, col, hl, shown.search());
        // Only listed (one waiting line): its tracklist isn't read yet.
        let listed = rec.total == 1
            && first.status == EntryStatus::Waiting(crate::playlist::WaitKind::Listed);
        let second = match playing_here {
            Some(p) => {
                let sign = if self.position.state == PlayState::Paused {
                    "⏸"
                } else {
                    "⏵"
                };
                let side = p.origin.as_ref().map(|o| o.position.trim()).unwrap_or("");
                [sign, side, p.title.as_str()]
                    .into_iter()
                    .filter(|t| !t.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            None if !copies.is_empty() => format::seller_record_line(
                first.origin.as_ref(),
                (rec.members.len(), rec.total, listed),
                &copies,
            ),
            None => {
                let line = format::record_line(
                    first.origin.as_ref(),
                    rec.members.len(),
                    rec.total,
                    listed,
                );
                // Fully tried with some tracks left without a preview.
                #[cfg(not(target_arch = "wasm32"))]
                let line = match self.record_tried(self.crates.shown_id(), &rec.key) {
                    Some((n, m)) if n > 0 && n < m => format!("{line} · {n}/{m} previews"),
                    _ => line,
                };
                line
            }
        };
        // "tracks coming…" blinks, twice a second, while its record waits for its tracklist.
        let second_col = if listed && playing_here.is_none() {
            let now = ui.input(|i| i.time);
            let till = BLINK - now.rem_euclid(BLINK);
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(till));
            if ((now / BLINK) as u64).is_multiple_of(2) {
                dim
            } else {
                dim.gamma_multiply(0.3)
            }
        } else {
            dim
        };
        text_clip.text(
            pos2(x0, line2),
            egui::Align2::LEFT_CENTER,
            second,
            egui::FontId::proportional(font.size * 0.9),
            second_col,
        );
        let menu_click = opens_context_menu(
            resp.secondary_clicked(),
            resp.clicked(),
            ui.input(|i| i.modifiers),
        );
        if resp.double_clicked() {
            actions.push(if rec.single {
                Action::PlayEntry(first.id)
            } else {
                Action::PlayRecord(ids.clone())
            });
        } else if resp.clicked() && !menu_click {
            actions.push(if rec.single {
                Action::Select(rec.members[0], look.mods)
            } else {
                Action::SelectRecord(ids.clone(), look.mods)
            });
            // A plain click opens (or closes) its card; with Shift or Cmd it only selects.
            if !look.mods.shift && !look.mods.command {
                actions.push(Action::OpenCard(rec.key.clone()));
            }
        } else if menu_click && !ids.iter().all(|&id| shown.is_selected(id)) {
            // The menu acts on the selection, which becomes the record.
            actions.push(Action::SelectRecord(ids.clone(), ClickMods::default()));
        }
        let open = if menu_click {
            Some(egui::SetOpenCommand::Bool(true))
        } else if resp.clicked() {
            Some(egui::SetOpenCommand::Bool(false))
        } else {
            None
        };
        let record = (!rec.single).then_some(ids.as_slice());
        egui::Popup::context_menu(&resp)
            .open_memory(open)
            .show(|ui| self.entry_menu(ui, first, record, actions));
        resp
    }

    /// The filter bar's search field, `w` skin pixels wide from `x`: an invisible egui text
    /// field takes the typing (so no shortcut fires), and the text is drawn in the skin font in
    /// its sunken box.
    fn search_field(
        &mut self,
        ui: &mut Ui,
        fsk: &Skinned,
        x: f32,
        w: f32,
        actions: &mut Vec<Action>,
    ) {
        let (y, h) = draw_field_box(fsk, x, w);
        // Cleared elsewhere (another crate shown, P, ×): the field follows.
        if !crate::playlist::fold_words(&self.pl_search).is_empty()
            && self.crates.shown().search().is_empty()
        {
            self.pl_search.clear();
        }
        let before = self.pl_search.clone();
        let rect = fsk.rect(x, y, w, h);
        let out = ui
            .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                let v = ui.visuals_mut();
                // egui paints selected glyphs in the selection stroke colour, not `text_color`.
                v.selection.bg_fill = Color32::TRANSPARENT;
                v.selection.stroke = egui::Stroke::NONE;
                v.text_cursor.stroke = egui::Stroke::NONE;
                egui::TextEdit::singleline(&mut self.pl_search)
                    .id(Id::new("pl_search"))
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .text_color(Color32::TRANSPARENT)
                    .desired_width(rect.width())
                    .show(ui)
            })
            .inner;
        let resp = &out.response;
        if std::mem::take(&mut self.search_focus) {
            resp.request_focus();
        }
        let (enter, esc, down) = ui.input(|i| {
            (
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::ArrowDown),
            )
        });
        if resp.lost_focus() && esc {
            self.pl_search.clear();
        }
        if resp.has_focus() && down {
            resp.surrender_focus();
            actions.push(Action::SearchToList);
        }
        if self.pl_search != before {
            actions.push(Action::SetSearch(self.pl_search.clone()));
        }
        if resp.lost_focus() && enter {
            actions.push(Action::PlayFirstShown);
        }

        // The text (its tail, when longer than the box) and the caret, in the LCD colour.
        let colors = &fsk.def.colors;
        let adv = fsk.def.font.advance as f32;
        let ty = y + ((h - fsk.def.font.glyph_h as f32) / 2.0).round();
        let fits = ((w - 6.0) / adv).floor().max(1.0) as usize;
        let focused = resp.has_focus();
        if self.pl_search.is_empty() && !focused {
            let hint = format!("SEARCH · {PASTE_MODIFIER}+F");
            let hint: String = hint.chars().take(fits).collect();
            fsk.text(x + 3.0, ty, &hint, lerp_color(colors.lcd, [0, 0, 0], 0.6));
            return;
        }
        let chars: Vec<char> = self.pl_search.chars().collect();
        let (caret, other) = out.cursor_range.map_or((chars.len(), chars.len()), |r| {
            (
                r.primary.index.0.min(chars.len()),
                r.secondary.index.0.min(chars.len()),
            )
        });
        let start = caret.saturating_sub(fits.saturating_sub(1));
        let shown: String = chars[start..].iter().take(fits).collect();
        fsk.text(x + 3.0, ty, &shown, color(colors.lcd));
        // The selection: an LCD block, its characters in the field's dark.
        let end = (start + fits).min(chars.len());
        let (lo, hi) = (caret.min(other).max(start), caret.max(other).min(end));
        if focused && lo < hi {
            let sx = x + 3.0 + (lo - start) as f32 * adv;
            fsk.fill(
                fsk.rect(sx - 1.0, y + 2.0, (hi - lo) as f32 * adv + 1.0, h - 4.0),
                color(colors.lcd),
            );
            let sel: String = chars[lo..hi].iter().collect();
            fsk.text(sx, ty, &sel, color(colors.pl_bg));
        }
        if focused {
            let cx = x + 3.0 + (caret - start) as f32 * adv - 1.0;
            fsk.fill(fsk.rect(cx, y + 2.0, 1.0, h - 4.0), color(colors.lcd));
        }
    }

    /// The filter bar's contents across `width` skin pixels: the search field, taking what the
    /// others leave (at least [`SEARCH_MIN_W`], at most [`SEARCH_MAX_SHARE`]), the BPM control, the filter buttons, the CART
    /// switch, and × while the search or a filter is set.
    fn filter_bar(&mut self, ui: &mut Ui, fsk: &Skinned, width: f32, actions: &mut Vec<Action>) {
        let h = fsk.def.pl_filter_h as f32;
        let shown = self.crates.shown();
        // ×'s place is kept while it is hidden, so nothing moves under a dragged handle.
        let clear_w = fsk.def.sprite("bpm_clear").w as f32;
        let (x0, x_end) = (4.0, width - 8.0 - clear_w);
        if shown.is_filtered() {
            let cy = ((h - clear_w) / 2.0).round();
            fsk.sprite("bpm_clear", x_end + 4.0, cy);
            let clear = ui.interact(
                fsk.rect(x_end + 2.0, 0.0, clear_w + 4.0, h),
                Id::new("filters_clear"),
                Sense::click(),
            );
            if clear.clicked() {
                actions.push(Action::ClearFilters);
            }
            clear.on_hover_text("Clear the search and every filter");
        }
        // What the controls want, so the search field takes only the rest.
        let mut want = 0.0;
        if shown.tempo_span().is_some() {
            let (label, core) = bpm_widths(fsk);
            want += label + core;
        }
        if let Some((chips, buttons)) = self.filter_widths(fsk) {
            want += STYLE_GAP + chips.unwrap_or(buttons);
        }
        if let Some(cart) = self.cart_label() {
            want += STYLE_GAP + fsk.text_width(&cart);
        }
        // The time readout takes what the controls and the search's minimum leave: the full
        // "selected/total", the total only, or nothing.
        let room = x_end - x0 - 4.0 - want - SEARCH_MIN_W - 4.0;
        let readout = self
            .time_readout()
            .into_iter()
            .map(|t| {
                let w = fsk.text_width(&t) + 6.0;
                (t, w)
            })
            .find(|(_, w)| *w <= room);
        if let Some((_, w)) = &readout {
            want += w + 4.0;
        }
        let search_w = (x_end - x0 - 4.0 - want)
            .min(width * SEARCH_MAX_SHARE)
            .max(SEARCH_MIN_W);
        self.search_field(ui, fsk, x0, search_w, actions);
        let mut x = x0 + search_w + 4.0;
        if let Some((text, w)) = readout {
            draw_lcd_box(fsk, x, w, &text);
            #[cfg(test)]
            {
                self.readout_drawn = Some((text.clone(), x, w));
            }
            x += w + 4.0;
        } else {
            #[cfg(test)]
            {
                self.readout_drawn = None;
            }
        }
        let bpm_end = self.bpm_control(ui, fsk, (x, 0.0, x_end, h), actions);
        let filters_end = self.filter_controls(ui, fsk, (bpm_end, 0.0, x_end, h), actions);
        self.cart_switch(ui, fsk, (filters_end, 0.0, x_end, h), actions);
    }

    fn playlist_section(&mut self, ui: &mut Ui, origin: Pos2) {
        self.follow_playing_entry();
        let rows = self.pl_rows();
        let d = crate::layout::playlist_def(&self.def, self.pl_geometry().0);
        let width = d.pl_width as f32;
        let list_h = (rows * d.pl_row_h as usize) as f32;
        let mut actions = Vec::new();
        // The rows the BPM filter shows: row r is crate entry `row_map[r]`.
        let row_map = self.crates.shown().shown_rows();
        // The search's words, lit in the rows.
        let words = self.crates.shown().search().to_vec();
        let hl = color(self.def.colors.pl_current);
        let bpm_progress = self.bpm_progress();
        // Wide enough: columns under a header row, which takes the first row.
        let columns = self.pl_columns();
        let visible = self.pl_visible_rows();
        // What the list draws: entries, or grouped, record rows and the tracks of open ones.
        let list_rows = self.pl_list();
        let max_start = records::max_start(&list_rows, visible);
        self.pl_scroll = self.pl_scroll.min(max_start);
        // The card closes once its record isn't shown: another crate, ungrouped, removed or
        // filtered away.
        if let Some(c) = &self.card {
            let shown = c.crate_id == self.crates.shown_id()
                && self.crates.shown().is_grouped()
                && list_rows
                    .iter()
                    .any(|r| matches!(r, ListRow::Record(rec) if rec.key == c.key));
            if !shown {
                self.card = None;
            }
        }
        let card_scale: f32;
        {
            let def = d.clone();
            let sk = self.skinned(&def, ui, origin);
            let scale = sk.scale;
            draw_stretched_bar(&sk, "pl_top", 0.0, width, d.pl_top_h as f32);
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
            let shown = self.crates.shown();
            let count = shown.is_filtered().then(|| (row_map.len(), shown.len()));
            draw_crate_name(&sk, self.crates.name(self.crates.shown_id()), count);
            dim_title(&sk, "pl_titlebar", self.focus == Focus::Playlist);
            // The sidebar does the crate menu's job when it shows; narrower, the title bar opens
            // the menu instead.
            if !self.pl_sidebar() {
                egui::Popup::menu(&title)
                    .id(Id::new("crate_menu"))
                    .show(|ui| self.crate_menu(ui, &mut actions));
            }
            // The filter bar, between the title bar and the list.
            let fsk = Skinned {
                origin: sk.origin + vec2(0.0, d.pl_top_h as f32 * scale),
                ..self.skinned(&def, ui, origin)
            };
            draw_stretched_bar(&fsk, "pl_filter", 0.0, width, d.pl_filter_h as f32);
            self.filter_bar(ui, &fsk, width, &mut actions);
            let top = (d.pl_top_h + d.pl_filter_h) as f32;
            sk.sprite_in("pl_left", sk.rect(0.0, top, 12.0, list_h));
            sk.sprite_in(
                "pl_right",
                sk.rect(d.pl_width as f32 - 20.0, top, 20.0, list_h),
            );
            // The crate sidebar takes the left of the list area.
            let mut l = d.at("pl_list");
            let side = self.pl_sidebar().then(|| {
                let area = (l.x as f32, top, SIDEBAR_W as f32, list_h);
                l.x += SIDEBAR_W;
                l.w -= SIDEBAR_W;
                area
            });
            let list = sk.rect(l.x as f32, l.y as f32, l.w as f32, list_h);
            sk.fill(list, color(d.colors.pl_bg));
            card_scale = scale;

            // Rows.
            let font = egui::FontId::proportional(9.5 * scale);
            let clip = sk.painter.with_clip_rect(list);
            let row_h = d.pl_row_h as f32;
            let mods = ui.input(|i| ClickMods {
                shift: i.modifiers.shift,
                command: i.modifiers.command,
            });
            let mut drop_target = None;
            let side_drop = side.and_then(|area| {
                self.crate_sidebar(ui, &sk, area, d.pl_row_h as f32, &font, &mut actions)
            });
            // The column header, then the rows.
            let head_top = top;
            // Grouped, the record rows span the width: no header (☰ › Sort still sorts).
            let header = self.pl_header();
            let rows_top = head_top + header as usize as f32 * row_h;
            let cols = if columns {
                // The number column fits the biggest number (entries keep their crate numbers).
                let digits = self.crates.shown().len().max(1).to_string().len() as f32;
                let cols = self
                    .settings
                    .columns
                    .layout(l.w as f32, digits * 6.0 + 10.0);
                if header {
                    let head = sk.rect(l.x as f32, head_top, l.w as f32, row_h);
                    self.column_header(ui, &sk, head, &cols, &font, &mut actions);
                }
                cols
            } else {
                Vec::new()
            };
            let shown = self.crates.shown();
            if row_map.is_empty() && shown.is_filtered() {
                let y = rows_top + ((row_h - sk.def.font.glyph_h as f32) / 2.0).round();
                sk.text(
                    l.x as f32 + 3.0,
                    y,
                    "NO TRACKS MATCH",
                    color(d.colors.pl_text),
                );
                ui.interact(
                    sk.rect(l.x as f32, rows_top, l.w as f32, row_h),
                    Id::new("pl_no_match"),
                    Sense::hover(),
                )
                .on_hover_text(if shown.picks_filter() {
                    "☰ › Show all records shows every style, artist, label and format"
                } else {
                    "× after the BPM range, or ☰ › Show all tempos, shows every track"
                });
            }
            // The hint's links: the list's own click area below the rows takes their clicks.
            let mut hint_links: Vec<(Rect, Rect, HintLink, [u8; 3])> = Vec::new();
            // An empty crate says how to fill it (not the Discogs crates: a paste can't).
            if shown.is_empty() && !self.crates.is_locked(self.crates.shown_id()) {
                let line_h = sk.def.font.glyph_h as f32 + 4.0;
                let body = list_h - (rows_top - top);
                #[cfg(not(target_arch = "wasm32"))]
                let friend: Option<Vec<HintLine>> = self
                    .friend_hint(self.crates.shown_id())
                    .map(|f| f.into_iter().map(HintLine::Text).collect())
                    .or_else(|| self.seller_hint(self.crates.shown_id()));
                #[cfg(target_arch = "wasm32")]
                let friend: Option<Vec<HintLine>> = None;
                let lines: Vec<HintLine> = match friend {
                    Some(f) => f,
                    None => {
                        let long = store_hint(PASTE_MODIFIER, self.setup_links());
                        // Too short a list for the stores: the four short lines.
                        if line_h * long.len() as f32 <= body {
                            long
                        } else {
                            empty_crate_hint(PASTE_MODIFIER)
                                .into_iter()
                                .map(HintLine::Text)
                                .collect()
                        }
                    }
                };
                let mut y = rows_top + ((body - line_h * lines.len() as f32) / 2.0).round();
                let gap = sk.text_width(" ");
                for line in lines {
                    match line {
                        HintLine::Text(t) => {
                            let x = l.x as f32 + ((l.w as f32 - sk.text_width(&t)) / 2.0).round();
                            sk.text(x, y, &t, color(d.colors.pl_text));
                        }
                        HintLine::Links(links) => {
                            let w: f32 = links.iter().map(|k| sk.text_width(k.label)).sum::<f32>()
                                + gap * 2.0 * (links.len() - 1) as f32;
                            let mut x = l.x as f32 + ((l.w as f32 - w) / 2.0).round();
                            for k in links {
                                let tint = readable(k.color, d.colors.pl_bg);
                                let w = sk.text_width(k.label);
                                sk.text(x, y, k.label, color(tint));
                                hint_links.push((
                                    sk.rect(x, y - 1.0, w, line_h - 2.0),
                                    sk.rect(x, y + line_h - 4.0, w, 1.0),
                                    k,
                                    tint,
                                ));
                                x += w + gap * 2.0;
                            }
                        }
                        HintLine::Inline(parts) => {
                            let width = |p: &HintPart| match p {
                                HintPart::Text(t) => sk.text_width(t),
                                HintPart::Link(k) => sk.text_width(k.label),
                            };
                            let w: f32 = parts.iter().map(width).sum::<f32>()
                                + gap * (parts.len() - 1) as f32;
                            let mut x = l.x as f32 + ((l.w as f32 - w) / 2.0).round();
                            for p in &parts {
                                let pw = width(p);
                                match p {
                                    HintPart::Text(t) => {
                                        sk.text(x, y, t, color(d.colors.pl_text));
                                    }
                                    HintPart::Link(k) => {
                                        let tint = readable(k.color, d.colors.pl_bg);
                                        sk.text(x, y, k.label, color(tint));
                                        hint_links.push((
                                            sk.rect(x, y - 1.0, pw, line_h - 2.0),
                                            sk.rect(x, y + line_h - 4.0, pw, 1.0),
                                            *k,
                                            tint,
                                        ));
                                    }
                                }
                                x += pw + gap;
                            }
                        }
                    }
                    y += line_h;
                }
            }
            // While an entry's menu is open, the rest of its album is tinted.
            self.pl_tint = list_rows
                .iter()
                .skip(self.pl_scroll)
                .take(records::fit(&list_rows, self.pl_scroll, visible))
                .filter_map(|r| match r {
                    ListRow::Entry { idx, .. } => Some(*idx),
                    ListRow::Record(_) | ListRow::Copy { .. } | ListRow::More { .. } => None,
                })
                .find(|&idx| egui::Popup::is_id_open(ui.ctx(), row_menu_id(idx)))
                .map(|idx| {
                    let id = shown.entries()[idx].id;
                    let mut album = shown.album_of(id);
                    album.retain(|&a| a != id);
                    album
                })
                .unwrap_or_default();
            let tint = lerp_color(d.colors.pl_selected_bg, d.colors.pl_bg, 0.65);
            let mut y = 0usize;
            // The record rows drawn, for their covers.
            let mut in_view: Vec<usize> = Vec::new();
            // The last list row in view, where a record's run of tracks is cut.
            let last_in_view = {
                let (mut y, mut last) = (0usize, self.pl_scroll);
                for (i, row) in list_rows.iter().enumerate().skip(self.pl_scroll) {
                    if y > 0 && y + row.units() > visible {
                        break;
                    }
                    y += row.units();
                    last = i;
                }
                last
            };
            // Each record's cart mark, drawn across its run: registered after every row, so
            // the rows under it don't take its clicks.
            let mut run_marks: Vec<RunMark> = Vec::new();
            for (i, row) in list_rows.iter().enumerate().skip(self.pl_scroll) {
                if y > 0 && y + row.units() > visible {
                    break;
                }
                let at_y = y;
                y += row.units();
                let (idx, track) = match row {
                    ListRow::Entry { idx, track } => (*idx, *track),
                    ListRow::More { next, left, .. } => {
                        let rr = sk.rect(
                            l.x as f32,
                            rows_top + at_y as f32 * row_h,
                            l.w as f32,
                            row_h,
                        );
                        self.more_row(
                            ui,
                            &clip,
                            rr,
                            (*next, *left),
                            &font,
                            &d.colors,
                            &mut actions,
                        );
                        continue;
                    }
                    ListRow::Copy { listing, first } => {
                        let indent = if columns {
                            0.0
                        } else {
                            records::RECORD_UNITS as f32 * row_h
                        };
                        let rr = sk.rect(
                            l.x as f32 + indent,
                            rows_top + at_y as f32 * row_h,
                            l.w as f32 - indent,
                            row_h,
                        );
                        self.copy_row(
                            ui,
                            &clip,
                            rr,
                            (*listing, *first),
                            &font,
                            &d.colors,
                            scale,
                            &mut actions,
                        );
                        continue;
                    }
                    ListRow::Record(rec) => {
                        in_view.push(rec.members[0]);
                        let top = rows_top + at_y as f32 * row_h;
                        let rr = sk.rect(
                            l.x as f32,
                            top,
                            l.w as f32,
                            records::RECORD_UNITS as f32 * row_h,
                        );
                        let first = &shown.entries()[rec.members[0]];
                        #[cfg(not(target_arch = "wasm32"))]
                        let cover = self.dig.as_mut().and_then(|d| d.covers.row_slot(first));
                        #[cfg(target_arch = "wasm32")]
                        let cover = None;
                        let look = RecordLook {
                            colors: &d.colors,
                            font: &font,
                            mods,
                        };
                        let resp =
                            self.record_row(ui, &sk, &clip, rr, rec, cover, &look, &mut actions);
                        if resp.drag_started() {
                            self.pl_drag_from = Some(rec.members[0]);
                            self.pl_drag_track = None;
                            self.pl_drag_block = (!rec.single).then(|| {
                                rec.members.iter().map(|&i| shown.entries()[i].id).collect()
                            });
                        }
                        if self.pl_drag_from.is_some()
                            && self.pl_drag_track.is_none()
                            && ui.rect_contains_pointer(rr)
                        {
                            drop_target = Some(rec.members[0]);
                            clip.rect_filled(
                                Rect::from_min_size(rr.min, vec2(rr.width(), scale)),
                                0.0,
                                Color32::WHITE,
                            );
                        }
                        continue;
                    }
                };
                let Some(e) = shown.entries().get(idx) else {
                    break;
                };
                // A track of an open record sits under its record, past the cover.
                let indent = if track && !columns {
                    records::RECORD_UNITS as f32 * row_h
                } else {
                    0.0
                };
                let rr = sk.rect(
                    l.x as f32 + indent,
                    rows_top + at_y as f32 * row_h,
                    l.w as f32 - indent,
                    row_h,
                );
                if shown.is_selected(e.id) {
                    clip.rect_filled(rr, 0.0, color(d.colors.pl_selected_bg));
                } else if self.pl_tint.contains(&e.id) {
                    clip.rect_filled(rr, 0.0, tint);
                }
                if self.focus == Focus::Playlist && shown.cursor() == Some(e.id) {
                    clip.rect_stroke(
                        rr,
                        0.0,
                        egui::Stroke::new(scale, color(d.colors.pl_text)),
                        egui::StrokeKind::Inside,
                    );
                }
                let current = shown.current() == Some(e.id);
                let (mut col, mut dur) = row_look(e, current, &d.colors);
                // Where the audio comes from: "YT", "BC" (nothing for local files).
                let source = e
                    .origin
                    .as_ref()
                    .and_then(|o| o.source())
                    .map(|s| s.badge());
                let marks = self.dig_marks(e);
                // Being searched for: SEARCHING… at its end, pulsing about once a second.
                if marks.searching {
                    let t = ui.input(|i| i.time) as f32;
                    let k = 0.5 + 0.5 * (t * std::f32::consts::TAU).sin();
                    let bright = if current {
                        d.colors.pl_current
                    } else {
                        d.colors.pl_text
                    };
                    col = lerp_color(bright, d.colors.pl_bg, 0.55 * (1.0 - k));
                    dur = RowEnd::Text("SEARCHING…".into());
                    ui.ctx().request_repaint_after(Duration::from_millis(50));
                }
                let owned = marks.owned.clone();
                // Already in the Discogs cart: its CART badge says so, no buy pill. A record
                // the user owns keeps a pill only to take it out of a cart.
                let store_pill = self
                    .store_pill(e)
                    .filter(|p| !(p.buy && marks.cart.is_some()))
                    .filter(|p| p.in_cart || self.owns(e).is_none());
                // One cart mark per record: its pill (or CART) and », centred on the part
                // of its run of tracks in view, between lines; each track keeps the slot so
                // the titles line up. An open record's tracks leave it to the record row.
                let (first, last) = record_run(&list_rows, shown.entries(), i);
                let (from, to) = (first.max(self.pl_scroll), last.min(last_in_view));
                let band = sk.rect(
                    l.x as f32 + indent,
                    rows_top + (at_y + from - i) as f32 * row_h,
                    l.w as f32 - indent,
                    (to + 1 - from) as f32 * row_h,
                );
                if last > first {
                    let line = lerp_color(d.colors.pl_text, d.colors.pl_bg, 0.7);
                    for y in [
                        (i == first).then_some(rr.top()),
                        (i == last).then_some(rr.bottom()),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        clip.rect_filled(
                            Rect::from_x_y_ranges(rr.x_range(), y - scale / 2.0..=y + scale / 2.0),
                            0.0,
                            line,
                        );
                    }
                }
                let pill = store_pill.as_ref().filter(|_| !track).map(|p| Pill {
                    in_cart: p.in_cart,
                    pointer: self.pill_pointer(ui, band),
                });
                let mark = Badges {
                    cart: pill.is_none() && marks.cart.is_some(),
                    pill,
                    checkout: checkout_page(pill, marks.cart.is_some(), store_pill.as_ref())
                        .filter(|_| !track)
                        .map(|page| Checkout {
                            page,
                            pointer: self.pill_pointer(ui, band),
                        }),
                    ..Default::default()
                };
                let slot = mark.pill.is_some() || mark.cart;
                let badges = Badges {
                    owned: owned.is_some(),
                    cart: marks.cart.is_some() && !slot,
                    sold: store_pill.as_ref().is_some_and(|p| p.sold),
                    slot,
                    ..Default::default()
                };
                let mut hits = BadgeHits::default();
                // Where the slot's cell is, to draw the mark across the run in it.
                let mut slot_cell = rr;
                if marks.passed {
                    let [r, g, b, _] = col.to_array();
                    col = lerp_color([r, g, b], d.colors.pl_bg, 0.55);
                }
                let failed = marks.wantlist_failed.is_some()
                    || marks.collection_failed.is_some()
                    || marks.discard_failed.is_some();
                let sync = self.settings.sync;
                let bpm_wait = self.bpm_wait(e, &bpm_progress);
                let dim = {
                    let [r, g, b, _] = col.to_array();
                    lerp_color([r, g, b], d.colors.pl_bg, 0.55)
                };
                let row_name = |e: &crate::playlist::Entry| {
                    let name = e.row_name();
                    if sync.on && sync.speed(e.bpm).is_none() {
                        name + " (own tempo)"
                    } else {
                        name
                    }
                };
                let marked = |name: String| {
                    format!(
                        "{}{}{name}{}",
                        if failed { "⚑ " } else { "" },
                        if marks.wanted { "★ " } else { "" },
                        if marks.wantlist_pending {
                            " (wantlist pending)"
                        } else {
                            ""
                        }
                    )
                };
                // Where the "needs yt-dlp" icon is drawn, if it is: clicking it asks again.
                let needs_tool = matches!(dur, RowEnd::Icon(NEEDS_TOOL));
                let mut tool_icon = None;
                if columns {
                    for &(c, x, w) in &cols {
                        let cell = Rect::from_min_size(
                            pos2(rr.left() + x * scale, rr.top()),
                            vec2(w * scale, rr.height()),
                        );
                        let cell_clip = clip.with_clip_rect(cell.shrink2(vec2(1.5 * scale, 0.0)));
                        let text = match c {
                            Col::Number => format!("{}.", idx + 1),
                            Col::Field(Field::Time) => {
                                let w = draw_row_end(&sk, &cell_clip, cell, &dur, &font, col);
                                tool_icon = needs_tool.then(|| icon_rect(cell, scale));
                                if let Some(src) = source {
                                    let right = cell.right() - w - 6.0 * scale;
                                    source_badge(
                                        &cell_clip,
                                        right,
                                        cell.center().y,
                                        src,
                                        &font,
                                        col,
                                        scale,
                                    );
                                }
                                continue;
                            }
                            Col::Field(Field::Title)
                                if badges.any() || e.format_mark().is_some() =>
                            {
                                let at = pos2(cell.left() + 3.0 * scale, cell.center().y);
                                let (w, r) = entry_badges(
                                    &cell_clip,
                                    at,
                                    badges,
                                    e.format_mark(),
                                    &font,
                                    &d.colors,
                                    scale,
                                );
                                hits = r;
                                slot_cell = cell_clip.clip_rect();
                                text_marked(
                                    &cell_clip,
                                    at + vec2(w, 0.0),
                                    marked(e.title.clone()),
                                    &font,
                                    col,
                                    hl,
                                    &words,
                                );
                                continue;
                            }
                            Col::Field(Field::Title) => marked(e.title.clone()),
                            Col::Field(Field::Bpm) if let Some(w) = bpm_wait => {
                                let at = pos2(cell.left() + 3.0 * scale, cell.center().y);
                                draw_bpm_wait(&cell_clip, at, w, scale, col, dim);
                                continue;
                            }
                            Col::Field(f) => crate::columns::cell_text(e, f),
                        };
                        text_marked(
                            &cell_clip,
                            pos2(cell.left() + 3.0 * scale, cell.center().y),
                            text,
                            &font,
                            col,
                            hl,
                            &words,
                        );
                    }
                } else {
                    let mut dur_w = draw_row_end(&sk, &clip, rr, &dur, &font, col);
                    tool_icon = needs_tool.then(|| icon_rect(rr, scale));
                    if let Some(src) = source {
                        let right = rr.right() - dur_w - 9.0 * scale;
                        dur_w += source_badge(&clip, right, rr.center().y, src, &font, col, scale);
                    }
                    // The BPM analysis, in its own place before the time (a long name would
                    // hide it).
                    if let Some(w) = bpm_wait {
                        dur_w += BPM_BAR_W * scale + 6.0 * scale;
                        let at = pos2(rr.right() - dur_w, rr.center().y);
                        draw_bpm_wait(&clip, at, w, scale, col, dim);
                    }
                    let name_clip = clip.with_clip_rect(Rect::from_min_max(
                        rr.min,
                        pos2(rr.right() - dur_w - 8.0 * scale, rr.max.y),
                    ));
                    let at = pos2(rr.left() + 3.0 * scale, rr.center().y);
                    if badges.any() || e.format_mark().is_some() {
                        // The number, the badges, then the name.
                        let num = name_clip
                            .text(
                                at,
                                egui::Align2::LEFT_CENTER,
                                format!("{}. ", idx + 1),
                                font.clone(),
                                col,
                            )
                            .width();
                        let at = at + vec2(num, 0.0);
                        let (w, r) = entry_badges(
                            &name_clip,
                            at,
                            badges,
                            e.format_mark(),
                            &font,
                            &d.colors,
                            scale,
                        );
                        hits = r;
                        slot_cell = name_clip.clip_rect();
                        text_marked(
                            &name_clip,
                            at + vec2(w, 0.0),
                            marked(row_name(e)),
                            &font,
                            col,
                            hl,
                            &words,
                        );
                    } else {
                        text_marked(
                            &name_clip,
                            at,
                            format!("{}. {}", idx + 1, marked(row_name(e))),
                            &font,
                            col,
                            hl,
                            &words,
                        );
                    }
                }
                if let Some(x) = hits.slot.filter(|_| i == from) {
                    let cell = Rect::from_x_y_ranges(slot_cell.x_range(), band.y_range());
                    let painter = clip.with_clip_rect(cell.intersect(clip.clip_rect()));
                    let at = pos2(x, band.center().y);
                    let (_, h) = entry_badges(&painter, at, mark, None, &font, &d.colors, scale);
                    run_marks.push(RunMark {
                        hits: h,
                        band,
                        id: e.id,
                        store_pill: store_pill.clone(),
                        checkout: mark.checkout,
                    });
                }
                let mut resp = ui.interact(rr, row_id(idx), Sense::click_and_drag());
                // Everything known about the entry, built only for the row under the pointer.
                if resp.hovered() && self.pl_drag_from.is_none() {
                    // A track under its open record leaves the record's facts to that row's card.
                    let mut details = format::track_details(e, marks.clone(), unix_now(), !track);
                    if let Some(w) = bpm_wait {
                        details.push(("Tempo", w.label()));
                    }
                    // While a source limits requests, what waits for it says so.
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(source) = self.waiting_for(e)
                        && let Some((_, status)) = details.iter_mut().find(|(k, _)| *k == "Status")
                    {
                        *status += &format!(" · waiting for {}", source.name());
                    }
                    resp = resp.on_hover_ui(move |ui| entry_tooltip(ui, &details));
                }
                let menu_click = opens_context_menu(
                    resp.secondary_clicked(),
                    resp.clicked(),
                    ui.input(|i| i.modifiers),
                );
                if menu_click {
                    // The album's tint shows from the next frame on.
                    ui.ctx().request_repaint();
                }
                let on_tool_icon = tool_icon
                    .zip(resp.interact_pointer_pos())
                    .is_some_and(|(r, p)| r.expand(3.0 * scale).contains(p));
                if resp.double_clicked() {
                    actions.push(Action::PlayEntry(e.id));
                } else if resp.clicked() && on_tool_icon && !menu_click {
                    #[cfg(not(target_arch = "wasm32"))]
                    actions.push(Action::Dig(DigAction::AskYtDlp));
                } else if resp.clicked() && !menu_click {
                    actions.push(Action::Select(idx, mods));
                } else if menu_click && !shown.is_selected(e.id) {
                    // The menu acts on the selection, which becomes the clicked entry.
                    actions.push(Action::Select(idx, ClickMods::default()));
                }
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
                    .show(|ui| self.entry_menu(ui, e, None, &mut actions));
                if resp.drag_started() {
                    self.pl_drag_from = Some(idx);
                    self.pl_drag_block = None;
                    // A track of an open record stays in its record.
                    self.pl_drag_track = track.then(|| e.album_key()).flatten();
                }
                // Where a drag may land: a record moves between rows, a track within its own.
                let lands = match (&self.pl_drag_block, &self.pl_drag_track) {
                    (Some(_), _) => !track,
                    (None, Some(k)) => track && e.album_key().as_ref() == Some(k),
                    (None, None) => !track,
                };
                if self.pl_drag_from.is_some() && lands && ui.rect_contains_pointer(rr) {
                    drop_target = Some(idx);
                    clip.rect_filled(
                        Rect::from_min_size(rr.min, vec2(rr.width(), scale)),
                        0.0,
                        Color32::WHITE,
                    );
                }
            }
            for m in &run_marks {
                if let (Some(rect), Some(p)) = (m.hits.pill, &m.store_pill) {
                    self.store_pill_click(ui, rect, m.band, m.id, p, &mut actions);
                }
                if let (Some(rect), Some(k)) = (m.hits.checkout, m.checkout) {
                    self.checkout_click(ui, rect, m.band, k.page, m.id, &mut actions);
                }
            }
            // Below the last row (all of an empty crate's list): the crate's own menu.
            let below = rows_top + y as f32 * row_h;
            let mut below_click = None;
            if below < top + list_h {
                let rect = sk.rect(l.x as f32, below, l.w as f32, top + list_h - below);
                let resp = ui.interact(rect, Id::new("pl_below"), Sense::click());
                let open = opens_context_menu(
                    resp.secondary_clicked(),
                    resp.clicked(),
                    ui.input(|i| i.modifiers),
                )
                .then_some(egui::SetOpenCommand::Bool(true));
                below_click = resp
                    .clicked()
                    .then(|| resp.interact_pointer_pos())
                    .flatten();
                egui::Popup::context_menu(&resp)
                    .open_memory(open)
                    .show(|ui| self.crate_items(ui, &mut actions));
            }
            self.pl_hint_links = hint_links
                .iter()
                .map(|(r, _, k, _)| (k.label, *r))
                .collect();
            for (r, underline, k, tint) in hint_links {
                if ui.rect_contains_pointer(r) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    ui.painter().rect_filled(underline, 0.0, color(tint));
                }
                #[cfg(not(target_arch = "wasm32"))]
                if below_click.is_some_and(|p| r.contains(p)) {
                    actions.push(match k.to {
                        HintTo::AddFiles => Action::AddFiles,
                        HintTo::Url(url) => Action::Dig(DigAction::OpenUrl(url.to_owned())),
                        HintTo::Extension if self.welcome_on() => {
                            Action::Dig(DigAction::OpenWelcome)
                        }
                        HintTo::Extension => Action::Dig(DigAction::OpenBrowserDialog),
                        HintTo::Discogs => Action::Dig(DigAction::OpenWelcome),
                        HintTo::Seller => Action::Dig(DigAction::Seller(
                            crate::app::sellers::SellerAction::Dig(self.crates.shown_id()),
                        )),
                    });
                }
            }
            if let Some(from) = self.pl_drag_from
                && ui.input(|i| i.pointer.any_released())
            {
                let shown = self.crates.shown();
                match (drop_target, self.pl_drag_block.take()) {
                    (Some(to), Some(block)) => {
                        let before = shown.entries().get(to).map(|e| e.id);
                        actions.push(Action::MoveBlock(block, before));
                    }
                    (Some(to), None) => actions.push(Action::Move(from, to)),
                    (None, _) => {
                        if let Some(c) = side_drop
                            && let Some(e) = shown.entries().get(from)
                        {
                            actions.push(Action::DropOnCrate(e.id, c));
                        }
                    }
                }
                self.pl_drag_from = None;
                self.pl_drag_track = None;
            }
            // The covers of the record rows in view, top first.
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(dig) = &mut self.dig {
                let shown = self.crates.shown();
                let es: Vec<&crate::playlist::Entry> = in_view
                    .iter()
                    .filter_map(|&i| shown.entries().get(i))
                    .collect();
                dig.covers.want_in_view(&es);
            }
            #[cfg(target_arch = "wasm32")]
            let _ = in_view;
            // Scrolling adds up, so a trackpad's small steps move the list too. The rows sit
            // on top of the list, so "over the list" is where the pointer is, not what egui
            // reports as hovered.
            if ui.rect_contains_pointer(list) && self.card.is_none() {
                let dy = ui.input(|i| i.smooth_scroll_delta.y);
                self.pl_scroll = scroll_rows(
                    &mut self.pl_scroll_acc,
                    self.pl_scroll,
                    max_start,
                    dy,
                    row_h * scale,
                );
            } else {
                self.pl_scroll_acc = 0.0;
            }

            // Scrollbar.
            let sc = d.at("pl_scroll");
            let thumb = d.sprite("pl_scroll_thumb");
            let travel = (list_h - thumb.h as f32).max(0.0);
            let max_scroll = max_start;
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
            // The edge under the last row, like the one at the left.
            let edge_h = d.sprite("pl_bottom_edge").h as f32;
            sk.sprite_in("pl_bottom_edge", sk.rect(0.0, top + list_h, width, edge_h));
        }
        self.show_card(ui.ctx(), card_scale, &mut actions);
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
            NameFor::ExportLabel(from) => {
                let to = self.crates.create(name)?;
                self.export_label(*from, to);
                self.show_crate(to);
            }
        }
        Ok(())
    }

    /// "Delete crate "X" (40 entries)?" before deleting a crate that has entries.
    fn delete_dialog(&mut self, ctx: &egui::Context) {
        let Some(id) = self.confirm_delete else {
            return;
        };
        let label = self.crates.is_label(id);
        let question = if label {
            format!(
                "Stop following \"{}\" and delete its crate ({})?",
                self.crates.name(id),
                entries_label(self.crates.entry_count(id))
            )
        } else {
            format!(
                "Delete crate \"{}\" ({})?",
                self.crates.name(id),
                entries_label(self.crates.entry_count(id))
            )
        };
        let mut choice = None;
        let modal = egui::Modal::new(Id::new("delete-crate")).show(ctx, |ui| {
            ui.label(question);
            ui.horizontal(|ui| {
                if ui
                    .button(if label { "Delete label" } else { "Delete" })
                    .clicked()
                {
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
        // ⏵ marks the crate only while its track plays or is paused.
        let live = self.position.state != PlayState::Stopped;
        // The user's crates, then their Discogs crates, the labels, the sellers and the
        // friends, each under its heading, in the sidebar's order.
        let seller_of: HashMap<CrateId, (bool, bool)> = self
            .sidebar_sellers()
            .into_iter()
            .map(|(id, dug, busy)| (id, (dug, busy)))
            .collect();
        let friends = self.sidebar_friends();
        let friend_of: HashMap<CrateId, (bool, bool, bool)> = friends
            .iter()
            .map(|&(id, dug, private, busy)| (id, (dug, private, busy)))
            .collect();
        let (mine, collection) = self.side_crates(&seller_of, &friend_of);
        let discogs = collection.clone();
        let infos = |ids: Vec<CrateId>| -> Vec<crate::crates::CrateInfo> {
            ids.into_iter()
                .filter_map(|id| self.crates.info(id).cloned())
                .collect()
        };
        let groups = [
            ("", mine),
            ("Discogs", collection),
            ("Labels", self.crates.labels().cloned().collect()),
            (
                "Top sellers",
                infos(self.sidebar_sellers().into_iter().map(|s| s.0).collect()),
            ),
            ("Friends", infos(friends.iter().map(|f| f.0).collect())),
        ];
        // The shown crate is scrolled into view as the menu opens: over its first frames, as
        // the first one only sizes the popup.
        let (seen, since) = (Id::new("crate_menu_frame"), Id::new("crate_menu_opened"));
        let frame = ui.ctx().cumulative_frame_nr();
        let was_open = ui.ctx().data(|d| d.get_temp::<u64>(seen)) == Some(frame.wrapping_sub(1));
        let opened = ui.ctx().data_mut(|d| {
            d.insert_temp(seen, frame);
            if !was_open {
                d.insert_temp(since, frame);
            }
            d.get_temp::<u64>(since).unwrap_or(frame)
        });
        let opening = frame - opened < 3;
        egui::ScrollArea::vertical()
            .animated(false)
            .max_height(ui.ctx().content_rect().height() * 2.0 / 3.0)
            .show(ui, |ui| {
                for (heading, group) in groups {
                    if group.is_empty() {
                        continue;
                    }
                    if !heading.is_empty() {
                        ui.separator();
                        ui.label(egui::RichText::new(heading).weak().small());
                    }
                    for c in group {
                        let unreadable = self.crates.is_unreadable(c.id);
                        let label = crate_menu_label(
                            &c.name,
                            c.id == shown,
                            live && c.id == playing,
                            unreadable,
                        );
                        let resp = ui.add_enabled(
                            !unreadable,
                            egui::Button::selectable(c.id == shown, label),
                        );
                        if opening && c.id == shown {
                            resp.scroll_to_me(Some(egui::Align::Center));
                        }
                        if resp.clicked() {
                            actions.push(Action::ShowCrate(c.id));
                        }
                    }
                }
            });
        // Refresh wantlist / collection, for the Discogs crates (with a token).
        #[cfg(not(target_arch = "wasm32"))]
        if (self.dig.as_ref().is_some_and(|d| d.has_token()) || self.fan_imported())
            && !discogs.is_empty()
        {
            ui.separator();
            for c in &discogs {
                self.dig_refresh_item(ui, c, actions);
            }
        }
        if discogs.iter().any(|c| c.collection) && ui.button("Open Insights").clicked() {
            actions.push(Action::OpenInsights);
            ui.close();
        }
        #[cfg(target_arch = "wasm32")]
        let _ = discogs;
        // Refresh friend, for a dug friend's crate on screen.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(r) = self
            .friend_rows()
            .into_iter()
            .find(|r| r.crate_id == shown && r.dug && !r.private)
        {
            ui.separator();
            let label = if r.busy {
                "Refreshing…"
            } else {
                "Refresh friend"
            };
            if ui.add_enabled(!r.busy, egui::Button::new(label)).clicked() {
                actions.push(Action::Dig(DigAction::Friend(
                    crate::app::friends::FriendAction::Refresh(shown),
                )));
                ui.close();
            }
        }
        ui.separator();
        if ui.button("New crate…").clicked() {
            actions.push(Action::NewCrate);
        }
        if self.crates.is_label(shown) {
            ui.separator();
            self.label_menu(ui, shown, actions);
            return;
        }
        if self.crates.is_discogs(shown) {
            return;
        }
        let editable = shown != PLAYLIST;
        if ui
            .add_enabled(editable, egui::Button::new("Rename crate…"))
            .clicked()
        {
            actions.push(Action::RenameCrate(shown));
        }
        if ui
            .add_enabled(editable, egui::Button::new("Delete crate…"))
            .clicked()
        {
            actions.push(Action::DeleteCrate(shown));
        }
    }

    /// Export to crate: every entry of label crate `from` copied into `to`, in order, without
    /// what `to` already holds. The label crate is unchanged.
    fn export_label(&mut self, from: CrateId, to: CrateId) {
        if !self.crates.load(from) || self.refuse_discogs_insert(to) {
            return;
        }
        let ids: Vec<EntryId> = self
            .crates
            .get(from)
            .map(|p| p.entries().iter().map(|e| e.id).collect())
            .unwrap_or_default();
        match self.crates.send(from, &ids, to) {
            Ok(n) => {
                self.mark_crate(to);
                self.notify(format!(
                    "{} › {}: {}",
                    self.crates.name(from),
                    self.crates.name(to),
                    match n {
                        0 => "nothing new".to_owned(),
                        n => entries_label(n),
                    }
                ));
            }
            Err(e) => self.notify(e),
        }
    }

    /// A label crate's menu, in the sidebar and the title-bar crate menu: Delete label…,
    /// Export to crate ▸ (the normal crates, then New crate…), Refresh label, and Download all
    /// tracks.
    fn label_menu(&self, ui: &mut Ui, c: CrateId, actions: &mut Vec<Action>) {
        if ui.button("Delete label…").clicked() {
            actions.push(Action::DeleteLabel(c));
            ui.close();
        }
        ui.menu_button("Export to crate", |ui| {
            for (id, name) in self.send_targets() {
                if id == c || self.crates.seller_of(id).is_some() {
                    continue;
                }
                if ui.button(name).clicked() {
                    actions.push(Action::ExportLabel(c, Some(id)));
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("New crate…").clicked() {
                actions.push(Action::ExportLabel(c, None));
                ui.close();
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        {
            let busy = self.label_refreshing(c);
            if ui
                .add_enabled(
                    !busy,
                    egui::Button::new(if busy {
                        "Refreshing…"
                    } else {
                        "Refresh label"
                    }),
                )
                .on_hover_text("Read the label's page again: only new records come in")
                .clicked()
            {
                actions.push(Action::Dig(DigAction::RefreshLabel(c)));
                ui.close();
            }
            let failed = self.retryable(c).0.len();
            if failed > 0
                && self.label_download().is_none_or(|(d, _, _)| d != c)
                && ui
                    .button(format!("Retry failed tracks ({failed})"))
                    .on_hover_text(
                        "Download the \"clip failed\" tracks again and search the \"not found\" \
                         ones again",
                    )
                    .clicked()
            {
                actions.push(Action::Dig(DigAction::RetryFailed(c)));
                ui.close();
            }
            match self.label_download() {
                Some((d, done, total)) if d == c => {
                    if ui
                        .button(format!("Downloading ({done} of {total})…"))
                        .on_hover_text("Show its progress, and Stop")
                        .clicked()
                    {
                        actions.push(Action::Dig(DigAction::ShowDownload));
                        ui.close();
                    }
                }
                other => {
                    let tip = match other {
                        Some(_) => {
                            "Downloads every preview of this label (stops the other label's)"
                        }
                        None => {
                            "Downloads every preview of this label to the cache, in the background"
                        }
                    };
                    if ui
                        .button("Download all tracks")
                        .on_hover_text(tip)
                        .clicked()
                    {
                        actions.push(Action::Dig(DigAction::DownloadLabel(c)));
                        ui.close();
                    }
                }
            }
        }
        self.export_item(ui, c, actions);
    }

    /// Export M3U… in crate `c`'s own menu: writes that crate, shown or not.
    fn export_item(&self, ui: &mut Ui, c: CrateId, actions: &mut Vec<Action>) {
        if ui
            .add_enabled(
                !self.crates.is_unreadable(c),
                egui::Button::new("Export M3U…"),
            )
            .clicked()
        {
            actions.push(Action::ExportM3u(c));
            ui.close();
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
        #[cfg(not(target_arch = "wasm32"))]
        let clips = match self.crates.load(id) {
            true => self.crates.get(id).map(|p| digging::clips_of(p, |_| true)),
            false => None,
        };
        match self.crates.delete(id) {
            Ok(()) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.dig_crate_deleted(id);
                    self.dig_forget_clips(clips.unwrap_or_default());
                }
            }
            Err(e) => self.notify(e),
        }
    }

    /// Copies entries of the shown crate to another one.
    fn send_to(&mut self, ids: &[EntryId], to: CrateId) {
        if self.refuse_discogs_insert(to) {
            return;
        }
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
            #[cfg(not(target_arch = "wasm32"))]
            Action::Dig(a) => self.dig_act(self.crates.shown_id(), a),
            Action::Prev => {
                self.armed = None;
                self.with_engine(|e| e.previous());
            }
            Action::Play => self.play(),
            Action::Pause => self.with_engine(|e| e.toggle_pause()),
            Action::Stop => self.with_engine(|e| e.stop()),
            Action::Next => self.play_next(),
            Action::Seek(s) => self.with_engine(|e| e.seek(s)),
            Action::Volume(v) => self.set_volume(v),
            Action::Shuffle => {
                self.settings.shuffle = !self.settings.shuffle;
                self.queue_dirty = true;
                self.mark_settings();
            }
            Action::Sync => self.toggle_sync(),
            Action::SyncRange => {
                let sync = &mut self.settings.sync;
                sync.wide = !sync.wide;
                let range = if sync.wide { 16 } else { 8 };
                self.notify(format!("SYNC fits tracks within ±{range}%"));
                self.mark_settings();
            }
            Action::SyncNudge(bpm) => self.nudge_sync(bpm),
            Action::SyncTarget(_) if self.sync_waiting() => {}
            Action::SyncTarget(bpm) => {
                self.settings.sync.target = bpm;
                self.settings.sync = self.settings.sync.sanitized();
                self.mark_settings();
            }
            Action::SyncTakeBpm if self.sync_waiting() => {}
            Action::SyncTakeBpm => match self.playing_bpm() {
                Some(bpm) => {
                    self.settings.sync.target = bpm;
                    self.mark_settings();
                }
                None => self.notify("Not analyzed yet"),
            },
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
            Action::ToggleRemaining => {
                self.settings.time_remaining = !self.settings.time_remaining;
                self.mark_settings();
            }
            Action::ToggleSpectrogram => self.spectro_open = !self.spectro_open,
            Action::OpenInsights => self.insights_open = true,
            Action::SetScale(scale) => {
                let scale = scale.clamp(1, 2);
                if self.settings.scale != scale {
                    self.settings.scale = scale;
                    self.mark_settings();
                }
            }
            Action::SavePreset => self.name_dialog = Some(NameDialog::new(NameFor::Preset, "")),
            Action::DeletePreset(name) => {
                self.presets.remove(&name);
                self.save_presets();
            }
            Action::PlayEntry(id) => self.play_entry(self.crates.shown_id(), id),
            Action::ToggleGrouped => {
                let c = self.crates.shown_id();
                let on = !self.crates.is_grouped(c);
                self.crates.set_grouped(c, on);
                self.mark_crate(c);
                if let Some(i) = self.crates.shown().cursor_index() {
                    self.scroll_into_view(i);
                }
            }
            Action::ToggleRecord(key) => {
                let open = self.pl_open.entry(self.crates.shown_id()).or_default();
                if !open.remove(&key) {
                    open.insert(key);
                }
                self.pl_open_rev += 1;
            }
            Action::OpenCard(key) => {
                let crate_id = self.crates.shown_id();
                let same = self
                    .card
                    .as_ref()
                    .is_some_and(|c| c.crate_id == crate_id && c.key == key);
                self.card = (!same).then_some(OpenCard {
                    crate_id,
                    key,
                    following: None,
                    moved: None,
                });
            }
            Action::CloseCard => self.card = None,
            Action::CardStep(step) => self.card_step(step, ctx.input(|i| i.time)),
            Action::SelectRecord(ids, mods) => {
                self.side_crate = None;
                let p = self.crates.shown_mut();
                if mods.command {
                    p.toggle_ids(&ids);
                } else if mods.shift {
                    let last = ids.iter().filter_map(|&id| p.index_of(id)).max();
                    if let Some(last) = last {
                        p.extend_to(last, &ids, ids[0]);
                    }
                } else {
                    p.select_only(&ids, ids[0]);
                }
            }
            Action::PlayRecord(ids) => {
                let p = self.crates.shown();
                let pick = ids
                    .iter()
                    .copied()
                    .find(|&id| p.get(id).is_some_and(|e| e.status.is_playable()))
                    .or_else(|| {
                        ids.iter()
                            .copied()
                            .find(|&id| p.get(id).is_some_and(|e| e.status.in_play_order()))
                    });
                if let Some(id) = pick {
                    self.play_entry(self.crates.shown_id(), id);
                }
            }
            Action::MoveBlock(ids, before) => {
                self.crates.shown_mut().move_block(&ids, before);
                self.mark_shown();
            }
            Action::Select(i, m) => {
                self.side_crate = None;
                self.crates.shown_mut().click(i, m);
            }
            Action::Move(from, to) => {
                self.crates.shown_mut().move_entry(from, to);
                self.mark_shown();
            }
            Action::ToggleColumn(f) => {
                self.settings.columns.toggle(f);
                self.mark_settings();
            }
            Action::ResizeColumn(f, share) => {
                self.settings.columns.resize(f, share);
                self.mark_settings();
            }
            Action::SetBpmFilter(range) => {
                if self.crates.shown_mut().set_bpm_filter(range) {
                    self.mark_shown();
                    let rows = self.pl_list().len();
                    self.pl_scroll = self.pl_scroll.min(rows.saturating_sub(1));
                }
            }
            Action::TogglePick(f, value) => {
                let p = self.crates.shown_mut();
                let on = !p.picked(f, &value);
                p.set_pick(f, &value, on);
                self.filter_changed();
            }
            Action::ClearPicks(f) => {
                if self.crates.shown_mut().clear_picks(f) {
                    self.filter_changed();
                }
            }
            Action::SetSearch(text) => {
                if self.crates.shown_mut().set_search(&text) {
                    self.filter_changed();
                }
            }
            Action::PlayFirstShown | Action::SearchToList => {
                let p = self.crates.shown();
                let first = p
                    .shown_rows()
                    .into_iter()
                    .map(|i| &p.entries()[i])
                    .find(|e| e.status.in_play_order())
                    .map(|e| e.id);
                if matches!(a, Action::SearchToList) {
                    self.focus = Focus::Playlist;
                    self.crates.shown_mut().set_cursor(first);
                } else if let Some(id) = first {
                    self.apply(Action::PlayEntry(id), ctx);
                }
            }
            Action::ToggleCartOnly => {
                let p = self.crates.shown_mut();
                let on = !p.cart_only();
                p.set_cart_only(on);
                self.filter_changed();
            }
            Action::OnlyPick(f, v) => {
                let p = self.crates.shown_mut();
                p.clear_picks(Some(f));
                p.set_pick(f, &v, true);
                self.filter_changed();
            }
            Action::SearchFor(text) => {
                self.pl_search = text;
                self.search_focus = true;
                if self.crates.shown_mut().set_search(&self.pl_search) {
                    self.filter_changed();
                }
            }
            Action::ClearFilters => {
                if self.crates.shown_mut().clear_filters() {
                    self.filter_changed();
                }
            }
            Action::Sort(field) => {
                // The same field again sorts the other way.
                let p = self.crates.shown_mut();
                let dir = match p.sorted() {
                    Some((f, Dir::Asc)) if f == field => Dir::Desc,
                    _ => Dir::Asc,
                };
                p.sort_by(field, dir);
                if let Some(i) = p.cursor_index() {
                    self.scroll_into_view(i);
                }
                self.mark_shown();
            }
            Action::AddFiles => self.open_files_dialog(Open::Add),
            Action::AddFolder => self.open_folder_dialog(),
            Action::RemoveSelected => self.remove_selected(),
            Action::Clear => self.apply(Action::ClearCrate(self.crates.shown_id()), ctx),
            Action::ClearCrate(id) => {
                if !self.refuse_discogs_edit(id)
                    && self.crates.load(id)
                    && let Some(p) = self.crates.get_mut(id)
                {
                    p.clear();
                    if id == self.crates.playing_id() {
                        self.with_engine(|e| e.stop());
                    }
                    self.mark_crate(id);
                }
            }
            Action::SelectAll => self.crates.shown_mut().select_all(),
            Action::SelectNone => self.crates.shown_mut().select_none(),
            Action::InvertSelection => self.crates.shown_mut().invert_selection(),
            Action::ExportM3u(id) => self.export_m3u(id),
            Action::ShowCrate(id) => self.show_crate(id),
            Action::DeleteLabel(id) => self.confirm_delete = Some(id),
            Action::ExportLabel(from, Some(to)) => self.export_label(from, to),
            Action::ExportLabel(from, None) => {
                let name = self.crates.name(from);
                let name = name.strip_prefix("Label: ").unwrap_or(name).to_owned();
                self.name_dialog = Some(NameDialog::new(NameFor::ExportLabel(from), name));
            }
            Action::ToggleSideGroup(None) => {
                self.settings.crates_folded = !self.settings.crates_folded;
                self.mark_settings();
            }
            Action::ToggleSideGroup(Some(g)) => {
                // An accordion: the open group closes, any other opens alone.
                self.settings.side_open = (self.settings.side_open != Some(g)).then_some(g);
                self.side_scroll[1] = 0;
                self.mark_settings();
            }
            Action::MoveToLabels(id, label) => {
                if self.crates.find_label(label).is_none() {
                    self.crates.set_label(id, label);
                    self.notify(format!(
                        "{} is under LABELS: it fills from its label now",
                        self.crates.name(id)
                    ));
                }
            }
            Action::NewCrate => self.name_dialog = Some(NameDialog::new(NameFor::NewCrate, "")),
            Action::RenameCrate(id) if self.crates.is_discogs(id) => {}
            Action::RenameCrate(id) => {
                let name = self.crates.name(id).to_owned();
                self.name_dialog = Some(NameDialog::new(NameFor::RenameCrate(id), name));
            }
            Action::DeleteCrate(id) if self.crates.is_discogs(id) => {
                self.notify("Your Discogs wantlist and collection can't be deleted")
            }
            Action::DeleteCrate(id) => {
                if self.crates.entry_count(id) > 0 {
                    self.confirm_delete = Some(id);
                } else {
                    self.delete_crate(id);
                }
            }
            Action::RemoveEntry(entry) => {
                if self.crates.shown().is_selected(entry) {
                    self.remove_selected();
                } else {
                    self.remove_entries(&[entry]);
                }
            }
            Action::RemoveAlbum(entry) => {
                let album = self.crates.shown().album_of(entry);
                self.remove_entries(&album);
            }
            Action::SelectAlbum(entry) => {
                let p = self.crates.shown_mut();
                let album = p.album_of(entry);
                if !album.is_empty() {
                    p.select_only(&album, entry);
                }
            }
            Action::DropOnCrate(entry, to) => {
                if to != self.crates.shown_id() {
                    let shown = self.crates.shown();
                    let ids = if shown.is_selected(entry) {
                        shown.selected_ids()
                    } else {
                        vec![entry]
                    };
                    self.send_to(&ids, to);
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

/// A row of the sidebar's Discogs group.
enum SideRow {
    /// CRATES: how many of the user's crates, and whether folded.
    Crates(usize, bool),
    /// "+ New crate".
    New,
    Discogs,
    /// LABELS, TOP SELLERS or FRIENDS: how many, whether open, and whether a crate in it is
    /// being dug or refreshed.
    Group(SideGroup, usize, bool, bool),
    /// A crate; for a seller crate, whether it was dug and is being refreshed.
    Crate(crate::crates::CrateInfo, Option<(bool, bool)>),
}

impl SideRow {
    fn is(&self, id: CrateId) -> bool {
        matches!(self, SideRow::Crate(c, _) if c.id == id)
    }
}

/// The sidebar's scrollbars, in skin pixels.
const SIDE_BAR_W: f32 = 3.0;
/// While entries are dragged over a list's first or last row, it steps a row this often.
const SIDE_EDGE_STEP: Duration = Duration::from_millis(120);

/// The first row that shows `row` in `visible` rows, moving from `first` as little as possible.
fn reveal_row(first: usize, row: usize, visible: usize) -> usize {
    if row < first {
        row
    } else if row >= first + visible {
        row + 1 - visible
    } else {
        first
    }
}

enum Action {
    /// Group the shown crate by record, or show it flat.
    ToggleGrouped,
    /// Open or close a record row of the shown crate.
    ToggleRecord(AlbumKey),
    /// A click on a record row: these entries, with these modifiers.
    SelectRecord(Vec<EntryId>, ClickMods),
    /// Opens the record's card, moves it to the record, or closes it when it's on it already.
    OpenCard(AlbumKey),
    CloseCard,
    /// The card's previous (-1) or next (1) record.
    CardStep(i8),
    /// Plays a record: its first playable track, or arms its first one.
    PlayRecord(Vec<EntryId>),
    /// Moves a record (these entries) before an entry, or to the end.
    MoveBlock(Vec<EntryId>, Option<EntryId>),
    Prev,
    Play,
    Pause,
    Stop,
    Next,
    Seek(f64),
    Volume(f32),
    /// Sort the shown crate by a column (again: the other way).
    Sort(Field),
    /// Show (and play) only the shown crate's entries in this BPM range; `None` shows all.
    SetBpmFilter(Option<(u16, u16)>),
    /// Picks or unpicks a value of the shown crate's style, artist or label filter.
    TogglePick(Facet, String),
    /// Turns a style, artist or label filter off, or all three with `None`.
    ClearPicks(Option<Facet>),
    /// The CART switch of the shown (seller) crate.
    ToggleCartOnly,
    /// The filter bar's search text changed.
    SetSearch(String),
    /// Enter in the search field: play the first shown entry.
    PlayFirstShown,
    /// ↓ in the search field: the list takes the keyboard, its cursor on the first shown entry.
    SearchToList,
    /// The filter bar's ×: the search and every filter off.
    ClearFilters,
    /// Only this value in its filter (the entry menu).
    OnlyPick(Facet, String),
    /// Search this text, the field taking the keyboard (the entry menu).
    SearchFor(String),
    ToggleColumn(Field),
    /// Widen a column by a share of the list's width.
    ResizeColumn(Field, f32),
    /// Remove an entry, or the whole selection when it is part of it (the entry menu).
    RemoveEntry(EntryId),
    /// Remove every entry of this entry's album (the entry menu).
    RemoveAlbum(EntryId),
    /// Select exactly this entry's album, with the cursor on it (the entry menu).
    SelectAlbum(EntryId),
    Shuffle,
    Repeat,
    /// SYNC on or off.
    Sync,
    /// SYNC's range: ±8% or ±16%.
    SyncRange,
    /// SYNC's target, by this many BPM.
    SyncNudge(i32),
    /// SYNC's target, in BPM.
    SyncTarget(u16),
    /// SYNC's target from the playing track.
    SyncTakeBpm,
    /// The EQ window.
    ToggleEq,
    ToggleRemaining,
    /// The size: 1 (classic) or 2 (double).
    SetScale(u8),
    ToggleSpectrogram,
    /// Open Insights, on the collection crate.
    OpenInsights,
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
    /// Export M3U… of this crate (the shown one, or any from its own menu).
    ExportM3u(CrateId),
    ShowCrate(CrateId),
    NewCrate,
    /// Rename or delete a crate (from the crate menu or the sidebar).
    RenameCrate(CrateId),
    DeleteCrate(CrateId),
    /// Fold or unfold CRATES (`None`) or open or close a Discogs group in the sidebar.
    ToggleSideGroup(Option<SideGroup>),
    /// Delete label…: asks, then stops following the label and deletes its crate.
    DeleteLabel(CrateId),
    /// Export to crate: copy a label crate's entries into a crate, or (`None`) a new one.
    ExportLabel(CrateId, Option<CrateId>),
    /// Make a crate filled from a label's page that label's crate (LABELS).
    MoveToLabels(CrateId, u64),
    /// Send the dragged entries (the selection when the dragged one is in it) to a crate.
    DropOnCrate(EntryId, CrateId),
    /// Empty a crate (Clear crate in ≡ acts on the shown one).
    ClearCrate(CrateId),
    /// Send an entry (with the rest of the selection, when it is selected) to a crate, or to
    /// a new one.
    SendTo(EntryId, Option<CrateId>),
    #[cfg(not(target_arch = "wasm32"))]
    Dig(DigAction),
}

/// How opened files are used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Open {
    /// Add to the shown crate.
    Add,
    /// Add to the shown crate and play the first one.
    AddAndPlay,
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
    /// A new crate for a copy of this label crate.
    ExportLabel(CrateId),
}

impl NameFor {
    /// Window title and confirm button.
    fn labels(&self) -> (&'static str, &'static str) {
        match self {
            NameFor::Preset => ("Save EQ preset", "Save"),
            NameFor::NewCrate | NameFor::SendToNew(_) | NameFor::ExportLabel(_) => {
                ("New crate", "Create")
            }
            NameFor::RenameCrate(_) => ("Rename crate", "Rename"),
        }
    }
}

/// "1 record", "3 records".
fn records_label(n: usize) -> String {
    if n == 1 {
        "1 record".into()
    } else {
        format!("{n} records")
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
/// The first visible row after scrolling by `dy` points (positive: towards the top), with rows
/// `row` points high. Partial rows add up in `acc`; they are dropped at either end of the list.
fn scroll_rows(acc: &mut f32, first: usize, max: usize, dy: f32, row: f32) -> usize {
    *acc += dy;
    let steps = (*acc / row).trunc();
    *acc -= steps * row;
    let to = first as isize - steps as isize;
    // Pushing past an end leaves nothing to carry over.
    if (to <= 0 && *acc > 0.0) || (to >= max as isize && *acc < 0.0) {
        *acc = 0.0;
    }
    to.clamp(0, max as isize) as usize
}

/// Seconds since the Unix epoch.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// What the playlist's rows were built from: the crate, its revision, whether it's grouped,
/// and the open records' revision.
type RowsKey = (CrateId, u64, bool, u64, Option<(usize, usize)>);

/// How record rows are drawn this frame.
/// Whose record card is open: a record of a crate, by its album key, so it stays on its
/// record while rows open, close or come in above it.
#[derive(Debug, Clone, PartialEq)]
struct OpenCard {
    crate_id: CrateId,
    key: AlbumKey,
    /// The record's track playing last frame: when playback moves on from it to another
    /// record, the card follows.
    following: Option<EntryId>,
    /// When the card moved to this record (egui time) and which way: its content slides in.
    moved: Option<(f64, i8)>,
}

struct RecordLook<'a> {
    colors: &'a crate::skin::Colors,
    font: &'a egui::FontId,
    mods: ClickMods,
}

/// What an entry's tooltip shows where its record's cover goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverSlot {
    /// The cover, at its own size.
    Loaded(egui::TextureId, egui::Vec2),
    /// The cover's space, empty until it arrives.
    Waiting,
}

/// A track's tooltip: its full name, then one labelled line per known detail (text only: the
/// cover is the record card's).
fn entry_tooltip(ui: &mut Ui, details: &[(&str, String)]) {
    let mut lines = details.iter();
    if let Some((_, name)) = lines.next() {
        ui.label(egui::RichText::new(name).strong());
    }
    egui::Grid::new("entry_details")
        .num_columns(2)
        .spacing(vec2(10.0, 2.0))
        .show(ui, |ui| {
            for (label, value) in lines {
                ui.label(egui::RichText::new(*label).weak());
                ui.label(value);
                ui.end_row();
            }
        });
}

/// What a playlist row shows where its duration goes.
#[derive(Debug, Clone, PartialEq)]
enum RowEnd {
    /// The duration (or nothing while it isn't known).
    Text(String),
    /// A status icon (a skin sprite, tinted with the row's colour).
    Icon(&'static str),
    /// A download's progress, in percent.
    Bar(u8),
}

/// Width of the download bar, in skin pixels.
const ROW_BAR_W: f32 = 30.0;

fn row_look(
    e: &crate::playlist::Entry,
    current: bool,
    colors: &crate::skin::Colors,
) -> (Color32, RowEnd) {
    use crate::playlist::{UnavailableKind as U, WaitKind as W};
    let base = if current {
        colors.pl_current
    } else {
        colors.pl_text
    };
    let dim = lerp_color(base, colors.pl_bg, 0.55);
    match &e.status {
        EntryStatus::Failed => (
            Color32::from_rgb(170, 60, 60),
            RowEnd::Text(duration_text(e)),
        ),
        EntryStatus::Waiting(w) => (
            dim,
            match w {
                W::Listed => RowEnd::Icon("st_listed"),
                // Preparing: on its way, not something to click (unlike "needs yt-dlp").
                W::Queued | W::Search | W::PreparingYtDlp => RowEnd::Icon("st_queued"),
                W::Downloading(p) => RowEnd::Bar(*p),
                W::NeedsYtDlp => RowEnd::Icon(NEEDS_TOOL),
                W::Other(_) => RowEnd::Icon("st_other"),
            },
        ),
        EntryStatus::Unavailable(u) => (
            dim,
            RowEnd::Icon(match u {
                U::NoClip | U::ClipFailed | U::NotFound | U::AlreadyInCrate => "st_unavailable",
                U::Other(_) => "st_other",
            }),
        ),
        _ => (color(base), RowEnd::Text(duration_text(e))),
    }
}

/// Draws a row's end (duration, icon or download bar) against the right of `row`, in `col`;
/// returns its width in points.
/// The "needs yt-dlp" row icon.
const NEEDS_TOOL: &str = "st_needs_tool";

/// Where [`draw_row_end`] puts an icon at the right of `area`.
fn icon_rect(area: Rect, scale: f32) -> Rect {
    let size = 9.0 * scale;
    let right = area.right() - 3.0 * scale;
    Rect::from_min_size(
        pos2(right - size, area.center().y - size / 2.0),
        vec2(size, size),
    )
}

fn draw_row_end(
    sk: &Skinned,
    painter: &egui::Painter,
    row: Rect,
    end: &RowEnd,
    font: &egui::FontId,
    col: Color32,
) -> f32 {
    let s = sk.scale;
    let right = row.right() - 3.0 * s;
    match end {
        RowEnd::Text(t) => painter
            .text(
                pos2(right, row.center().y),
                egui::Align2::RIGHT_CENTER,
                t,
                font.clone(),
                col,
            )
            .width(),
        RowEnd::Icon(name) => {
            let r = icon_rect(row, s);
            sk.sprite_tinted(name, r, col);
            r.width()
        }
        RowEnd::Bar(pct) => {
            let (w, h) = (ROW_BAR_W * s, 3.0 * s);
            let track = Rect::from_min_size(pos2(right - w, row.center().y - h / 2.0), vec2(w, h));
            painter.rect_filled(track, 0.0, col.gamma_multiply(0.35));
            let done = w * (*pct).min(100) as f32 / 100.0;
            painter.rect_filled(Rect::from_min_size(track.min, vec2(done, h)), 0.0, col);
            w
        }
    }
}

fn duration_text(e: &crate::playlist::Entry) -> String {
    e.duration.map(format::clock).unwrap_or_default()
}

/// The playlist title bar's text: the crate name folded to the skin font's upper case, without
/// characters the font lacks, cut to `max_w` skin pixels.
/// The crate's title with, while a BPM filter is on, how many entries show of how many
/// ("LOWTIDE TAPES · 42/301"). The name is shortened first; the count always shows.
pub fn crate_title_with_count(
    def: &crate::skin::SkinDef,
    name: &str,
    count: Option<(usize, usize)>,
    max_w: f32,
) -> String {
    let Some((shown, all)) = count else {
        return crate_title(def, name, max_w);
    };
    let suffix = format!(" · {shown}/{all}");
    let suffix_w = suffix.chars().count() as f32 * def.font.advance as f32;
    let name = crate_title(def, name, max_w - suffix_w);
    format!("{name}{suffix}").trim().to_owned()
}

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
/// The badges before an entry's name: OWNED (dark letters on the skin's amber), then the
/// dim format mark of a record not on vinyl ("FILE", "CD"…), left edge centred on `at`.
/// Returns their width with the gap after them (0 for none).
/// The marks drawn before a row's name.
#[derive(Debug, Clone, Copy, Default)]
struct Badges {
    owned: bool,
    /// A copy is in the user's Discogs cart.
    cart: bool,
    /// No longer for sale (a copy, or a record whose copies all sold).
    sold: bool,
    /// The cart pill of a seller crate's copy, drawn where CART would be (instead of it).
    pill: Option<Pill>,
    /// Keep the cart slot (pill and ») blank: the record's mark is drawn across its run.
    slot: bool,
    /// The » pill after IN CART or CART, to the cart to pay at.
    checkout: Option<Checkout>,
}

/// The » pill: the cart page it opens, and the pointer as for `Pill`.
#[derive(Debug, Clone, Copy)]
struct Checkout {
    page: &'static str,
    pointer: Option<Pos2>,
}

/// Where the badges' pills went, for their clicks.
#[derive(Debug, Clone, Copy, Default)]
struct BadgeHits {
    pill: Option<Rect>,
    checkout: Option<Rect>,
    /// Where a blank cart slot starts.
    slot: Option<f32>,
}

/// A record's cart mark drawn across its run of rows (`band`), whose clicks are registered
/// after all the rows.
struct RunMark {
    hits: BadgeHits,
    band: Rect,
    id: EntryId,
    store_pill: Option<StorePill>,
    checkout: Option<Checkout>,
}

/// The cart a row's » opens: an IN CART pill's (a store's basket, else Discogs'), or the
/// Discogs cart behind a CART badge. None for a row not in a cart.
fn checkout_page(
    pill: Option<Pill>,
    cart_badge: bool,
    store: Option<&StorePill>,
) -> Option<&'static str> {
    match pill {
        Some(p) if p.in_cart => Some(store.and_then(|s| s.cart_page).unwrap_or(CART_PAGE)),
        Some(_) => None,
        None => cart_badge.then_some(CART_PAGE),
    }
}

/// "discogs.com" for `https://www.discogs.com/sell/cart`.
fn site_of(page: &str) -> &str {
    let host = page.split('/').nth(2).unwrap_or(page);
    host.strip_prefix("www.").unwrap_or(host)
}

/// A copy's cart pill: + CART, or IN CART that reads REMOVE while hovered.
#[derive(Debug, Clone, Copy)]
struct Pill {
    in_cart: bool,
    /// The pointer, when it is over the pill's row (and nothing covers it): the pill takes
    /// the row's full height, so only its x matters.
    pointer: Option<Pos2>,
}

/// A store entry's cart pill (see `stores`), drawn as a copy's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StorePill {
    pub(crate) in_cart: bool,
    pub(crate) sold: bool,
    /// Bleep's formats to choose from (more than one, none in the cart): (product id, label).
    pub(crate) choose: Vec<(String, String)>,
    pub(crate) tip: String,
    /// A click buys where the record is known, or opens the Buy chooser.
    pub(crate) buy: bool,
    /// The store's basket page, for » (only for the baskets the app can reach).
    pub(crate) cart_page: Option<&'static str>,
}

impl Badges {
    fn any(self) -> bool {
        self.owned
            || self.cart
            || self.sold
            || self.slot
            || self.pill.is_some()
            || self.checkout.is_some()
    }
}

/// The user's cart on discogs.com, where they pay (one checkout per seller).
pub(crate) const CART_PAGE: &str = "https://www.discogs.com/sell/cart";

/// CART's fill: apart from OWNED's amber, whatever the skin.
const CART_FILL: [u8; 3] = [64, 196, 232];
const SOLD_FILL: [u8; 3] = [196, 84, 84];

/// The checkout pill's label: a word, since » alone says nothing until hovered (and the
/// default fonts have no arrow glyph).
const CHECKOUT_MARK: &str = "PAY »";

/// The pill's labels; it is as wide as the widest, so hovering never moves the row's text.
const PILL_LABELS: [&str; 3] = ["+ CART", "IN CART", "REMOVE"];

/// The badges' font, from the row's.
fn badge_font(font: &egui::FontId) -> egui::FontId {
    egui::FontId::proportional(font.size * 0.78)
}

/// The width of a cart pill's label area: its widest label.
fn pill_text_width(painter: &egui::Painter, font: &egui::FontId) -> f32 {
    PILL_LABELS
        .iter()
        .map(|l| {
            painter
                .layout_no_wrap((*l).into(), badge_font(font), Color32::WHITE)
                .size()
                .x
        })
        .fold(0.0, f32::max)
}

/// The width of a cart pill and its » slot, with the gaps after them.
fn cart_slot_width(painter: &egui::Painter, font: &egui::FontId, scale: f32) -> f32 {
    let pad = 2.5 * scale * 2.0;
    let mark = painter
        .layout_no_wrap(CHECKOUT_MARK.into(), badge_font(font), Color32::WHITE)
        .size()
        .x;
    pill_text_width(painter, font) + pad + 3.0 * scale + mark + pad + 3.0 * scale
}

/// The list rows `first..=last` of row `i`'s run: the flat rows next to it of the same
/// record. A row alone when it has no record, or is a track under its open record.
fn record_run(rows: &[ListRow], entries: &[crate::playlist::Entry], i: usize) -> (usize, usize) {
    let key = |j: usize| match rows.get(j) {
        Some(ListRow::Entry { idx, track: false }) => entries.get(*idx)?.album_key(),
        _ => None,
    };
    let Some(k) = key(i) else {
        return (i, i);
    };
    let mut first = i;
    while first > 0 && key(first - 1).as_ref() == Some(&k) {
        first -= 1;
    }
    let mut last = i;
    while key(last + 1).as_ref() == Some(&k) {
        last += 1;
    }
    (first, last)
}

/// Draws the badges; returns their width with the gap after them (0 for none), and where the
/// pills went, for their clicks.
fn entry_badges(
    painter: &egui::Painter,
    at: Pos2,
    marks: Badges,
    format: Option<&str>,
    font: &egui::FontId,
    colors: &crate::skin::Colors,
    scale: f32,
) -> (f32, BadgeHits) {
    let small = badge_font(font);
    let pad = vec2(2.5 * scale, 0.5 * scale);
    let bg = color(colors.pl_bg);
    let mut x = at.x;
    let mut pill_rect = None;
    if marks.owned {
        x += badge(
            painter,
            x,
            at.y,
            "OWNED",
            color(colors.pl_owned),
            &small,
            bg,
            scale,
        );
    }
    let mut slot = None;
    if marks.slot {
        slot = Some(x);
        x += cart_slot_width(painter, font, scale);
    }
    match marks.pill {
        Some(p) => {
            let galley_h = painter
                .layout_no_wrap("C".into(), small.clone(), bg)
                .size()
                .y;
            let rect = Rect::from_min_size(
                pos2(x, at.y - galley_h / 2.0 - pad.y),
                vec2(pill_text_width(painter, font), galley_h) + pad * 2.0,
            );
            let cart = color(CART_FILL);
            let r = 2.0 * scale;
            let hovered = p.pointer.is_some_and(|q| rect.x_range().contains(q.x));
            let (label, text) = match (p.in_cart, hovered) {
                (true, true) => {
                    painter.rect_filled(rect, r, color(SOLD_FILL));
                    ("REMOVE", bg)
                }
                (true, false) => {
                    painter.rect_filled(rect, r, cart);
                    ("IN CART", bg)
                }
                (false, hovered) => {
                    if hovered {
                        painter.rect_filled(rect, r, cart.gamma_multiply(0.3));
                    }
                    let stroke = egui::Stroke::new(scale, cart);
                    painter.rect_stroke(rect, r, stroke, egui::StrokeKind::Inside);
                    ("+ CART", cart)
                }
            };
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                small.clone(),
                text,
            );
            pill_rect = Some(rect);
            x += rect.width() + 3.0 * scale;
        }
        None if marks.cart => {
            x += badge(
                painter,
                x,
                at.y,
                "CART",
                color(CART_FILL),
                &small,
                bg,
                scale,
            );
        }
        None => {}
    }
    // A cart pill keeps the » slot after it, drawn or not, so a row going in or out of the
    // cart never moves its text (and copy rows stay lined up).
    let mut checkout_rect = None;
    if marks.pill.is_some() || marks.checkout.is_some() {
        let galley =
            painter.layout_no_wrap(CHECKOUT_MARK.into(), small.clone(), Color32::PLACEHOLDER);
        let rect = Rect::from_min_size(
            pos2(x, at.y - galley.size().y / 2.0 - pad.y),
            galley.size() + pad * 2.0,
        );
        if let Some(c) = marks.checkout {
            let cart = color(CART_FILL);
            let r = 2.0 * scale;
            // Blue on black; hovered, the other way round.
            let text = if c.pointer.is_some_and(|q| rect.x_range().contains(q.x)) {
                painter.rect_filled(rect, r, cart);
                bg
            } else {
                painter.rect_filled(rect, r, bg);
                let stroke = egui::Stroke::new(scale, cart);
                painter.rect_stroke(rect, r, stroke, egui::StrokeKind::Inside);
                cart
            };
            painter.galley(rect.min + pad, galley, text);
            checkout_rect = Some(rect);
        }
        x += rect.width() + 3.0 * scale;
    }
    if marks.sold {
        x += badge(
            painter,
            x,
            at.y,
            "SOLD",
            color(SOLD_FILL),
            &small,
            bg,
            scale,
        );
    }
    if let Some(f) = format {
        let fill = lerp_color(colors.pl_text, colors.pl_bg, 0.55);
        x += badge(painter, x, at.y, f, fill, &small, bg, scale);
    }
    (
        x - at.x,
        BadgeHits {
            pill: pill_rect,
            checkout: checkout_rect,
            slot,
        },
    )
}

/// An entry's source at the end of its row ("YT", "BC"), outlined, its right edge at `right`;
/// returns the width it takes.
fn source_badge(
    painter: &egui::Painter,
    right: f32,
    y: f32,
    label: &str,
    font: &egui::FontId,
    col: Color32,
    scale: f32,
) -> f32 {
    let small = badge_font(font);
    let dim = col.gamma_multiply(0.7);
    let pad = vec2(2.0 * scale, 0.5 * scale);
    let galley = painter.layout_no_wrap(label.into(), small, dim);
    let size = galley.size() + pad * 2.0;
    let rect = Rect::from_min_size(pos2(right - size.x, y - size.y / 2.0), size);
    let stroke = egui::Stroke::new(scale * 0.75, dim);
    painter.rect_stroke(rect, 2.0 * scale, stroke, egui::StrokeKind::Inside);
    painter.galley(rect.min + pad, galley, dim);
    size.x + 4.0 * scale
}

/// One filled badge at `x`, centred on `y`; returns its width with the gap after it.
#[allow(clippy::too_many_arguments)]
fn badge(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    label: &str,
    fill: Color32,
    font: &egui::FontId,
    text: Color32,
    scale: f32,
) -> f32 {
    let pad = vec2(2.5 * scale, 0.5 * scale);
    let galley = painter.layout_no_wrap(label.into(), font.clone(), text);
    let rect = Rect::from_min_size(
        pos2(x, y - galley.size().y / 2.0 - pad.y),
        galley.size() + pad * 2.0,
    );
    painter.rect_filled(rect, 2.0 * scale, fill);
    painter.galley(rect.min + pad, galley, text);
    rect.width() + 3.0 * scale
}

/// A momentary skin button at any rectangle (the mini player's buttons have no layout entry).
fn sprite_button(ui: &mut Ui, sk: &Skinned, id: &str, rect: Rect, sprite: &str) -> egui::Response {
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    let name = if resp.is_pointer_button_down_on() {
        format!("{sprite}_p")
    } else {
        sprite.to_owned()
    };
    sk.sprite_in(&name, rect);
    resp
}

/// A window header button, and whether it was clicked with the pointer inside its own rectangle
/// (not merely near it).
fn corner_button(
    ui: &mut Ui,
    sk: &Skinned,
    id: &str,
    rect: Rect,
    sprite: &str,
) -> (egui::Response, bool) {
    let resp = sprite_button(ui, sk, id, rect, sprite);
    let inside = ui
        .input(|i| i.pointer.interact_pos())
        .is_some_and(|p| rect.contains(p));
    let clicked = resp.clicked() && inside;
    (resp, clicked)
}

/// A toggle icon (`{base}_on` / `{base}_off`) at any rectangle: the mini player's icons have
/// no layout entry.
fn icon_toggle(
    ui: &mut Ui,
    sk: &Skinned,
    id: &str,
    rect: Rect,
    base: &str,
    on: bool,
) -> egui::Response {
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    sk.sprite_in(&format!("{base}_{}", if on { "on" } else { "off" }), rect);
    resp
}

/// The band's icon buttons' width and the gap between them (skin pixels).
const ICON_W: f32 = 17.0;
const ICON_GAP: f32 = 2.0;
/// The header's buttons: their gap, the margin at its right end, and their top.
const HB_GAP: f32 = 2.0;
const HB_MARGIN: f32 = 4.0;
const HB_Y: f32 = 2.0;
/// The mini player's LCD icons: ▤, EQ and spectrum.
const LCD_ICONS: f32 = 3.0;
/// The player's rows, in skin pixels from the band's top: the LCD box, the time digits and the
/// icons, the two text lines, and the transport. Volume, shuffle and repeat share the
/// transport's centre line (`BUTTON_ROW_Y` puts their 12 to 13 pixels on it).
const LCD_Y: f32 = 6.0;
const LCD_H: f32 = 34.0;
const DIGITS_Y: f32 = 10.0;
const LINE1_Y: f32 = 13.0;
const LINE2_Y: f32 = 26.0;
const TRANSPORT_Y: f32 = 48.0;
const BUTTON_ROW_Y: f32 = 51.0;
/// The seek bar's top, under the transport, in the band's last pixels.
const SEEK_Y: f32 = 68.0;
/// How long the seek bar's thumb waits at a seek's target for the clock to get there.
const SEEK_SETTLE: Duration = Duration::from_millis(500);
/// Where the volume slider starts, after the transport's five buttons.
const VOLUME_X: f32 = 123.0;

/// Where the mini player's LCD icons start, in a player `w` wide: the track lines end before
/// it. The icons end 4 pixels from the edge.
fn lcd_icons_x(w: f32) -> f32 {
    w - 4.0 - LCD_ICONS * ICON_W - (LCD_ICONS - 1.0) * ICON_GAP
}

/// Shuffle, then repeat, end the button row where the LCD icons end (4 pixels in); the volume
/// slider takes the room between them and the transport.
fn shuffle_x(w: f32) -> f32 {
    w - 4.0 - 2.0 * ICON_W - ICON_GAP
}

/// The EQ window's viewport.
fn eq_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("eq")
}

/// The side that has the keyboard: the player (the band) or the playlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Player,
    Playlist,
}

impl Focus {
    fn other(self) -> Self {
        match self {
            Focus::Player => Focus::Playlist,
            Focus::Playlist => Focus::Player,
        }
    }
}

/// Title bars of the side without the keyboard are drawn dimmed, as classic players did with their
/// inactive windows. Drawn over the bar, before its buttons.
/// Strobe dots on the turntable's rim.
const STROBE_DOTS: usize = 24;
/// The strobe dots' radius (the middle of the platter's rim), the dots of the lock flash, how
/// long the flash and the digits' count last, and how dark the dimmed deck is.
const STROBE_R: f32 = 28.0;
const RIM_DOTS: usize = 72;
const FLASH_RIM_SECS: f32 = 0.6;
const COUNT_SECS: f32 = 0.5;
const DIM_ALPHA: u8 = 125;

fn dim_title(sk: &Skinned, layout: &str, lit: bool) {
    if !lit {
        sk.painter
            .rect_filled(sk.at(layout), 0.0, Color32::from_black_alpha(140));
    }
}

/// The filter bar's sunken LCD box at `x`, `w` wide, centred in the bar: its top and height.
fn draw_field_box(fsk: &Skinned, x: f32, w: f32) -> (f32, f32) {
    let h = fsk.def.sprite("pl_field_fill").h as f32;
    let y = ((fsk.def.pl_filter_h as f32 - h) / 2.0).floor();
    fsk.sprite("pl_field_l", x, y);
    fsk.sprite_in("pl_field_fill", fsk.rect(x + 1.0, y, w - 2.0, h));
    fsk.sprite("pl_field_r", x + w - 1.0, y);
    (y, h)
}

/// A sunken LCD box holding `text`, as the search field draws its own.
fn draw_lcd_box(fsk: &Skinned, x: f32, w: f32, text: &str) {
    let (y, h) = draw_field_box(fsk, x, w);
    let ty = y + ((h - fsk.def.font.glyph_h as f32) / 2.0).round();
    fsk.text(x + 3.0, ty, text, color(fsk.def.colors.lcd));
}

/// A playlist bar (`{base}_l`, `{base}_fill`, `{base}_r`) stretched to `width`: the end pieces
/// as drawn, the middle column tiled between them.
fn draw_stretched_bar(sk: &Skinned, base: &str, y: f32, width: f32, h: f32) {
    let (l, r) = (
        sk.def.sprite(&format!("{base}_l")).w as f32,
        sk.def.sprite(&format!("{base}_r")).w as f32,
    );
    sk.sprite(&format!("{base}_l"), 0.0, y);
    sk.sprite_in(
        &format!("{base}_fill"),
        sk.rect(l, y, (width - l - r).max(0.0), h),
    );
    sk.sprite(&format!("{base}_r"), width - r, y);
}

fn draw_crate_name(sk: &Skinned, name: &str, count: Option<(usize, usize)>) {
    const PAD: f32 = 5.0; // plain title bar either side of the name
    const SIDE: f32 = 40.0; // decoration left visible at each end (the close button's side)
    let bar = sk.def.at("pl_titlebar");
    let text = crate_title_with_count(sk.def, name, count, bar.w as f32 - 2.0 * (SIDE + PAD));
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

/// What the window is doing, for the repaint policy.
#[derive(Debug, Clone, Copy, Default)]
pub struct Activity {
    pub playing: bool,
    /// The waveform section is visible (it scrolls with the playhead).
    pub waveform: bool,
    pub engine_starting: bool,
    pub hidden: bool,
}

/// When to repaint without input. `None` means only on input, so an idle window costs nothing.
pub fn repaint_after(a: Activity) -> Option<Duration> {
    if a.hidden {
        None
    } else if a.playing && a.waveform {
        // The waveform scrolls with the playhead: smooth at the display rate.
        Some(Duration::from_millis(16))
    } else if a.playing {
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
        EngineSlot::Ready(_) => "DIGGR - DROP FILES HERE".into(),
        EngineSlot::Failed(e) => format!("AUDIO ERROR: {e}"),
    }
}

/// What a label crate says when the user tries to add to it or remove from it.
const LABEL_CRATE_HINT: &str =
    "Label crates fill from their label: Refresh label brings new records, N passes a track";

/// How long a verdict stays on the title line.
const FLASH_SECS: Duration = Duration::from_millis(1500);

/// A verdict the title line flashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Wanted,
    Unwanted,
    Pass,
    Owned,
}

/// "WANTED", or "WANTED 3" when it took several records.
fn flash_text(verdict: Verdict, count: usize) -> String {
    let word = match verdict {
        Verdict::Wanted => "WANTED",
        Verdict::Unwanted => "UNWANTED",
        Verdict::Pass => "PASS",
        Verdict::Owned => "OWNED",
    };
    if count > 1 {
        format!("{word} {count}")
    } else {
        word.into()
    }
}

/// The paste key's modifier on this platform, as the hints spell it.
const PASTE_MODIFIER: &str = if cfg!(target_os = "macos") {
    "CMD"
} else {
    "CTRL"
};

/// The list menu's Add files…, with the shortcut that does the same.
const ADD_FILES: &str = if cfg!(target_os = "macos") {
    "Add files… (Cmd+O)"
} else {
    "Add files… (Ctrl+O)"
};

/// A line of the empty crate's hint: text, or links side by side.
#[derive(Debug, Clone, PartialEq)]
enum HintLine {
    Text(String),
    Links(Vec<HintLink>),
    /// A sentence with links inside it, its parts a space apart.
    Inline(Vec<HintPart>),
}

/// A part of an inline hint line.
#[derive(Debug, Clone, PartialEq)]
enum HintPart {
    Text(&'static str),
    Link(HintLink),
}

/// What a hint's link does.
#[derive(Debug, Clone, Copy, PartialEq)]
enum HintTo {
    /// Opens a page in the browser.
    Url(&'static str),
    /// The extension: the setup panel, or (without it) the pairing dialog.
    Extension,
    /// The Discogs account: the setup panel.
    Discogs,
    /// The shown seller crate's double-click: count and dig it (or connect Discogs first).
    Seller,
    /// Add files…, to the shown crate.
    AddFiles,
}

/// A store name in its own colour (made readable on the list), opening its home page; or the
/// extension's pairing dialog; or a seller crate's dig.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HintLink {
    label: &'static str,
    color: [u8; 3],
    to: HintTo,
}

/// The stores' names in their brand colours: pasted links (Discogs, Bandcamp), and through
/// the extension (Juno, Bleep, Beatport, Traxsource).
const PASTE_STORES: [HintLink; 2] = [
    HintLink {
        label: "DISCOGS",
        color: [51, 51, 51],
        to: HintTo::Url("https://www.discogs.com"),
    },
    HintLink {
        label: "BANDCAMP",
        color: [29, 160, 195],
        to: HintTo::Url("https://bandcamp.com"),
    },
];
const EXTENSION_STORES: [HintLink; 4] = [
    HintLink {
        label: "JUNO",
        color: [255, 102, 0],
        to: HintTo::Url("https://www.juno.co.uk"),
    },
    HintLink {
        label: "BLEEP",
        color: [230, 30, 80],
        to: HintTo::Url("https://bleep.com"),
    },
    HintLink {
        label: "BEATPORT",
        color: [1, 255, 149],
        to: HintTo::Url("https://www.beatport.com"),
    },
    HintLink {
        label: "TRAXSOURCE",
        color: [58, 157, 211],
        to: HintTo::Url("https://www.traxsource.com"),
    },
];

/// A crate's name in the sidebar: under TOP SELLERS, LABELS and FRIENDS, without the
/// "Seller: ", "Label: " or "Friend: " its name starts with (the heading says it).
fn side_name(c: &crate::crates::CrateInfo, grouped: bool) -> &str {
    if !(grouped || c.is_label()) {
        return &c.name;
    }
    ["Seller: ", "Label: ", "Friend: "]
        .iter()
        .find_map(|p| c.name.strip_prefix(p))
        .unwrap_or(&c.name)
}

/// The empty crate's hint where the list is tall enough: where to dig, by paste or through
/// the browser extension.
/// Which setup links an empty crate shows: the steps still open (see `welcome-setup`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct SetupLinks {
    discogs: bool,
    extension: bool,
}

fn store_hint(modifier: &str, links: SetupLinks) -> Vec<HintLine> {
    let mut lines = vec![
        HintLine::Text(format!("PASTE A LINK HERE · {modifier}+V")),
        HintLine::Links(PASTE_STORES.to_vec()),
        HintLine::Text(String::new()),
        HintLine::Text("WITH THE BROWSER EXTENSION".into()),
        HintLine::Links(EXTENSION_STORES.to_vec()),
    ];
    let link = |label, to| {
        HintLine::Links(vec![HintLink {
            label,
            color: [47, 228, 90],
            to,
        }])
    };
    if links.extension {
        lines.push(link("SET UP THE EXTENSION", HintTo::Extension));
    }
    if links.discogs {
        lines.push(HintLine::Text(String::new()));
        lines.push(link("CONNECT YOUR DISCOGS", HintTo::Discogs));
    }
    lines.push(HintLine::Text(String::new()));
    lines.push(link("ADD FILES FROM YOUR COMPUTER", HintTo::AddFiles));
    lines.push(HintLine::Text("OR DROP FILES · PRESS H FOR HELP".into()));
    lines
}

/// WCAG contrast ratio of two colours.
fn contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
    let lum = |c: [u8; 3]| {
        let ch = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.039_28 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
    };
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `c`, moved toward white or black (away from `bg`) until it reads at 3:1 on `bg`.
fn readable(c: [u8; 3], bg: [u8; 3]) -> [u8; 3] {
    let away = if contrast([255; 3], bg) >= contrast([0; 3], bg) {
        [255, 255, 255]
    } else {
        [0, 0, 0]
    };
    (0..=10)
        .map(|i| lerp_color(c, away, i as f32 / 10.0).to_array())
        .map(|[r, g, b, _]| [r, g, b])
        .find(|&m| contrast(m, bg) >= 3.0)
        .unwrap_or(away)
}

/// What an empty crate shows in its list when it is too short for the stores: how to fill
/// it, and where the help is. An empty line is a gap.
fn empty_crate_hint(modifier: &str) -> [String; 4] {
    [
        "PASTE A DISCOGS OR BANDCAMP LINK".into(),
        format!("{modifier}+V · OR DROP FILES"),
        String::new(),
        "PRESS H FOR HELP".into(),
    ]
}

/// "tracks coming…" is shown, then faint, for this long each (seconds).
const BLINK: f64 = 0.5;

/// The BPM slider is this wide, whatever the playlist's width (skin pixels).
const BPM_SLIDER_W: f32 = 40.0;

/// The BPM control's "BPM" label (with its gap) and the rest (slider, gap, widest range).
fn bpm_widths(sk: &Skinned) -> (f32, f32) {
    (
        sk.text_width("BPM") + 4.0,
        BPM_SLIDER_W + 3.0 + sk.text_width("000-000"),
    )
}

/// The narrowest the filter bar's search field gets (skin pixels).
const SEARCH_MIN_W: f32 = 60.0;
/// The widest it gets, as a share of the bar: a maximized playlist doesn't need more.
const SEARCH_MAX_SHARE: f32 = 0.5;
/// A crate's values for a filter, each with its number of records, the most first.
type FacetCounts = Rc<Vec<(String, usize)>>;

/// How the footer shows the style, artist and label filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FooterFilters {
    /// Style chips, then ARTISTS and LABELS.
    Chips,
    /// STYLES, ARTISTS and LABELS.
    Buttons,
    /// One FILTERS button, opening the panel.
    Folded,
    /// Not even that fits.
    None,
}

/// The filter panel, open from a filter's button in the bar.
fn facet_popup_id(f: Facet) -> Id {
    Id::new(("facet_list", f.name()))
}

/// A filter's button in the bar: "ARTISTS", or "ARTISTS 2" while two are picked.
fn facet_button_label(shown: &crate::playlist::Playlist, f: Facet) -> String {
    let name = match f {
        Facet::Style => "STYLES",
        Facet::Artist => "ARTISTS",
        Facet::Label => "LABELS",
        Facet::Year => "YEARS",
        Facet::Format => "FORMATS",
    };
    match shown.filter(f).map(|p| p.len()) {
        Some(n) => format!("{name} {n}"),
        None => name.to_owned(),
    }
}

/// The filter panel, open from the FILTERS button.
fn filters_popup_id() -> Id {
    Id::new("filters_panel")
}
/// Past this many styles, the footer shows the STYLES button and its list instead of chips.
const STYLE_CHIPS_MAX: usize = 20;
/// Skin pixels between two style chips (and before the first).
const STYLE_GAP: f32 = 5.0;

/// The crate sidebar's width, in skin pixels.
const SIDEBAR_W: u16 = 110;
/// The playlist width (skin pixels) from which the crate sidebar fits, unless maximized.
const SIDEBAR_FROM_WIDTH: u16 = 600;

/// A playlist row's widget id, by its index in the crate.
fn row_id(idx: usize) -> Id {
    Id::new(("pl_row", idx))
}

/// The id of a playlist row's context menu (egui's default for a response's popup).
fn row_menu_id(idx: usize) -> Id {
    row_id(idx).with("popup")
}

/// Text drawn left-centred at `at` as `Painter::text` draws it, with the chars a search word
/// covers (see [`crate::playlist::search_marks`]) in `hl`. Returns where it went.
fn text_marked(
    p: &egui::Painter,
    at: Pos2,
    text: String,
    font: &egui::FontId,
    col: Color32,
    hl: Color32,
    words: &[Vec<char>],
) -> Rect {
    let marks = crate::playlist::search_marks(&text, words);
    if !marks.contains(&true) {
        return p.text(at, egui::Align2::LEFT_CENTER, text, font.clone(), col);
    }
    let mut job = egui::text::LayoutJob::default();
    let mut run = String::new();
    let mut lit = marks[0];
    let mut push = |run: &str, lit: bool| {
        let c = if lit { hl } else { col };
        job.append(run, 0.0, egui::TextFormat::simple(font.clone(), c));
    };
    for (c, &m) in text.chars().zip(&marks) {
        if m != lit {
            push(&run, lit);
            run.clear();
            lit = m;
        }
        run.push(c);
    }
    push(&run, lit);
    let galley = p.layout_job(job);
    let rect = egui::Align2::LEFT_CENTER.anchor_size(at, galley.size());
    p.galley(rect.min, galley, col);
    rect
}

/// Where an entry without a BPM stands in the background analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
enum BpmWait {
    /// The share of its file analyzed so far.
    Analyzing(f32),
    Queued,
}

impl BpmWait {
    /// For the entry's tooltip.
    fn label(self) -> String {
        match self {
            Self::Analyzing(share) => format!("analyzing {}%", (share * 100.0).round() as u32),
            Self::Queued => "BPM queued".into(),
        }
    }
}

/// The BPM analysis bar's width, in skin pixels.
const BPM_BAR_W: f32 = 16.0;

/// An entry's place in the BPM analysis (`at` is the bar's left centre): an empty frame while
/// queued, filling up to the share analyzed.
fn draw_bpm_wait(
    p: &egui::Painter,
    at: Pos2,
    wait: BpmWait,
    scale: f32,
    col: Color32,
    dim: Color32,
) {
    let bar = Rect::from_min_size(at - vec2(0.0, 3.0 * scale), vec2(BPM_BAR_W, 6.0) * scale);
    let share = match wait {
        BpmWait::Analyzing(share) => share.clamp(0.0, 1.0),
        BpmWait::Queued => 0.0,
    };
    let done = vec2(bar.width() * share, bar.height());
    p.rect_filled(Rect::from_min_size(bar.min, done), 0.0, col);
    let frame = if share > 0.0 { col } else { dim };
    p.rect_stroke(
        bar,
        0.0,
        egui::Stroke::new(scale, frame),
        egui::StrokeKind::Inside,
    );
}

fn lerp_color(a: [u8; 3], b: [u8; 3], t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgb(l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2]))
}

impl eframe::App for DiggrApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.logic_inner(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ui_start = Instant::now();
        self.ui_inner(ui);
        if let Some(p) = &mut self.profile {
            p.ui_done(ui_start);
        }
    }

    fn on_exit(&mut self) {
        self.exit();
    }
}

impl DiggrApp {
    fn logic_inner(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        if let Some(p) = &mut self.profile {
            p.logic_start(now);
        }
        if self.auto_quit.is_some_and(|t| now >= t) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        let dt = (now - self.last_frame).as_secs_f32().min(0.25);
        self.last_frame = now;
        self.update_audio();
        self.show_playing_bpm();
        self.sync_wait();
        self.handle_keys(ctx);
        if self.apply_sync() {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.dig_update();
        self.follow_shown();
        // A grouped crate places what arrived (sends, digs, tags) with its record.
        for c in [self.crates.shown_id(), self.crates.playing_id()] {
            if self.crates.get_mut(c).is_some_and(|p| p.settle()) {
                self.mark_crate(c);
            }
        }

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

        // Scroll the title while playing (classic players scroll ~5 characters per second).
        if self.position.state == PlayState::Playing {
            self.title_tick += dt as f64;
            while self.title_tick > 0.2 {
                self.title_tick -= 0.2;
                self.title_offset = self.title_offset.wrapping_add(1);
            }
        }

        let hidden = ctx.input(|i| {
            i.viewport().minimized.unwrap_or(false) || i.viewport().occluded.unwrap_or(false)
        });
        let activity = Activity {
            playing: self.position.state == PlayState::Playing,
            waveform: true,
            engine_starting: matches!(self.engine, EngineSlot::Starting(_)),
            hidden,
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
        let origin = ui.max_rect().min;
        let scale = self.settings.scale as f32;
        self.win_px = ui.max_rect().size() / scale;
        // A click gives its side the keyboard (lit in this same frame).
        if let Some(p) = ctx.input(|i| {
            i.pointer
                .primary_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        }) {
            // The player is the header and the band on top, the playlist under them.
            let band = crate::layout::TOP_H as f32 * scale;
            self.focus = if p.y >= origin.y + band {
                Focus::Playlist
            } else {
                Focus::Player
            };
        }
        self.band_layout(ui, origin);
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
            .or_else(|| self.message.as_ref().map(|(text, _)| text.clone()))
            .or_else(|| self.dig_line());
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
        #[cfg(not(target_arch = "wasm32"))]
        self.dig_dialog_ui(&ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.dig_bridge_dialog_ui(&ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.welcome_ui(&ctx);
        if self.help {
            crate::help::show(&ctx, &mut self.help);
        }
        self.spectrogram_window(&ctx);
        self.eq_window(&ctx);
        self.insights_window(&ctx);
        if !self.first_frame_done {
            self.first_frame_done = true;
            let ms = self.startup.process_start.elapsed().as_secs_f64() * 1e3;
            if self.startup.report {
                println!("startup: first frame after {ms:.1} ms");
            }
            if self.startup.exit_after_first_frame {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            if let Some((spawner, store)) = self.cleanup.take() {
                let _ = spawner.spawn(
                    "cleanup",
                    platform::Priority::Low,
                    Box::new(move || store.remove_old_visuals()),
                );
            }
        }
    }

    /// The one layout: the window header on top, then the band of three panels, a third each
    /// (the mini player, the turntable, the waveform), and the playlist under them, filling the
    /// window.
    fn band_layout(&mut self, ui: &mut Ui, origin: Pos2) {
        let scale = self.settings.scale as f32;
        let win_w = self.win_px.x;
        self.header(ui, origin, win_w);
        let top = origin + vec2(0.0, crate::layout::HEADER_H as f32 * scale);
        let band_h = crate::layout::BAND_H as f32;
        let panels = crate::layout::band_panels(win_w);
        let at = |x: u16| top + vec2(x as f32 * scale, 0.0);
        // Anywhere on the band that is not a control drags a window that isn't maximized: this
        // area goes first, so the controls drawn after it win their own clicks.
        let band = Rect::from_min_size(top, vec2(win_w, band_h) * scale);
        let drag = ui.interact(band, Id::new("band_drag"), Sense::drag());
        if drag.drag_started() && !ui.input(|i| i.viewport().maximized.unwrap_or(false)) {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let wave = Rect::from_min_size(
            at(panels.wave.0),
            vec2(panels.wave.1 as f32, band_h) * scale,
        );
        self.waveform_section(ui, wave);
        self.deck_section(ui, at(panels.deck.0), panels.deck.1 as f32);
        self.mini_player(ui, top, panels.player.1 as f32);
        self.playlist_section(ui, origin + vec2(0.0, crate::layout::TOP_H as f32 * scale));
    }

    /// The window header, `w` skin pixels wide: DIGGR between groove lines, and at the right
    /// help, the Options gear (the same menu as a right-click on the player), minimize and
    /// close (which quits). It lights while the player has the keyboard. Dragging it moves a
    /// window that isn't maximized, and a double-click maximizes or restores it. Each button
    /// takes only clicks inside its own square, so a near-miss never quits.
    fn header(&mut self, ui: &mut Ui, origin: Pos2, w: f32) {
        let def = self.def.clone();
        let sk = self.skinned(&def, ui, origin);
        let h = crate::layout::HEADER_H as f32;
        sk.sprite("hdr_l", 0.0, 0.0);
        sk.sprite_in("hdr_fill", sk.rect(2.0, 0.0, (w - 4.0).max(0.0), h));
        sk.sprite("hdr_r", w - 2.0, 0.0);
        let caption = "DIGGR";
        let tw = sk.text_width(caption);
        let x = ((w - tw) / 2.0).round();
        sk.sprite_in("hdr_plain", sk.rect(x - 6.0, 0.0, tw + 12.0, h));
        let ty = ((h - def.font.glyph_h as f32) / 2.0).floor();
        sk.text(x, ty, caption, color(def.colors.pl_title));
        if self.focus != Focus::Player {
            sk.fill(sk.rect(0.0, 0.0, w, h), Color32::from_black_alpha(140));
        }
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        let drag = ui.interact(
            sk.rect(0.0, 0.0, w, h),
            Id::new("header_drag"),
            Sense::click_and_drag(),
        );
        if drag.drag_started() && !maximized {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        }
        // Right to left: close, minimize, the gear, help. Without anything in the Options
        // menu the gear's slot stays empty, so the others keep their places.
        let hb = crate::skin::generate::HB as f32;
        let slot = |n: f32| w - HB_MARGIN - hb - n * (hb + HB_GAP);
        let rect = |n: f32| sk.rect(slot(n), HB_Y, hb, hb);
        if corner_button(ui, &sk, "hdr_close", rect(0.0), "hb_close").1 {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
        if corner_button(ui, &sk, "hdr_min", rect(1.0), "hb_min").1 {
            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
        }
        if self.has_options() {
            let (opts, clicked) = corner_button(ui, &sk, "hdr_opts", rect(2.0), "hb_opts");
            let mut actions = Vec::new();
            egui::Popup::menu(&opts)
                .open_memory(clicked.then_some(egui::SetOpenCommand::Toggle))
                .show(|ui| self.options_items(ui, &mut actions));
            opts.on_hover_text("Options");
            for a in actions {
                self.apply(a, ui.ctx());
            }
        }
        let (help, clicked) = corner_button(ui, &sk, "hdr_help", rect(3.0), "hb_help");
        help.on_hover_text("Shortcuts (H)");
        if clicked {
            self.help = !self.help;
        }
    }

    /// The player, the band's left panel `w` skin pixels wide: play state, time and two lines
    /// of title on an LCD, with ▤, EQ and spectrum at its end; previous, play, pause, stop,
    /// next, volume, shuffle and repeat under it. A right-click off the controls opens the
    /// Options menu.
    fn mini_player(&mut self, ui: &mut Ui, origin: Pos2, w: f32) {
        let mut def = (*self.def).clone();
        let h = crate::layout::BAND_H as f32;
        // The volume slider's place here: `hslider` reads its rectangle from the layout.
        def.layout.insert(
            "mini_volume".into(),
            crate::skin::R {
                x: VOLUME_X as u16,
                y: BUTTON_ROW_Y as u16,
                w: (shuffle_x(w) - 4.0 - VOLUME_X) as u16,
                ..def.at("volume")
            },
        );
        let sk = self.skinned(&def, ui, origin);
        sk.sprite_in("deck_fill", sk.rect(0.0, 0.0, w, h));
        sk.sprite_in("deck_edge_r", sk.rect(w - 1.0, 0.0, 1.0, h));
        let mut actions = Vec::new();
        self.options_menu(ui, sk.rect(0.0, 0.0, w, h), "mini_options", &mut actions);
        let colors = &def.colors;
        let lcd = color(colors.lcd);
        sk.fill(sk.rect(4.0, LCD_Y, w - 9.0, LCD_H), color(colors.pl_bg));
        let state = self.position.state;
        sk.sprite(
            match state {
                PlayState::Playing => "status_play",
                PlayState::Paused => "status_pause",
                PlayState::Stopped => "status_stop",
            },
            6.0,
            DIGITS_Y + 2.0,
        );
        // The time, elapsed or (a click away) remaining.
        let duration = self.now_playing.as_ref().and_then(|i| i.duration_secs);
        let elapsed = self.position.seconds();
        let remaining = self.settings.time_remaining && duration.is_some();
        let secs = if remaining {
            (duration.unwrap_or(0.0) - elapsed).max(0.0)
        } else {
            elapsed
        };
        let show = state != PlayState::Stopped;
        if show && remaining {
            sk.sprite("digit_minus", 16.0, DIGITS_Y);
        }
        let (mm, ss) = format::lcd(secs);
        let digit = |c: char| {
            if show {
                format!("digit_{c}")
            } else {
                "digit_blank".to_owned()
            }
        };
        let mins: Vec<char> = mm.chars().collect();
        for (i, c) in mins.iter().rev().take(2).enumerate() {
            sk.sprite(&digit(*c), 36.0 - 10.0 * i as f32, DIGITS_Y);
        }
        if show {
            sk.sprite("digit_colon", 46.0, DIGITS_Y);
        }
        for (i, c) in ss.chars().enumerate() {
            sk.sprite(&digit(c), 50.0 + 10.0 * i as f32, DIGITS_Y);
        }
        if ui
            .interact(
                sk.rect(16.0, DIGITS_Y, 53.0, 13.0),
                Id::new("mini_time"),
                Sense::click(),
            )
            .clicked()
        {
            actions.push(Action::ToggleRemaining);
        }
        // The title line, or a verdict's flash centred on it.
        let icons_x = lcd_icons_x(w);
        let (tx, tw) = (74.0, icons_x - 3.0 - 74.0);
        let ty = LINE1_Y;
        let left = self
            .flash
            .as_ref()
            .and_then(|(_, at)| FLASH_SECS.checked_sub(at.elapsed()))
            .filter(|d| !d.is_zero());
        if let (Some(left), Some((text, _))) = (left, &self.flash) {
            let x = tx + ((tw - sk.text_width(text)) / 2.0).round();
            sk.text(x, ty, text, lcd);
            ui.ctx().request_repaint_after(left);
        } else {
            self.flash = None;
            let title = self
                .now_playing_line()
                .unwrap_or_else(|| engine_status(&self.engine));
            let fits = (tw / def.font.advance as f32) as usize;
            sk.text(
                tx,
                ty,
                &format::scroll(&title, fits, self.title_offset),
                lcd,
            );
        }
        // The second line: the record's label, side, year and what's for sale, or the
        // file's album (the verdict flash leaves it alone).
        if let Some(detail) = self.now_playing_detail() {
            let fits = (tw / def.font.advance as f32) as usize;
            sk.text(
                tx,
                LINE2_Y,
                &format::scroll(&detail, fits, self.title_offset),
                lerp_color(colors.lcd, [0, 0, 0], 0.35),
            );
        }
        // ▤, EQ and spectrum: the band's tools, at the LCD's right end.
        for (i, (id, icon, on, tip, action)) in [
            (
                "mini_group",
                "tog_group",
                self.crates.shown().is_grouped(),
                "Group by record",
                Action::ToggleGrouped,
            ),
            (
                "mini_eq",
                "tog_eq",
                self.settings.show_eq,
                "Equalizer",
                Action::ToggleEq,
            ),
            (
                "mini_spec",
                "tog_spec",
                self.spectro_open,
                "Spectrogram (S)",
                Action::ToggleSpectrogram,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let r = sk.rect(
                icons_x + (ICON_W + ICON_GAP) * i as f32,
                DIGITS_Y,
                ICON_W,
                12.0,
            );
            if icon_toggle(ui, &sk, id, r, icon, on)
                .on_hover_text(tip)
                .clicked()
            {
                actions.push(action);
            }
        }
        // The main transport's buttons, the one of the play state lit.
        let lit = match state {
            PlayState::Playing => "play",
            PlayState::Paused => "pause",
            PlayState::Stopped => "stop",
        };
        for (i, (name, action)) in [
            ("prev", Action::Prev),
            ("play", Action::Play),
            ("pause", Action::Pause),
            ("stop", Action::Stop),
            ("next", Action::Next),
        ]
        .into_iter()
        .enumerate()
        {
            let r = sk.rect(4.0 + 23.0 * i as f32, TRANSPORT_Y, 23.0, 18.0);
            let sprite = if name == lit {
                format!("{name}_on")
            } else {
                name.to_owned()
            };
            if sprite_button(ui, &sk, &format!("mini_{name}"), r, &sprite).clicked() {
                actions.push(action);
            }
        }
        let vol = SliderSprites {
            track: "volume_track",
            fill: Some("volume_fill"),
            thumb: "volume_thumb",
        };
        if let (_, Some(v)) = widgets::hslider(
            ui,
            &sk,
            "mini_vol",
            "mini_volume",
            vol,
            self.settings.volume,
        ) {
            actions.push(Action::Volume(v));
        }
        // The button row ends where the LCD row's icons do: shuffle and repeat at the right,
        // the volume slider filling the room between them and the transport.
        let r = sk.rect(shuffle_x(w), BUTTON_ROW_Y, ICON_W, 12.0);
        if icon_toggle(
            ui,
            &sk,
            "mini_shuffle",
            r,
            "tog_shuf",
            self.settings.shuffle,
        )
        .on_hover_text("Shuffle")
        .clicked()
        {
            actions.push(Action::Shuffle);
        }
        let (icon, on, tip) = match self.settings.repeat {
            Repeat::Off => ("tog_rep", false, "Repeat: off"),
            Repeat::All => ("tog_rep", true, "Repeat: all"),
            Repeat::One => ("tog_rep1", true, "Repeat: one"),
        };
        let r = sk.rect(shuffle_x(w) + ICON_W + ICON_GAP, BUTTON_ROW_Y, ICON_W, 12.0);
        if icon_toggle(ui, &sk, "mini_repeat", r, icon, on)
            .on_hover_text(tip)
            .clicked()
        {
            actions.push(Action::Repeat);
        }
        self.seek_bar(ui, &sk, w, &mut actions);
        for a in actions {
            self.apply(a, ui.ctx());
        }
    }

    /// The EQ window while it is open: the skin's EQ section in a window of its own, above the
    /// app's. Where it was put is remembered.
    fn eq_window(&mut self, ctx: &egui::Context) {
        if !self.settings.show_eq {
            return;
        }
        let (w, h) = self.def.eq_size;
        let mut builder = egui::ViewportBuilder::default()
            .with_title("Diggr EQ")
            .with_inner_size(vec2(w as f32, h as f32) * self.settings.scale as f32)
            .with_decorations(false)
            .with_resizable(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop);
        if let Some((x, y)) = self.settings.eq_window_pos {
            builder = builder.with_position([x, y]);
        }
        // Without windows of its own (the web, tests), it floats over the playlist's top left.
        if ctx.embed_viewports() {
            let band = crate::layout::TOP_H as f32;
            let at = pos2(0.0, band * self.settings.scale as f32);
            egui::Area::new(Id::new("eq_area"))
                .order(egui::Order::Foreground)
                .fixed_pos(at)
                .show(ctx, |ui| {
                    ui.allocate_space(vec2(w as f32, h as f32) * self.settings.scale as f32);
                    self.eq_section(ui, at);
                });
            return;
        }
        ctx.show_viewport_immediate(eq_viewport_id(), builder, |ui, _| {
            if ui.input(|i| i.viewport().close_requested()) {
                self.apply(Action::ToggleEq, ui.ctx());
                return;
            }
            let origin = ui.max_rect().min;
            self.eq_section(ui, origin);
            let pos = ui
                .input(|i| i.viewport().outer_rect)
                .map(|r| (r.min.x, r.min.y));
            if pos.is_some()
                && self.settings.eq_window_pos != pos
                && ui.input(|i| !i.pointer.any_down())
            {
                self.settings.eq_window_pos = pos;
                self.mark_settings();
            }
        });
    }

    /// The Insights window while it is open: the collection's numbers, recomputed when the
    /// crate changed, and what its clicks ask.
    fn insights_window(&mut self, ctx: &egui::Context) {
        use crate::insights::{self, Action as Ia, LabelInfo};
        if !self.insights_open {
            self.insights_opened = None;
            return;
        }
        let Some(c) = self
            .crates
            .list()
            .iter()
            .find(|c| c.collection)
            .map(|c| c.id)
        else {
            self.insights_open = false;
            return;
        };
        if !self.crates.load(c) {
            return;
        }
        let rev = self.crates.get(c).map_or(0, |p| p.rev());
        if self
            .insights
            .as_ref()
            .is_none_or(|(id, r, _)| (*id, *r) != (c, rev))
            && let Some(p) = self.crates.get(c)
        {
            self.insights = Some((c, rev, Rc::new(insights::Stats::of(p))));
            #[cfg(not(target_arch = "wasm32"))]
            self.insights_ask_totals();
        }
        let Some((_, _, stats)) = self.insights.clone() else {
            return;
        };
        // The bars grow in over GROW_SECS, easing out.
        const GROW_SECS: f64 = 0.7;
        let now = ctx.input(|i| i.time);
        let x = ((now - *self.insights_opened.get_or_insert(now)) / GROW_SECS).clamp(0.0, 1.0);
        if x < 1.0 {
            ctx.request_repaint();
        }
        let colors = &self.skin.def.colors;
        let look = insights::Look {
            bg: color(colors.pl_bg),
            text: color(colors.pl_text),
            bars: color(colors.lcd),
            title: color(colors.pl_title),
            grow: 1.0 - (1.0 - x as f32).powi(3),
        };
        let label = |name: &str| -> LabelInfo {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(d) = self.dig.as_ref().filter(|d| d.has_token()) {
                return match d.label_totals.get(name) {
                    Some(&(id, total)) => LabelInfo::Total {
                        total,
                        following: self.crates.find_label(id).is_some(),
                    },
                    None => LabelInfo::Asking,
                };
            }
            let _ = name;
            LabelInfo::Unknown
        };
        let builder = egui::ViewportBuilder::default()
            .with_title("Insights")
            .with_inner_size([780.0, 640.0])
            .with_min_inner_size([380.0, 300.0]);
        let actions = ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("insights"),
            builder,
            |ui, _| {
                let mut a = insights::ui(ui, &stats, &label, &look);
                if ui.input(|i| i.viewport().close_requested()) {
                    a.push(Ia::Close);
                }
                a
            },
        );
        for a in actions {
            match a {
                Ia::Close => self.insights_open = false,
                Ia::Filter(picks) => {
                    self.show_crate(c);
                    let p = self.crates.shown_mut();
                    p.clear_picks(None);
                    for (f, v) in &picks {
                        p.set_pick(*f, v, true);
                    }
                    self.filter_changed();
                }
                Ia::ShowRecord(id) => {
                    self.show_crate(c);
                    let p = self.crates.shown_mut();
                    p.clear_picks(None);
                    p.select_only(&[id], id);
                    self.filter_changed();
                    if let Some(i) = self.crates.shown().index_of(id) {
                        self.scroll_into_view(i);
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                Ia::Follow(name) => {
                    let id = self
                        .dig
                        .as_ref()
                        .and_then(|d| d.label_totals.get(&name))
                        .map(|t| t.0);
                    if let Some(id) = id {
                        let page = ::dig::discogs::url::Page::new(
                            ::dig::discogs::url::PageKind::Label(id),
                        );
                        self.dig_follow_label(page, id);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                Ia::Follow(_) => {}
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
            lossless: info.is_some_and(|i| i.lossless),
        };
        if self.spectro.feed(feed) {
            ctx.request_repaint_of(crate::spectrogram::viewport_id());
        }
        self.spectro.show(ctx);
    }

    fn exit(&mut self) {
        self.settings_dirty.get_or_insert_with(Instant::now);
        self.save_now(true);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(e) = self.dig.as_mut().and_then(|d| d.save_memory()) {
            eprintln!("{e}");
        }
    }
}

/// Presentation settings for the app's window: vsync with up to 3 frames queued. With eframe's
/// default of 1 (measured on an M2 MacBook, even for a blank window) frames regularly
/// miss their vsync slot and the rate swings between 30 and 60 fps; with 3 it holds 60. The
/// windowed player only draws on input or at 30 fps, so its queue stays short and clicks stay
/// snappy. (Set at startup: eframe 0.36's `Frame::set_wgpu_surface_config` changes a clone of
/// the render state and has no effect.)
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

/// Where each frame's time goes: our `logic`, our `ui` (the egui layer), and everything eframe does outside them (egui tessellation, GPU submit, waiting
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
        let path = std::env::temp_dir().join("diggr_frame_stats.log");
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

    fn ui_done(&mut self, start: Instant) {
        let now = Instant::now();
        self.ui.add((now - start).as_secs_f64() * 1e3);
        self.ui_end = Some(now);
        self.frames += 1;
        if self.window.elapsed() >= Duration::from_secs(1) {
            let line = format!(
                "fps {:3} | frame ms avg/max {} | logic {} | ui {} | eframe paint+present {}",
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
    fn a_verdict_flashes_its_word_and_count() {
        assert_eq!(flash_text(Verdict::Wanted, 1), "WANTED");
        assert_eq!(flash_text(Verdict::Wanted, 3), "WANTED 3");
        assert_eq!(flash_text(Verdict::Unwanted, 1), "UNWANTED");
        assert_eq!(flash_text(Verdict::Pass, 1), "PASS");
        assert_eq!(flash_text(Verdict::Owned, 2), "OWNED 2");
        // Every flash fits the mini player's title line, between the time and its icons.
        let def = LoadedSkin::default_skin().def;
        let w = flash_text(Verdict::Unwanted, 999).chars().count() as f32 * def.font.advance as f32;
        let icons_x = lcd_icons_x(def.main_size.0 as f32);
        assert!(w <= icons_x - 3.0 - 74.0);
    }

    #[test]
    fn an_empty_crate_says_how_to_fill_it() {
        assert_eq!(
            empty_crate_hint("CMD"),
            [
                "PASTE A DISCOGS OR BANDCAMP LINK",
                "CMD+V · OR DROP FILES",
                "",
                "PRESS H FOR HELP"
            ]
        );
        assert_eq!(empty_crate_hint("CTRL")[1], "CTRL+V · OR DROP FILES");
        // Every character is in the skin font, and the longer line fits the narrowest list.
        let def = LoadedSkin::default_skin().def;
        for line in empty_crate_hint("CTRL")
            .into_iter()
            .filter(|l| !l.is_empty())
        {
            assert!(line.chars().all(|c| def.glyph(c).is_some()), "{line}");
            let w = line.chars().count() as u16 * def.font.advance - 1;
            assert!(w <= def.at("pl_list").w, "{line}: {w}");
        }
    }

    #[test]
    fn the_store_hint_fits_and_every_store_reads_on_the_list() {
        let def = LoadedSkin::default_skin().def;
        let all = SetupLinks {
            discogs: true,
            extension: true,
        };
        for line in store_hint("CTRL", all) {
            let text = match line {
                HintLine::Text(t) => t,
                HintLine::Links(links) => {
                    links.iter().map(|k| k.label).collect::<Vec<_>>().join("  ")
                }
                HintLine::Inline(parts) => parts
                    .iter()
                    .map(|p| match p {
                        HintPart::Text(t) => *t,
                        HintPart::Link(k) => k.label,
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            assert!(text.chars().all(|c| def.glyph(c).is_some()), "{text}");
            let w = (text.chars().count() as u16 * def.font.advance).saturating_sub(1);
            assert!(w <= def.at("pl_list").w, "{text}: {w}");
        }
        for k in PASTE_STORES.iter().chain(&EXTENSION_STORES) {
            let c = readable(k.color, def.colors.pl_bg);
            assert!(contrast(c, def.colors.pl_bg) >= 3.0, "{}", k.label);
        }
        // Discogs' near-black is lightened on the black list; Beatport's green is kept.
        assert_ne!(readable([51, 51, 51], [0, 0, 0]), [51, 51, 51]);
        assert_eq!(readable([1, 255, 149], [0, 0, 0]), [1, 255, 149]);
    }

    #[test]
    fn slow_scrolling_adds_up_to_whole_rows() {
        let mut acc = 0.0;
        let mut first = 10;
        // 3 points a frame for 10 frames, downwards, with 13-point rows: 2 rows.
        for _ in 0..10 {
            first = scroll_rows(&mut acc, first, 100, -3.0, 13.0);
        }
        assert_eq!(first, 12);
        // Leftovers pushing past an end are dropped; scrolling away from it keeps them.
        let mut acc = 12.0;
        assert_eq!(scroll_rows(&mut acc, 0, 100, 0.5, 13.0), 0);
        assert_eq!(acc, 0.0);
        let mut acc = 0.0;
        assert_eq!(scroll_rows(&mut acc, 0, 100, -5.0, 13.0), 0);
        assert_eq!(acc, -5.0);
        let mut acc = 0.0;
        assert_eq!(
            scroll_rows(&mut acc, 5, 100, 40.0, 13.0),
            2,
            "a wheel step moves at once"
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
}

#[cfg(test)]
mod headless_tests;
