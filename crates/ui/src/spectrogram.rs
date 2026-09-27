//! The spectrogram window (`S`): a separate, resizable window with the current track's
//! spectrogram on a log-frequency axis.
//!
//! - **Track mode** draws the whole-track spectral overview built by the overview pass; zooming
//!   past its resolution asks the detail worker for the visible range at screen resolution and
//!   draws that on top as it arrives.
//! - **Live mode** is a scrolling waterfall of the audible sound, computed from the tap on the UI
//!   thread (a 4096-point FFT every 10 ms) and drawn up to the audible position.
//!
//! The window is a deferred egui viewport, so it repaints on its own schedule: only while
//! something moves (live playback, the playhead, detail arriving) or on input. Its state lives
//! behind a mutex shared with the player, which feeds it each frame and takes back its actions.
//! Levels are coloured on the CPU through a 256-entry lookup table; changing the dB range
//! re-colours the textures (at most ~2 M texels).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use analysis::detail::{Detail, DetailRequest, DetailService};
use analysis::overview::Overview;
use analysis::spectral::{Channel, MIN_HZ, ROWS, RowMap, Stft, byte_to_db, power_to_byte};
use audio::tap::TapChunk;
use egui::epaint::{Mesh, Vertex};
use egui::{
    Align2, Color32, ColorImage, FontId, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions,
    Ui, ViewportBuilder, ViewportId, pos2, vec2,
};
use platform::TrackRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Mode {
    /// The whole track (and zoomed detail), with the playhead.
    #[default]
    Track,
    /// A scrolling waterfall of what is audible.
    Live,
}

/// What the window remembers between sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpectroSettings {
    pub mode: Mode,
    pub channel: Channel,
    /// The colour map spans this dB range.
    pub db_floor: f32,
    pub db_ceiling: f32,
    /// Window size in points.
    pub size: (f32, f32),
}

impl Default for SpectroSettings {
    fn default() -> Self {
        Self {
            mode: Mode::Track,
            channel: Channel::Mid,
            db_floor: -120.0,
            db_ceiling: 0.0,
            size: (900.0, 480.0),
        }
    }
}

impl SpectroSettings {
    pub fn sanitized(mut self) -> Self {
        self.db_ceiling = self.db_ceiling.clamp(-100.0, 0.0);
        self.db_floor = self.db_floor.clamp(-120.0, self.db_ceiling - 10.0);
        self.size = (
            self.size.0.clamp(MIN_SIZE.0, 4000.0),
            self.size.1.clamp(MIN_SIZE.1, 3000.0),
        );
        self
    }
}

const MIN_SIZE: (f32, f32) = (420.0, 260.0);
/// Seconds of history shown in live mode.
const LIVE_SPAN: f64 = 8.0;
/// Live columns kept (10 ms each: ~10 s).
const LIVE_COLUMNS: usize = 1024;
const LIVE_FFT: usize = 4096;
/// Largest detail request, in columns and rows.
const MAX_DETAIL: (usize, usize) = (2048, 1024);
/// The view must rest this long before detail is requested (no work while dragging).
const DETAIL_SETTLE: Duration = Duration::from_millis(120);
const AXIS_W: f32 = 46.0;
const AXIS_H: f32 = 16.0;

pub fn viewport_id() -> ViewportId {
    ViewportId::from_hash_of("spectrogram")
}

/// A perceptual colour map (matplotlib's inferno, polynomial fit), `t` in 0..=1.
pub fn inferno(t: f32) -> Color32 {
    const C: [[f32; 3]; 7] = [
        [0.000_218_94, 0.001_651, -0.019_480_9],
        [0.106513, 0.563956, 3.93271],
        [11.6025, -3.97285, -15.9424],
        [-41.704, 17.4364, 44.3541],
        [77.1629, -33.4024, -81.8073],
        [-71.3194, 32.6261, 73.2095],
        [25.1311, -12.2427, -23.0703],
    ];
    let t = t.clamp(0.0, 1.0);
    let ch = |i: usize| {
        let v = C.iter().rev().fold(0.0, |acc, c| acc * t + c[i]);
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Color32::from_rgb(ch(0), ch(1), ch(2))
}

/// Colour for each level byte, spreading `floor..ceiling` dB over the colour map.
pub fn colour_lut(floor: f32, ceiling: f32) -> [Color32; 256] {
    std::array::from_fn(|b| {
        let db = byte_to_db(b as u8);
        inferno((db - floor) / (ceiling - floor).max(1.0))
    })
}

/// The nearest note, e.g. "A4" for 440 Hz.
pub fn note_name(hz: f32) -> Option<String> {
    const NAMES: [&str; 12] = [
        "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
    ];
    if !(16.0..=30_000.0).contains(&hz) {
        return None;
    }
    let midi = (69.0 + 12.0 * (hz / 440.0).log2()).round() as i32;
    Some(format!(
        "{}{}",
        NAMES[midi.rem_euclid(12) as usize],
        midi.div_euclid(12) - 1
    ))
}

/// `m:ss.mmm`.
pub fn clock_ms(secs: f64) -> String {
    let ms = (secs.max(0.0) * 1000.0).round() as u64;
    format!("{}:{:02}.{:03}", ms / 60_000, ms / 1000 % 60, ms % 1000)
}

fn hz_label(hz: f32) -> String {
    if hz >= 1000.0 {
        let k = hz / 1000.0;
        if (k - k.round()).abs() < 0.05 {
            format!("{}k", k.round())
        } else {
            format!("{k:.1}k")
        }
    } else {
        format!("{}", hz.round())
    }
}

/// The cursor readout: time, frequency with note name, and level.
pub fn readout(t: f64, hz: f32, db: Option<f32>) -> String {
    let note = note_name(hz).map(|n| format!(" {n}")).unwrap_or_default();
    let level = db.map(|d| format!(" · {d:.1} dB")).unwrap_or_default();
    format!("{} · {:.0} Hz{note}{level}", clock_ms(t), hz)
}

/// The visible time and frequency ranges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub t0: f64,
    pub t1: f64,
    pub f_lo: f32,
    pub f_hi: f32,
}

impl View {
    pub fn whole(duration: f64, nyquist: f32) -> Self {
        Self {
            t0: 0.0,
            t1: duration.max(0.1),
            f_lo: MIN_HZ,
            f_hi: nyquist,
        }
    }

    pub fn time_at(&self, x: f32, rect: Rect) -> f64 {
        let u = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
        self.t0 + u * (self.t1 - self.t0)
    }

    pub fn x_of(&self, t: f64, rect: Rect) -> f32 {
        rect.left() + ((t - self.t0) / (self.t1 - self.t0)) as f32 * rect.width()
    }

    /// Log frequency, low at the bottom.
    pub fn hz_at(&self, y: f32, rect: Rect) -> f32 {
        let v = ((rect.bottom() - y) / rect.height()).clamp(0.0, 1.0);
        self.f_lo * (self.f_hi / self.f_lo).powf(v)
    }

    pub fn y_of(&self, hz: f32, rect: Rect) -> f32 {
        let v = (hz / self.f_lo).ln() / (self.f_hi / self.f_lo).ln();
        rect.bottom() - v * rect.height()
    }

    /// Zooms time by `factor` (< 1 zooms in) keeping `around` in place, within the track.
    pub fn zoom_time(&mut self, factor: f64, around: f64, duration: f64) {
        let span = ((self.t1 - self.t0) * factor).clamp(0.05, duration.max(0.1));
        let u = ((around - self.t0) / (self.t1 - self.t0)).clamp(0.0, 1.0);
        self.t0 = around - u * span;
        self.t1 = self.t0 + span;
        self.clamp_time(duration);
    }

    /// Zooms frequency (log scale) by `factor`, keeping `around` in place.
    pub fn zoom_freq(&mut self, factor: f32, around: f32, nyquist: f32) {
        let (lo, hi, c) = (self.f_lo.ln(), self.f_hi.ln(), around.ln());
        let full = (nyquist / MIN_HZ).ln();
        // At least half an octave visible.
        let span = ((hi - lo) * factor).clamp(0.5 * std::f32::consts::LN_2, full);
        let u = ((c - lo) / (hi - lo)).clamp(0.0, 1.0);
        self.f_lo = (c - u * span).exp();
        self.f_hi = self.f_lo * span.exp();
        self.clamp_freq(nyquist);
    }

    /// Moves by `dt` seconds and `dlog` natural-log units of frequency.
    pub fn pan(&mut self, dt: f64, dlog: f32, duration: f64, nyquist: f32) {
        self.t0 += dt;
        self.t1 += dt;
        self.clamp_time(duration);
        self.f_lo *= dlog.exp();
        self.f_hi *= dlog.exp();
        self.clamp_freq(nyquist);
    }

    fn clamp_time(&mut self, duration: f64) {
        let span = self.t1 - self.t0;
        self.t0 = self.t0.clamp(0.0, (duration - span).max(0.0));
        self.t1 = self.t0 + span;
    }

    fn clamp_freq(&mut self, nyquist: f32) {
        let ratio = (self.f_hi / self.f_lo).min(nyquist / MIN_HZ);
        self.f_lo = self.f_lo.clamp(MIN_HZ, nyquist / ratio);
        self.f_hi = self.f_lo * ratio;
    }
}

/// A texture's coverage: time `t0..t1` along its height, log frequency `lo..hi` along its width.
#[derive(Debug, Clone, Copy)]
struct Coverage {
    t0: f64,
    t1: f64,
    lo: f32,
    hi: f32,
}

impl Coverage {
    fn u(&self, hz: f32) -> f32 {
        (hz / self.lo).ln() / (self.hi / self.lo).ln()
    }

    fn v(&self, t: f64) -> f32 {
        ((t - self.t0) / (self.t1 - self.t0)) as f32
    }
}

/// Draws the part of `tex` that falls inside the view.
fn paint_tex(painter: &egui::Painter, rect: Rect, view: &View, tex: &TextureHandle, c: Coverage) {
    let (t0, t1) = (c.t0.max(view.t0), c.t1.min(view.t1));
    let (lo, hi) = (c.lo.max(view.f_lo), c.hi.min(view.f_hi));
    if t1 <= t0 || hi <= lo {
        return;
    }
    let (x0, x1) = (view.x_of(t0, rect), view.x_of(t1, rect));
    let (y_top, y_bot) = (view.y_of(hi, rect), view.y_of(lo, rect));
    let mut mesh = Mesh::with_texture(tex.id());
    // Texture x is frequency and y is time, so the quad's UVs are transposed.
    let corners = [
        (pos2(x0, y_top), c.u(hi), c.v(t0)),
        (pos2(x1, y_top), c.u(hi), c.v(t1)),
        (pos2(x1, y_bot), c.u(lo), c.v(t1)),
        (pos2(x0, y_bot), c.u(lo), c.v(t0)),
    ];
    for (pos, u, v) in corners {
        mesh.vertices.push(Vertex {
            pos,
            uv: pos2(u, v),
            color: Color32::WHITE,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// Live waterfall: FFT columns from the tap, stamped with the track frame at their centre.
pub struct Live {
    rate: u32,
    hop: u64,
    stft: Stft,
    rows: RowMap,
    hist: Vec<[f32; 2]>,
    pos: usize,
    since: u64,
    track: audio::TrackId,
    next_frame: u64,
    /// Ring of `LIVE_COLUMNS` columns of `ROWS` level bytes, and each column's centre frame.
    cols: Vec<u8>,
    col_frame: Vec<u64>,
    head: usize,
    len: usize,
    /// Columns written since the texture was last updated.
    fresh: usize,
    power_rows: Vec<f32>,
}

impl Live {
    pub fn new(rate: u32) -> Self {
        let rate = rate.max(8000);
        Self {
            rate,
            hop: (rate / 100) as u64,
            stft: Stft::new(LIVE_FFT),
            rows: RowMap::new(LIVE_FFT, rate, ROWS, MIN_HZ, rate as f32 / 2.0),
            hist: vec![[0.0; 2]; LIVE_FFT],
            pos: 0,
            since: 0,
            track: u64::MAX,
            next_frame: 0,
            cols: vec![0; LIVE_COLUMNS * ROWS],
            col_frame: vec![0; LIVE_COLUMNS],
            head: 0,
            len: 0,
            fresh: 0,
            power_rows: vec![0.0; ROWS],
        }
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn clear(&mut self) {
        self.hist.fill([0.0; 2]);
        self.len = 0;
        self.since = 0;
        self.fresh = LIVE_COLUMNS;
    }

    /// Adds a tap chunk. A new track or a jump in frames starts a fresh history.
    pub fn push(&mut self, chunk: &TapChunk, channel: Channel) {
        if chunk.track != self.track || chunk.start_frame != self.next_frame {
            self.clear();
            self.track = chunk.track;
        }
        self.next_frame = chunk.start_frame + chunk.frames as u64;
        for (i, f) in chunk.data().as_chunks::<2>().0.iter().enumerate() {
            self.hist[self.pos] = *f;
            self.pos = (self.pos + 1) % LIVE_FFT;
            self.since += 1;
            if self.since == self.hop {
                self.since = 0;
                let end = chunk.start_frame + i as u64 + 1;
                self.column(end.saturating_sub(LIVE_FFT as u64 / 2), channel);
            }
        }
    }

    fn column(&mut self, centre: u64, channel: Channel) {
        let (a, b) = self.hist.split_at(self.pos);
        let power = self
            .stft
            .power(b.iter().chain(a).map(|f| channel.of(f[0], f[1])));
        self.rows.map(power, &mut self.power_rows);
        let slot = &mut self.cols[self.head * ROWS..(self.head + 1) * ROWS];
        for (o, &p) in slot.iter_mut().zip(&self.power_rows) {
            *o = power_to_byte(p);
        }
        self.col_frame[self.head] = centre;
        self.head = (self.head + 1) % LIVE_COLUMNS;
        self.len = (self.len + 1).min(LIVE_COLUMNS);
        self.fresh += 1;
    }

    /// Columns in time order as (ring slot, centre time in seconds).
    pub fn columns(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        let first = (self.head + LIVE_COLUMNS - self.len) % LIVE_COLUMNS;
        (0..self.len).map(move |k| {
            let slot = (first + k) % LIVE_COLUMNS;
            (slot, self.col_frame[slot] as f64 / self.rate as f64)
        })
    }

    pub fn column_bytes(&self, slot: usize) -> &[u8] {
        &self.cols[slot * ROWS..(slot + 1) * ROWS]
    }

    fn hop_secs(&self) -> f64 {
        self.hop as f64 / self.rate as f64
    }
}

/// What the player tells the window each frame.
#[derive(Clone, Default)]
pub struct Feed {
    pub title: String,
    pub track: Option<TrackRef>,
    pub overview: Option<Arc<Overview>>,
    /// Audible position.
    pub now: f64,
    pub duration: Option<f64>,
    pub playing: bool,
    pub fullscreen: bool,
    pub lossless: bool,
}

/// What the window asks the player to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Seek(f64),
    Close,
    /// Mode, channel, range or size changed: persist them.
    Settings(SpectroSettings),
}

/// When to repaint without input (`None`: only on input).
#[derive(Debug, Clone, Copy, Default)]
pub struct Activity {
    pub playing: bool,
    pub live: bool,
    pub detail_arriving: bool,
    pub fullscreen: bool,
}

pub fn repaint_after(a: Activity) -> Option<Duration> {
    if a.fullscreen {
        None
    } else if a.playing && a.live {
        Some(Duration::from_millis(16))
    } else if a.detail_arriving {
        Some(Duration::from_millis(40))
    } else if a.playing {
        // The playhead moves a pixel every few hundred ms on a whole-track view.
        Some(Duration::from_millis(100))
    } else {
        None
    }
}

#[derive(Default)]
struct Tex {
    handle: Option<TextureHandle>,
    /// What the texture shows (pointer, columns, channel, colour generation).
    key: (usize, usize, u8, u64),
}

impl Tex {
    /// Uploads `rows × columns` level bytes (column-major) through `lut` when `key` changed.
    fn update(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        key: (usize, usize, u8, u64),
        rows: usize,
        data: &[u8],
        lut: &[Color32; 256],
    ) -> Option<&TextureHandle> {
        let columns = data.len() / rows.max(1);
        if columns == 0 {
            return None;
        }
        if self.handle.is_none() || self.key != key {
            let pixels = data[..columns * rows]
                .iter()
                .map(|&b| lut[b as usize])
                .collect();
            let img = ColorImage::new([rows, columns], pixels);
            match &mut self.handle {
                Some(h) if h.size() == [rows, columns] => h.set(img, TextureOptions::LINEAR),
                _ => self.handle = Some(ctx.load_texture(name, img, TextureOptions::LINEAR)),
            }
            self.key = key;
        }
        self.handle.as_ref()
    }
}

struct Inner {
    feed: Feed,
    settings: SpectroSettings,
    view: Option<View>,
    /// Track the view belongs to.
    view_track: Option<TrackRef>,
    view_changed: Instant,
    /// The user hasn't zoomed or panned: show the whole track (it follows the duration).
    whole: bool,
    /// Plot width in physical pixels, for deciding when detail is needed.
    plot_px: f32,
    live: Live,
    live_tex: Option<TextureHandle>,
    /// Colour generation of the live texture.
    live_lut_gen: u64,
    overview_tex: Tex,
    detail_tex: Tex,
    lut: [Color32; 256],
    lut_gen: u64,
    detail: Option<Arc<DetailService>>,
    actions: Vec<Action>,
}

/// The window and its state, shared between the player and the window's viewport.
#[derive(Clone)]
pub struct SpectrogramWindow {
    inner: Arc<Mutex<Inner>>,
}

impl SpectrogramWindow {
    pub fn new(settings: SpectroSettings, detail: Option<Arc<DetailService>>) -> Self {
        let lut = colour_lut(settings.db_floor, settings.db_ceiling);
        Self {
            inner: Arc::new(Mutex::new(Inner {
                feed: Feed::default(),
                settings,
                view: None,
                view_track: None,
                view_changed: Instant::now(),
                whole: true,
                plot_px: 800.0,
                live: Live::new(48_000),
                live_tex: None,
                live_lut_gen: 0,
                overview_tex: Tex::default(),
                detail_tex: Tex::default(),
                lut,
                lut_gen: 0,
                detail,
                actions: Vec::new(),
            })),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Feeds tap chunks at `rate` (only kept in live mode).
    pub fn push_tap(&self, chunks: &[TapChunk], rate: u32) {
        let mut s = self.lock();
        if s.settings.mode != Mode::Live || chunks.is_empty() {
            return;
        }
        if s.live.rate() != rate && rate > 0 {
            s.live = Live::new(rate);
        }
        let channel = s.settings.channel;
        for c in chunks {
            s.live.push(c, channel);
        }
    }

    pub fn live_mode(&self) -> bool {
        self.lock().settings.mode == Mode::Live
    }

    /// Updates what the window shows. Returns whether the window should be woken (something
    /// visible changed while it would otherwise sleep).
    pub fn feed(&self, feed: Feed) -> bool {
        let mut s = self.lock();
        let wake = !feed.fullscreen
            && (s.feed.track != feed.track
                || (s.feed.now - feed.now).abs() > 0.01
                || s.feed.playing != feed.playing
                || s.feed.overview.as_ref().map(Arc::as_ptr)
                    != feed.overview.as_ref().map(Arc::as_ptr));
        s.feed = feed;
        wake
    }

    pub fn take_actions(&self) -> Vec<Action> {
        std::mem::take(&mut self.lock().actions)
    }

    /// Declares the window's viewport for this frame (call every frame while it is open).
    pub fn show(&self, ctx: &egui::Context) {
        let (title, size) = {
            let s = self.lock();
            let t = if s.feed.title.is_empty() {
                "Spectrogram".to_owned()
            } else {
                format!("Spectrogram — {}", s.feed.title)
            };
            (t, s.settings.size)
        };
        let builder = ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([size.0, size.1])
            .with_min_inner_size([MIN_SIZE.0, MIN_SIZE.1]);
        let me = self.clone();
        ctx.show_viewport_deferred(viewport_id(), builder, move |ui, _class| {
            me.ui(ui);
        });
    }

    /// The window's contents (also usable directly, e.g. in tests).
    pub fn ui(&self, ui: &mut Ui) {
        let mut s = self.lock();
        let before = s.actions.len();
        let ctx = ui.ctx().clone();
        egui::Frame::NONE
            .fill(Color32::from_rgb(10, 10, 14))
            .inner_margin(6.0)
            .show(ui, |ui| s.draw(ui));
        let (close, size) = ctx.input(|i| {
            (
                i.viewport().close_requested(),
                i.viewport().inner_rect.map(|r| r.size()),
            )
        });
        if close {
            s.actions.push(Action::Close);
        }
        if let Some(size) = size {
            let size = (size.x.round(), size.y.round());
            if size != s.settings.size && size.0 >= MIN_SIZE.0 && size.1 >= MIN_SIZE.1 {
                s.settings.size = size;
                let st = s.settings.clone();
                s.actions.push(Action::Settings(st));
            }
        }
        let activity = Activity {
            playing: s.feed.playing,
            live: s.settings.mode == Mode::Live,
            detail_arriving: s.detail_arriving(),
            fullscreen: s.feed.fullscreen,
        };
        if let Some(after) = repaint_after(activity) {
            ctx.request_repaint_after(after);
        }
        if s.actions.len() > before {
            ctx.request_repaint_of(ViewportId::ROOT);
        }
    }
}

impl Inner {
    fn nyquist(&self) -> f32 {
        match self.settings.mode {
            Mode::Live => self.live.rate() as f32 / 2.0,
            Mode::Track => self
                .feed
                .overview
                .as_ref()
                .map_or(22_050.0, |o| o.sample_rate as f32 / 2.0),
        }
    }

    fn duration(&self) -> f64 {
        self.feed
            .duration
            .or_else(|| self.feed.overview.as_ref().map(|o| o.seconds()))
            .unwrap_or(0.0)
    }

    fn detail_arriving(&self) -> bool {
        self.settings.mode == Mode::Track
            && self
                .detail
                .as_ref()
                .and_then(|d| d.get())
                .is_some_and(|d| !d.complete())
    }

    fn settings_changed(&mut self) {
        self.lut = colour_lut(self.settings.db_floor, self.settings.db_ceiling);
        self.lut_gen += 1;
        let st = self.settings.clone();
        self.actions.push(Action::Settings(st));
    }

    fn draw(&mut self, ui: &mut Ui) {
        self.controls(ui);
        let avail = ui.available_rect_before_wrap();
        let plot = Rect::from_min_max(
            avail.min + vec2(AXIS_W, 0.0),
            avail.max - vec2(0.0, AXIS_H + 18.0),
        );
        if plot.width() < 20.0 || plot.height() < 20.0 {
            return;
        }
        let response = ui.allocate_rect(plot, Sense::click_and_drag());
        let painter = ui.painter_at(avail);
        painter.rect_filled(plot, 0.0, Color32::BLACK);
        let nyquist = self.nyquist();
        let duration = self.duration();

        // The view: whole track by default, reset when the track changes.
        let view = match self.settings.mode {
            Mode::Live => {
                let f = self.view.unwrap_or(View::whole(1.0, nyquist));
                View {
                    t0: self.feed.now - LIVE_SPAN,
                    t1: self.feed.now,
                    ..f
                }
            }
            Mode::Track => {
                if self.view_track != self.feed.track {
                    self.view_track = self.feed.track.clone();
                    self.whole = true;
                }
                match self.view {
                    Some(v) if !self.whole => v,
                    // The frequency zoom survives; time follows the whole track.
                    Some(v) => View {
                        t0: 0.0,
                        t1: duration.max(0.1),
                        ..v
                    },
                    None => View::whole(duration, nyquist),
                }
            }
        };
        self.plot_px = plot.width() * ui.ctx().pixels_per_point();
        let mut view = view;
        let before = view;
        if self.interact(ui, &response, plot, &mut view, duration, nyquist) {
            self.view_changed = Instant::now();
            if self.settings.mode == Mode::Track && (view.t0 != before.t0 || view.t1 != before.t1) {
                self.whole = view.t0 <= 0.0 && view.t1 >= duration - 1e-6;
            }
        }
        if self.settings.mode == Mode::Track {
            self.view = Some(view);
        } else if let Some(v) = &mut self.view {
            (v.f_lo, v.f_hi) = (view.f_lo, view.f_hi);
        } else {
            self.view = Some(view);
        }

        let ctx = ui.ctx().clone();
        let painter_plot = ui.painter_at(plot);
        let mut level_at: Option<f32> = None;
        let hover = response.hover_pos();
        match self.settings.mode {
            Mode::Track => {
                self.paint_track(&ctx, &painter_plot, plot, &view, hover, &mut level_at);
                if self.feed.track.is_some() && duration > 0.0 {
                    let x = view.x_of(self.feed.now, plot);
                    if (plot.left()..=plot.right()).contains(&x) {
                        painter_plot.vline(x, plot.y_range(), Stroke::new(1.5, Color32::WHITE));
                    }
                }
            }
            Mode::Live => self.paint_live(&ctx, &painter_plot, plot, &view, hover, &mut level_at),
        }
        let has_time =
            self.feed.track.is_some() && (duration > 0.0 || self.settings.mode == Mode::Live);
        axes(&painter, plot, &view, has_time);

        // Bottom line: readout on the left, verdict on the right.
        let y = plot.bottom() + AXIS_H + 10.0;
        let text = match hover {
            Some(p) => readout(view.time_at(p.x, plot), view.hz_at(p.y, plot), level_at),
            None if self.feed.track.is_none() => "Nothing playing".into(),
            None => "Click: seek · scroll: zoom time · Shift+scroll: zoom frequency · drag: pan · double-click: whole track".into(),
        };
        painter.text(
            pos2(plot.left(), y),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(12.0),
            Color32::from_gray(210),
        );
        if let Some(v) = self.verdict() {
            painter.text(
                pos2(plot.right(), y),
                Align2::RIGHT_CENTER,
                v,
                FontId::proportional(12.0),
                Color32::from_rgb(255, 196, 90),
            );
        }
    }

    fn verdict(&self) -> Option<String> {
        let o = self.feed.overview.as_ref()?;
        if !o.complete {
            return Some("Checking where the content ends…".into());
        }
        let c = o.spectral.as_ref()?.cutoff.cutoff()?;
        Some(c.describe(self.feed.lossless))
    }

    fn controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let mut changed = false;
            for (m, label) in [(Mode::Track, "Track"), (Mode::Live, "Live")] {
                if ui
                    .selectable_label(self.settings.mode == m, label)
                    .clicked()
                    && self.settings.mode != m
                {
                    self.settings.mode = m;
                    if m == Mode::Live {
                        self.live.clear();
                    }
                    changed = true;
                }
            }
            ui.separator();
            let zoomed = self.detail_active();
            for (c, label) in [
                (Channel::Mid, "Mid"),
                (Channel::Side, "Side"),
                (Channel::Left, "L"),
                (Channel::Right, "R"),
            ] {
                // The overview stores mid and side; L and R need detail or live data.
                let enabled = matches!(c, Channel::Mid | Channel::Side)
                    || self.settings.mode == Mode::Live
                    || zoomed;
                let r = ui.add_enabled(
                    enabled,
                    egui::Button::selectable(self.settings.channel == c, label),
                );
                if r.clicked() && self.settings.channel != c {
                    self.settings.channel = c;
                    self.live.clear();
                    changed = true;
                }
            }
            ui.separator();
            ui.label("dB");
            let floor = ui.add(
                egui::DragValue::new(&mut self.settings.db_floor)
                    .range(-120.0..=(self.settings.db_ceiling - 10.0))
                    .speed(1.0),
            );
            ui.label("to");
            let ceil = ui.add(
                egui::DragValue::new(&mut self.settings.db_ceiling)
                    .range((self.settings.db_floor + 10.0)..=0.0)
                    .speed(1.0),
            );
            changed |= floor.changed() || ceil.changed();
            if let Some(d) = self.detail.as_ref().and_then(|d| d.get())
                && self.settings.mode == Mode::Track
                && zoomed
            {
                ui.label(
                    egui::RichText::new(format!("detail · {}-point FFT", d.fft))
                        .small()
                        .color(Color32::from_gray(150)),
                );
            }
            if changed {
                self.settings_changed();
            }
        });
    }

    /// Zoomed past the overview's resolution, so detail is shown.
    fn detail_active(&self) -> bool {
        self.settings.mode == Mode::Track
            && self
                .view
                .as_ref()
                .zip(self.feed.overview.as_ref())
                .is_some_and(|(v, o)| {
                    o.spectral.as_ref().is_some_and(|s| {
                        // Fewer overview columns in view than pixels.
                        ((v.t1 - v.t0) / s.column_secs()) < self.plot_px as f64
                    })
                })
    }

    /// Mouse input on the plot. Returns whether the view changed.
    fn interact(
        &mut self,
        ui: &Ui,
        r: &egui::Response,
        plot: Rect,
        view: &mut View,
        duration: f64,
        nyquist: f32,
    ) -> bool {
        let before = *view;
        let track = self.settings.mode == Mode::Track;
        if r.double_clicked() && track {
            *view = View::whole(duration, nyquist);
            self.whole = true;
        } else if r.clicked()
            && track
            && self.feed.track.is_some()
            && let Some(p) = r.interact_pointer_pos()
        {
            self.actions.push(Action::Seek(view.time_at(p.x, plot)));
        }
        if r.dragged() {
            let d = r.drag_delta();
            let dt = if track {
                -(d.x / plot.width()) as f64 * (view.t1 - view.t0)
            } else {
                0.0
            };
            let dlog = d.y / plot.height() * (view.f_hi / view.f_lo).ln();
            view.pan(dt, dlog, duration, nyquist);
        }
        if let Some(p) = r.hover_pos() {
            let (scroll, shift) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.shift));
            // Shift+wheel arrives as horizontal scroll on some platforms.
            let amount = scroll.y + if shift { scroll.x } else { 0.0 };
            if amount != 0.0 {
                let factor = (-amount * 0.003).exp();
                if shift {
                    view.zoom_freq(factor, view.hz_at(p.y, plot), nyquist);
                } else if track {
                    view.zoom_time(factor as f64, view.time_at(p.x, plot), duration);
                }
            }
        }
        *view != before
    }

    fn paint_track(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        plot: Rect,
        view: &View,
        hover: Option<Pos2>,
        level: &mut Option<f32>,
    ) {
        let Some(spectral) = self.feed.overview.as_ref().and_then(|o| o.spectral.clone()) else {
            painter.text(
                plot.center(),
                Align2::CENTER_CENTER,
                if self.feed.track.is_some() {
                    "Building the overview…"
                } else {
                    "Play a track to see its spectrogram"
                },
                FontId::proportional(14.0),
                Color32::from_gray(160),
            );
            return;
        };
        // The overview stores mid and side only.
        let ov_channel = if self.settings.channel == Channel::Side {
            Channel::Side
        } else {
            Channel::Mid
        };
        let data = match ov_channel {
            Channel::Side => &spectral.side,
            _ => &spectral.mid,
        };
        let key = (
            Arc::as_ptr(&spectral) as usize,
            spectral.columns,
            ov_channel as u8,
            self.lut_gen,
        );
        let coverage = Coverage {
            t0: 0.0,
            t1: spectral.columns as f64 * spectral.column_secs(),
            lo: MIN_HZ,
            hi: spectral.nyquist(),
        };
        if let Some(tex) =
            self.overview_tex
                .update(ctx, "spectrogram-overview", key, ROWS, data, &self.lut)
        {
            paint_tex(painter, plot, view, tex, coverage);
        }
        if let Some(p) = hover {
            let (t, hz) = (view.time_at(p.x, plot), view.hz_at(p.y, plot));
            if let Some(col) = spectral
                .column_at(t)
                .and_then(|c| spectral.column(ov_channel, c))
            {
                let row = analysis::spectral::hz_to_row(hz, ROWS, spectral.nyquist()) as usize;
                *level = col.get(row.min(ROWS - 1)).map(|&b| byte_to_db(b));
            }
        }

        // Detail on top once zoomed past the overview's resolution.
        if !self.detail_active() {
            return;
        }
        let Some(svc) = self.detail.clone() else {
            return;
        };
        let ppp = ctx.pixels_per_point();
        if let Some(track) = self.feed.track.clone()
            && self.view_changed.elapsed() >= DETAIL_SETTLE
        {
            svc.request(DetailRequest {
                track,
                t0: view.t0,
                t1: view.t1,
                columns: ((plot.width() * ppp) as usize).clamp(16, MAX_DETAIL.0),
                rows: ((plot.height() * ppp) as usize).clamp(16, MAX_DETAIL.1),
                lo_hz: view.f_lo,
                hi_hz: view.f_hi,
                channel: self.settings.channel,
            });
        } else if self.view_changed.elapsed() < DETAIL_SETTLE {
            ctx.request_repaint_after(DETAIL_SETTLE);
        }
        let Some(d) = svc.get() else { return };
        if d.failed || d.columns_done == 0 {
            return;
        }
        let rows = d.request.rows;
        let key = (
            Arc::as_ptr(&d) as usize,
            d.columns_done,
            d.request.channel as u8,
            self.lut_gen,
        );
        let col_secs = (d.request.t1 - d.request.t0) / d.request.columns as f64;
        let cov = Coverage {
            t0: d.request.t0,
            t1: d.request.t0 + d.columns_done as f64 * col_secs,
            lo: d.lo_hz,
            hi: d.hi_hz,
        };
        if let Some(tex) =
            self.detail_tex
                .update(ctx, "spectrogram-detail", key, rows, &d.data, &self.lut)
        {
            paint_tex(painter, plot, view, tex, cov);
        }
        if let Some(p) = hover {
            *level = detail_level(&d, view.time_at(p.x, plot), view.hz_at(p.y, plot)).or(*level);
        }
    }

    fn paint_live(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        plot: Rect,
        view: &View,
        hover: Option<Pos2>,
        level: &mut Option<f32>,
    ) {
        if self.live.len == 0 {
            painter.text(
                plot.center(),
                Align2::CENTER_CENTER,
                "Live: play something",
                FontId::proportional(14.0),
                Color32::from_gray(160),
            );
            return;
        }
        // Upload new columns into the ring texture (whole texture after a colour change).
        let live = &mut self.live;
        let stale = self
            .live_tex
            .as_ref()
            .is_none_or(|t| t.size() != [ROWS, LIVE_COLUMNS])
            || live.fresh >= LIVE_COLUMNS
            || self.live_lut_gen != self.lut_gen;
        if stale {
            let pixels = live.cols.iter().map(|&b| self.lut[b as usize]).collect();
            let img = ColorImage::new([ROWS, LIVE_COLUMNS], pixels);
            match &mut self.live_tex {
                Some(t) if t.size() == [ROWS, LIVE_COLUMNS] => t.set(img, TextureOptions::LINEAR),
                _ => {
                    self.live_tex =
                        Some(ctx.load_texture("spectrogram-live", img, TextureOptions::LINEAR))
                }
            }
            self.live_lut_gen = self.lut_gen;
            live.fresh = 0;
        } else if live.fresh > 0 {
            let tex = self.live_tex.as_mut().expect("uploaded above");
            let mut slot = (live.head + LIVE_COLUMNS - live.fresh) % LIVE_COLUMNS;
            let mut left = live.fresh;
            while left > 0 {
                let n = left.min(LIVE_COLUMNS - slot);
                let pixels = live.cols[slot * ROWS..(slot + n) * ROWS]
                    .iter()
                    .map(|&b| self.lut[b as usize])
                    .collect();
                tex.set_partial(
                    [0, slot],
                    ColorImage::new([ROWS, n], pixels),
                    TextureOptions::LINEAR,
                );
                slot = (slot + n) % LIVE_COLUMNS;
                left -= n;
            }
            live.fresh = 0;
        }
        let tex = self.live_tex.as_ref().expect("uploaded above");
        // Draw contiguous runs of ring slots; columns after the audible position stay hidden.
        let half = live.hop_secs() / 2.0;
        let cols: Vec<(usize, f64)> = live.columns().collect();
        let mut i = 0;
        while i < cols.len() {
            let mut j = i;
            while j + 1 < cols.len() && cols[j + 1].0 == cols[j].0 + 1 {
                j += 1;
            }
            let (first, last) = (cols[i], cols[j]);
            let (t0, t1) = (first.1 - half, last.1 + half);
            let cov = Coverage {
                t0: t0 - first.0 as f64 * live.hop_secs(),
                t1: t0 + (LIVE_COLUMNS - first.0) as f64 * live.hop_secs(),
                lo: MIN_HZ,
                hi: live.rate() as f32 / 2.0,
            };
            let visible = View {
                t0: view.t0.max(t0),
                t1: view.t1.min(t1),
                ..*view
            };
            if visible.t1 > visible.t0 {
                // Clip to this run, mapped through the full view.
                let clip = Rect::from_x_y_ranges(
                    view.x_of(visible.t0, plot)..=view.x_of(visible.t1, plot),
                    plot.y_range(),
                );
                paint_tex(&painter.with_clip_rect(clip), plot, view, tex, cov);
            }
            i = j + 1;
        }
        if let Some(p) = hover {
            let (t, hz) = (view.time_at(p.x, plot), view.hz_at(p.y, plot));
            if let Some(&(slot, _)) = cols
                .iter()
                .min_by(|a, b| (a.1 - t).abs().total_cmp(&(b.1 - t).abs()))
            {
                let row =
                    analysis::spectral::hz_to_row(hz, ROWS, live.rate() as f32 / 2.0) as usize;
                *level = Some(byte_to_db(live.column_bytes(slot)[row.min(ROWS - 1)]));
            }
        }
    }
}

fn detail_level(d: &Detail, t: f64, hz: f32) -> Option<f32> {
    let col_secs = (d.request.t1 - d.request.t0) / d.request.columns as f64;
    let c = ((t - d.request.t0) / col_secs).floor();
    if c < 0.0 || hz < d.lo_hz || hz > d.hi_hz {
        return None;
    }
    let rows = d.request.rows;
    let r = ((hz / d.lo_hz).ln() / (d.hi_hz / d.lo_hz).ln() * rows as f32) as usize;
    d.column(c as usize)
        .map(|col| byte_to_db(col[r.min(rows - 1)]))
}

/// Frequency ticks on the left, time ticks below.
fn axes(painter: &egui::Painter, plot: Rect, view: &View, time: bool) {
    let grey = Color32::from_gray(170);
    let font = FontId::proportional(11.0);
    for hz in [
        20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10_000.0, 20_000.0,
    ] {
        if hz < view.f_lo || hz > view.f_hi {
            continue;
        }
        let y = view.y_of(hz, plot);
        painter.hline((plot.left() - 4.0)..=plot.left(), y, Stroke::new(1.0, grey));
        painter.text(
            pos2(plot.left() - 6.0, y),
            Align2::RIGHT_CENTER,
            hz_label(hz),
            font.clone(),
            grey,
        );
    }
    if !time {
        return;
    }
    let span = view.t1 - view.t0;
    let step = [
        0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0,
        600.0, 1200.0,
    ]
    .into_iter()
    .find(|s| span / s * 80.0 <= plot.width() as f64)
    .unwrap_or(1800.0);
    let mut t = (view.t0 / step).ceil() * step;
    while t <= view.t1 {
        let x = view.x_of(t, plot);
        if x > plot.right() - 24.0 {
            break; // the label would be cut off
        }
        painter.vline(
            x,
            plot.bottom()..=(plot.bottom() + 4.0),
            Stroke::new(1.0, grey),
        );
        let label = if step < 1.0 {
            clock_ms(t)
        } else {
            crate::format::clock(t)
        };
        painter.text(
            pos2(x, plot.bottom() + 5.0),
            Align2::CENTER_TOP,
            label,
            font.clone(),
            grey,
        );
        t += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysis::overview::OverviewBuilder;

    const SR: u32 = 44_100;

    #[test]
    fn note_names() {
        assert_eq!(note_name(440.0).as_deref(), Some("A4"));
        assert_eq!(note_name(261.63).as_deref(), Some("C4"));
        assert_eq!(note_name(27.5).as_deref(), Some("A0"));
        assert_eq!(note_name(466.16).as_deref(), Some("A♯4"));
        assert_eq!(note_name(5.0), None);
    }

    #[test]
    fn readout_shows_time_frequency_note_and_level() {
        assert_eq!(
            readout(62.345, 440.2, Some(-12.34)),
            "1:02.345 · 440 Hz A4 · -12.3 dB"
        );
    }

    #[test]
    fn cursor_maps_to_frequency_and_back() {
        let rect = Rect::from_min_size(pos2(50.0, 20.0), vec2(800.0, 400.0));
        let v = View::whole(180.0, 22_050.0);
        let y = v.y_of(440.0, rect);
        assert!((v.hz_at(y, rect) - 440.0).abs() < 0.5);
        assert!((v.hz_at(rect.bottom(), rect) - MIN_HZ).abs() < 0.01);
        assert!((v.hz_at(rect.top(), rect) - 22_050.0).abs() < 1.0);
        assert_eq!(v.time_at(rect.left() + 400.0, rect), 90.0);
    }

    #[test]
    fn zoom_keeps_the_cursor_in_place_and_stays_in_range() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 500.0));
        let mut v = View::whole(200.0, 22_050.0);
        v.zoom_time(0.1, 50.0, 200.0);
        assert!((v.t1 - v.t0 - 20.0).abs() < 1e-9);
        assert!((v.time_at(v.x_of(50.0, rect), rect) - 50.0).abs() < 1e-6);
        v.zoom_time(100.0, 50.0, 200.0);
        assert_eq!((v.t0, v.t1), (0.0, 200.0), "zooming out stops at the track");
        v.zoom_freq(0.01, 1000.0, 22_050.0);
        assert!(
            (v.f_hi / v.f_lo - std::f32::consts::SQRT_2).abs() < 0.01,
            "half an octave at most"
        );
        v.pan(-1000.0, 100.0, 200.0, 22_050.0);
        assert_eq!(v.t0, 0.0);
        assert!(v.f_hi <= 22_050.0 + 0.1);
    }

    #[test]
    fn colour_map_runs_dark_to_bright() {
        let lut = colour_lut(-120.0, 0.0);
        let lum = |c: Color32| c.r() as u32 + c.g() as u32 + c.b() as u32;
        assert!(lum(lut[0]) < 30, "{:?}", lut[0]);
        assert!(lum(lut[255]) > 600, "{:?}", lut[255]);
        assert!(lum(lut[200]) > lum(lut[100]));
        let narrow = colour_lut(-60.0, 0.0);
        assert!(lum(narrow[100]) < 30, "below the floor is black");
    }

    #[test]
    fn repaints_only_when_something_moves() {
        let a = |playing, live, detail_arriving, fullscreen| Activity {
            playing,
            live,
            detail_arriving,
            fullscreen,
        };
        assert_eq!(
            repaint_after(a(false, false, false, false)),
            None,
            "paused track mode"
        );
        assert_eq!(
            repaint_after(a(false, true, false, false)),
            None,
            "paused live mode"
        );
        assert!(repaint_after(a(true, true, false, false)).unwrap() <= Duration::from_millis(16));
        assert!(repaint_after(a(false, false, true, false)).is_some());
        assert!(
            repaint_after(a(true, false, false, false)).is_some(),
            "playhead"
        );
        assert_eq!(repaint_after(a(true, true, true, true)), None, "fullscreen");
    }

    fn chunk(track: u64, start: u64, f: impl Fn(u64) -> f32) -> TapChunk {
        let mut c = TapChunk {
            track,
            start_frame: start,
            frames: audio::tap::TAP_CHUNK_FRAMES,
            samples: [0.0; audio::tap::TAP_CHUNK_FRAMES * 2],
        };
        for i in 0..c.frames {
            let x = f(start + i as u64);
            c.samples[2 * i] = x;
            c.samples[2 * i + 1] = x;
        }
        c
    }

    #[test]
    fn live_columns_line_up_with_the_audio() {
        // Silence with a loud burst at 1.0–1.02 s: the loudest column is centred there.
        let mut live = Live::new(SR);
        let burst = |f: u64| {
            if (SR as u64..SR as u64 + 882).contains(&f) {
                0.8 * ((f as f32) * 0.7).sin()
            } else {
                0.0
            }
        };
        let mut start = 0;
        while start < 2 * SR as u64 {
            live.push(&chunk(1, start, burst), Channel::Mid);
            start += audio::tap::TAP_CHUNK_FRAMES as u64;
        }
        let loud = |slot: usize| {
            live.column_bytes(slot)
                .iter()
                .map(|&b| b as u32)
                .sum::<u32>()
        };
        let (slot, t) = live.columns().max_by_key(|&(s, _)| loud(s)).unwrap();
        assert!(
            (t - 1.01).abs() < 0.03,
            "burst column at {t} s (slot {slot})"
        );
        // A jump in frames (seek) starts over.
        live.push(&chunk(1, 10 * SR as u64, |_| 0.0), Channel::Mid);
        assert!(live.columns().all(|(_, t)| t > 9.9));
    }

    fn overview(secs: f32) -> Arc<Overview> {
        let mut b = OverviewBuilder::new(SR);
        let x: Vec<f32> = (0..(secs * SR as f32) as usize)
            .flat_map(|i| {
                let s = 0.3 * (std::f32::consts::TAU * 440.0 * i as f32 / SR as f32).sin();
                [s, s]
            })
            .collect();
        b.push(&x);
        Arc::new(b.finish())
    }

    fn run(win: &SpectrogramWindow, ctx: &egui::Context, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(900.0, 480.0))),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| win.ui(ui));
        out.textures_delta.clear();
    }

    #[test]
    fn click_seeks_to_the_time_under_the_pointer() {
        let win = SpectrogramWindow::new(SpectroSettings::default(), None);
        win.feed(Feed {
            title: "t".into(),
            track: Some(TrackRef::new("/a.wav")),
            overview: Some(overview(10.0)),
            now: 1.0,
            duration: Some(180.0),
            playing: false,
            fullscreen: false,
            lossless: true,
        });
        let ctx = egui::Context::default();
        run(&win, &ctx, vec![]);
        // The plot spans x = 6 + AXIS_W .. 900 − 6 (the frame's margins).
        let (left, right) = (6.0 + AXIS_W, 894.0);
        let x = left + (right - left) * 0.5;
        let p = pos2(x, 200.0);
        let button = |pressed| egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        run(&win, &ctx, vec![egui::Event::PointerMoved(p), button(true)]);
        run(&win, &ctx, vec![button(false)]);
        let seeks: Vec<f64> = win
            .take_actions()
            .into_iter()
            .filter_map(|a| match a {
                Action::Seek(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(seeks.len(), 1, "one click, one seek");
        assert!((seeks[0] - 90.0).abs() < 0.5, "seek to {}", seeks[0]);
    }

    #[test]
    fn draws_every_mode_headless() {
        let win = SpectrogramWindow::new(SpectroSettings::default(), None);
        let ctx = egui::Context::default();
        run(&win, &ctx, vec![]); // nothing playing
        win.feed(Feed {
            track: Some(TrackRef::new("/a.wav")),
            overview: Some(overview(3.0)),
            duration: Some(3.0),
            playing: true,
            ..Default::default()
        });
        run(&win, &ctx, vec![]);
        win.lock().settings.mode = Mode::Live;
        let chunks: Vec<TapChunk> = (0..400)
            .map(|k| chunk(7, k * 256, |f| 0.2 * (f as f32 * 0.1).sin()))
            .collect();
        win.push_tap(&chunks, SR);
        win.feed(Feed {
            track: Some(TrackRef::new("/a.wav")),
            now: 400.0 * 256.0 / SR as f64,
            playing: true,
            ..Default::default()
        });
        run(&win, &ctx, vec![]);
        assert!(win.lock().live.len > 0);
    }
}
