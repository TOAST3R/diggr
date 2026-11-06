//! The player driven headlessly: a real engine on a `ManualSink`, crates in a temp config
//! folder, and egui frames fed with synthetic pointer and key events.

use super::*;
use crate::crates::PLAYLIST;
use crate::playlist::Origin;
use ::dig::browser::FakeBrowser;
use ::dig::clock::RealClock;
use ::dig::cover::{FakeImages, test_jpeg};
use ::dig::discogs::transport::FakeTransport;
use ::dig::preview::fetcher::{FakeFetcher, Fetcher};
use ::dig::preview::scheduler::Finder;
use egui::{Event, PointerButton};
use platform::CallbackInfo;
use platform::native::{NativeFileSource, NativeSpawner};
use platform::testing::ManualSink;
use std::path::Path;

const BUF: usize = 512;
/// Classic size, no SYNC. The playlist's top-left at 1×: under the band.
const PL_LEFT: f32 = 0.0;
const PL_TOP: f32 = crate::layout::TOP_H as f32;
/// Where the band starts: under the window header.
const BAND_TOP: f32 = crate::layout::HEADER_H as f32;
/// The rig's window by default: the classic playlist's width (no sidebar, no columns), and
/// tall enough for menus and dialogs.
const SCREEN: egui::Vec2 = vec2(275.0, 400.0);
/// The list's top below the playlist's: the title bar, then the filter bar.
const LIST_TOP: f32 = 36.0;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../audio/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn temp(name: &str) -> platform::testing::TestDir {
    platform::testing::TestDir::new(&format!("ui-app-{name}"))
}

struct Rig {
    app: DiggrApp,
    ctx: egui::Context,
    sink: ManualSink,
    now_ns: u64,
    dir: platform::testing::TestDir,
    /// Modifier keys held during the next frames.
    mods: Modifiers,
    /// The window, in points; the playlist fills it under the band.
    screen: egui::Vec2,
}

impl Rig {
    /// `prepare` may fill the config folder before the app starts.
    fn new(name: &str, open: Vec<PathBuf>, prepare: impl FnOnce(&Store)) -> Self {
        Self::with_dig(name, open, prepare, |_| None)
    }

    /// `dig` gets the rig's folder and gives the digging setup (fakes), if any.
    fn with_dig(
        name: &str,
        open: Vec<PathBuf>,
        prepare: impl FnOnce(&Store),
        dig: impl FnOnce(&Path) -> Option<DigSetup>,
    ) -> Self {
        let mut rig = Self::build(name, open, prepare, dig);
        rig.until(|r| r.app.engine().is_some(), "the engine starts");
        // Digging starts on the frame after the first.
        rig.frame(Vec::new());
        rig
    }

    /// The app before its first frame.
    fn build(
        name: &str,
        open: Vec<PathBuf>,
        prepare: impl FnOnce(&Store),
        dig: impl FnOnce(&Path) -> Option<DigSetup>,
    ) -> Self {
        dig::config::skip_flush_for_tests();
        let dir = temp(name);
        let store = Store::new(dir.join("config"));
        store
            .save(
                SETTINGS_FILE,
                &Settings {
                    scale: 1,
                    show_eq: false,
                    repeat: Repeat::One, // the 2 s fixtures keep playing
                    ..Default::default()
                },
            )
            .unwrap();
        prepare(&store);
        let sink = ManualSink::new(48_000, 2);
        let engine_sink = sink.clone();
        let ctx = egui::Context::default();
        let mut app = DiggrApp::build(
            ctx.clone(),
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
                startup: Startup {
                    process_start: Instant::now(),
                    report: false,
                    exit_after_first_frame: false,
                },
                analysis: None,
                overviews: None,
                dig: dig(&dir),
            },
        );
        // The tests were written for a crate that starts flat: the shown crate starts so
        // unless it was saved toggled or is a kind that was always grouped (Discogs, seller
        // and label crates). The grouping default has tests of its own.
        let shown = app.crates.shown_id();
        if app.crates.info(shown).is_some_and(|c| {
            c.grouped.is_none() && !c.discogs() && c.seller.is_none() && !c.is_label()
        }) {
            app.crates.set_grouped(shown, false);
        }
        // The tests read where things end up in the frame after an action; the motion tests
        // turn motion back on.
        app.settings.reduce_motion = true;
        Rig {
            app,
            ctx,
            sink,
            now_ns: 0,
            dir,
            mods: Modifiers::NONE,
            screen: SCREEN,
        }
    }

    fn frame(&mut self, mut events: Vec<Event>) -> egui::FullOutput {
        events.insert(0, Event::ModifiersChanged(self.mods));
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.screen)),
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
        // Generous: the whole workspace's tests share the machine, and a passing wait
        // ends as soon as `done` holds.
        let deadline = Instant::now() + Duration::from_secs(15);
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

    /// A click with modifiers held (Cmd, Shift).
    fn click_with_mods(&mut self, pos: Pos2, modifiers: Modifiers) -> egui::FullOutput {
        self.mods = modifiers;
        let out = self.click_with(pos, PointerButton::Primary);
        self.mods = Modifiers::NONE;
        out
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

    /// A key press, with `modifiers` held for that frame.
    fn key(&mut self, key: Key, modifiers: Modifiers) -> egui::FullOutput {
        self.mods = modifiers;
        let out = self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]);
        self.mods = Modifiers::NONE;
        out
    }

    /// A crate of `n` entries (the same fixture) shown in the playlist.
    fn fill_playlist(&mut self, n: usize) -> Vec<EntryId> {
        let t = TrackRef::new(fixture("tone.flac").to_string_lossy());
        let p = self.app.crates.shown_mut();
        p.add(std::iter::repeat_n(t, n));
        p.entries().iter().map(|e| e.id).collect()
    }

    fn title_bar(&self) -> Pos2 {
        pos2(PL_LEFT + 60.0, PL_TOP + 10.0)
    }

    /// A point on row `index`, right of its badges and cart pill.
    /// A window just tall enough for `rows` playlist rows under the band.
    fn set_rows(&mut self, rows: usize) {
        let d = &self.app.skin.def;
        let h = crate::layout::playlist_chrome(d) as usize + rows * d.pl_row_h as usize;
        self.screen.y = PL_TOP + h as f32;
    }

    fn row(&self, index: usize) -> Pos2 {
        pos2(
            PL_LEFT + 200.0,
            PL_TOP + LIST_TOP + index as f32 * 13.0 + 6.5,
        )
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

    fn dig(&self) -> &digging::Dig {
        self.app.dig.as_ref().expect("digging is on")
    }

    fn dig_mut(&mut self) -> &mut digging::Dig {
        self.app.dig.as_mut().expect("digging is on")
    }

    /// Crate `c`'s entries.
    fn entries(&self, c: CrateId) -> &[crate::playlist::Entry] {
        self.app.crates.get(c).expect("the crate").entries()
    }

    fn escape(&mut self) {
        self.key(Key::Escape, Modifiers::NONE);
    }

    /// Waits for "No preview found" and closes it, so that it doesn't take the next clicks.
    /// For records the fakes find nothing for.
    fn close_not_found(&mut self) {
        self.until(|r| !r.dig().not_found.notice.is_empty(), "No preview found");
        self.escape();
    }
}

/// Digging against fakes: Discogs answers from `transport`, pages, searches and previews come
/// from `fetcher`; no bridge, no welcome, and previews are cached in the rig's folder.
fn fake_dig(
    dir: &Path,
    transport: Arc<FakeTransport>,
    fetcher: &Arc<FakeFetcher>,
    browser: Arc<FakeBrowser>,
) -> DigSetup {
    let f = fetcher.clone();
    let finder: Finder = Arc::new(move |_| {
        let v = f.version()?;
        Some((f.clone() as Arc<dyn Fetcher>, v))
    });
    DigSetup {
        transport,
        images: FakeImages::new(Ok(test_jpeg(300, 300))),
        clock: Arc::new(RealClock::default()),
        finder,
        opener: ::dig::preview::scheduler::system_opener(),
        samples: fetcher.clone(),
        browser,
        cache_root: Some(dir.join("cache")),
        bridge: BridgeSetup::Off,
        tools: None,
        guided_ytdlp: false,
        welcome: false,
        consumer: None,
    }
}

/// A fake Discogs that answers from the dig crate's fixtures.
fn discogs_fixtures() -> Arc<FakeTransport> {
    Arc::new(FakeTransport::with_fixtures(format!(
        "{}/../dig/tests/fixtures/discogs",
        env!("CARGO_MANIFEST_DIR")
    )))
}

/// Saves a Discogs token before launch.
fn with_token(store: &Store) {
    ::dig::config::save_token(store.dir(), Some("tok")).unwrap();
}

/// The main window's message, or "".
fn message(rig: &Rig) -> String {
    rig.app
        .message
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default()
}

/// Where `label` is drawn.
fn at(rig: &mut Rig, label: &str) -> Pos2 {
    let out = rig.frame(Vec::new());
    texts(&out)
        .into_iter()
        .find(|t| t.text == label)
        .unwrap_or_else(|| panic!("{label:?} is not on screen: {:?}", text_list(&out)))
        .rect
        .center()
}

struct Text {
    text: String,
    rect: Rect,
    color: Option<Color32>,
}

// Bandcamp pages, with fake pages and previews.
mod bandcamp_tests;
// Digging, with a fake Discogs, fake previews and a fake browser.
mod dig_tests;
mod record_tests;
// Friends' collections, with a fake Discogs.
mod friend_tests;
// A Bandcamp fan's collection and wishlist, with fake pages and a fake Discogs.
mod fan_tests;
// Juno records sent by the extension, with a fake Discogs and fake previews.
mod juno_tests;
// Bleep records sent by the extension.
mod bleep_tests;
// Beatport and Traxsource tracks sent by the extension.
mod store_tests;
// Juno and Bleep baskets through a fake extension.
mod store_cart_tests;
// Top Sellers and the cart, with a fake Discogs.
mod seller_tests;
// The collection's Insights window, with a fake Discogs.
mod insights_tests;
// Things appearing and disappearing with motion on.
mod motion_tests;
// The mini player's level meter.
mod meter_tests;

fn texts(out: &egui::FullOutput) -> Vec<Text> {
    fn walk(shape: &egui::Shape, acc: &mut Vec<Text>) {
        match shape {
            egui::Shape::Text(t) => acc.push(Text {
                text: t.galley.text().to_owned(),
                rect: t.visual_bounding_rect(),
                // As drawn: a placeholder colour takes the shape's fallback.
                color: t.galley.job.sections.first().map(|s| match s.format.color {
                    Color32::PLACEHOLDER => t.fallback_color,
                    c => c,
                }),
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

/// Filled rectangles drawn, with their colour.
fn rects(out: &egui::FullOutput) -> Vec<(Rect, Color32)> {
    fn walk(shape: &egui::Shape, acc: &mut Vec<(Rect, Color32)>) {
        match shape {
            egui::Shape::Rect(r) => acc.push((r.rect, r.fill)),
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

// ---- SYNC ------------------------------------------------------------------------------

/// A point in the turntable's control `name` (a layout entry), at fractions (`fx`, `fy`) of its
/// rectangle, in a window `rig.screen.x` wide: the turntable's content is centred in its panel.
fn deck_at(rig: &Rig, name: &str, fx: f32, fy: f32) -> Pos2 {
    let panels = crate::layout::band_panels(rig.screen.x);
    let ox = ((panels.deck.1 as f32 - crate::skin::generate::DECK_W as f32) / 2.0).floor();
    let r = rig.app.def.at(name);
    pos2(
        panels.deck.0 as f32 + ox + r.x as f32 + fx * r.w as f32,
        BAND_TOP + r.y as f32 + fy * r.h as f32,
    )
}

fn deck_sync(rig: &Rig) -> Pos2 {
    deck_at(rig, "deck_sync", 0.5, 0.5)
}

/// What the turntable last drew: its state line, its big digits and the meter's needle.
fn deck(rig: &Rig) -> (String, String, Option<f32>) {
    rig.app.deck_drawn.clone().expect("the turntable is drawn")
}

#[test]
fn sync_lights_the_turntable_whose_fader_sets_the_tempo() {
    let mut rig = Rig::new("sync", vec![fixture("tone.flac")], |_| {});
    rig.screen.x = 1100.0;
    rig.until(
        |r| {
            r.app.position.state == PlayState::Playing
                && r.app.track_refs.contains_key(&r.app.position.track)
        },
        "the track plays",
    );
    let track = rig.app.track_refs[&rig.app.position.track].clone();
    rig.app.set_track_bpm(&track, 123.0);
    let rows = rig.app.pl_geometry().1;
    // Off: the turntable is there, dimmed, with the record's own BPM; the fader does nothing.
    rig.frame(Vec::new());
    let (line, digits, needle) = deck(&rig);
    assert_eq!(
        (line.as_str(), digits.as_str(), needle),
        ("SYNC OFF", "123", None)
    );
    rig.click(deck_at(&rig, "deck_pitch", 0.5, 0.0));
    rig.click(deck_at(&rig, "deck_reset", 0.5, 0.5));
    assert!(!rig.app.settings.sync.on);
    assert_eq!(
        rig.app.settings.sync.cents(),
        12_400,
        "the target is unchanged while off"
    );
    // On: the record's tempo becomes the target, and plays on as it is.
    rig.click(deck_sync(&rig));
    assert!(rig.app.settings.sync.on);
    assert_eq!(rig.app.settings.sync.cents(), 12_300);
    rig.frame(Vec::new());
    assert_eq!(
        rig.app.pl_geometry().1,
        rows,
        "the band doesn't change height"
    );
    assert_eq!(rig.engine().speed(0), 1.0);
    // The fader at the top: as fast as the range allows, and heard.
    rig.click(deck_at(&rig, "deck_pitch", 0.5, 0.0));
    assert_eq!(
        rig.app.settings.sync.cents(),
        13_284,
        "123 + 8%, to 0.01 BPM, still locked"
    );
    let fitted = 132.84 / 123.0;
    rig.until(
        |r| (r.app.position.speed - fitted).abs() < 1e-9,
        "heard at the new speed",
    );
    // At its centre detent: the record's own tempo.
    let mid = deck_at(&rig, "deck_pitch", 0.5, 0.5);
    rig.click(mid);
    assert_eq!(rig.app.settings.sync.cents(), 12_300);
    // Shift+↑ and scrolling over the fader step it.
    for _ in 0..3 {
        rig.key(Key::ArrowUp, Modifiers::SHIFT);
    }
    assert_eq!(rig.app.settings.sync.cents(), 12_600);
    assert!(
        rig.app.settings.volume == Settings::default().volume,
        "not the volume"
    );
    rig.frame(vec![
        Event::PointerMoved(mid),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 8.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        },
    ]);
    // (The scroll is smoothed over frames.)
    for _ in 0..30 {
        rig.frame(Vec::new());
    }
    assert!(rig.app.settings.sync.cents() > 12_600, "scrolled up");
    // RESET, the range switch, and SYNC again to turn it off.
    rig.click(deck_at(&rig, "deck_reset", 0.5, 0.5));
    assert_eq!(rig.app.settings.sync.cents(), 12_300);
    rig.until(|r| r.app.position.speed == 1.0, "back to its own speed");
    rig.click(deck_at(&rig, "deck_range", 0.5, 0.5));
    assert!(rig.app.settings.sync.wide);
    rig.click(deck_sync(&rig));
    assert!(!rig.app.settings.sync.on);
    assert_eq!(rig.app.pl_geometry().1, rows);
    rig.frame(Vec::new());
    assert_eq!(deck(&rig).0, "SYNC OFF");
}

#[test]
fn the_turntable_says_what_sync_is_doing() {
    let mut rig = Rig::new("sync-state", vec![fixture("tone.flac")], |_| {});
    rig.screen.x = 720.0;
    rig.until(
        |r| {
            r.app.position.state == PlayState::Playing
                && r.app.track_refs.contains_key(&r.app.position.track)
        },
        "the track plays",
    );
    let track = rig.app.track_refs[&rig.app.position.track].clone();
    rig.app.set_track_bpm(&track, 124.0);
    let set = |rig: &mut Rig, target: u16| {
        rig.app.settings.sync = crate::sync::SyncSettings {
            on: true,
            target,
            target_cents: None,
            wide: false,
            pending: None,
        };
        rig.frame(Vec::new());
        deck(rig)
    };
    // 124 against 102: out of the ±8% range, so it plays at its own tempo, the needle pinned
    // to the slow end.
    let (line, digits, needle) = set(&mut rig, 102);
    assert_eq!(
        (line.as_str(), digits.as_str(), needle),
        ("OWN · OUT OF ±8%", "102", Some(-1.0))
    );
    // 126 against 124: locked, pushed 1.6% up, the needle a fifth of the way to the fast end.
    let (line, _, needle) = set(&mut rig, 126);
    assert_eq!(line, "LOCKED +1.6%");
    assert!((needle.unwrap() - 0.2).abs() < 0.01, "{needle:?}");
    // Waiting for a BPM: the track has none yet, and SYNC waits for it.
    rig.app
        .crates
        .playing_mut()
        .entries_mut()
        .for_each(|e| e.bpm = None);
    rig.app.settings.sync.pending = Some(rig.app.position.track);
    rig.frame(Vec::new());
    let (line, digits, needle) = deck(&rig);
    assert_eq!(
        (line.as_str(), digits.as_str(), needle),
        ("ANALYZING", "  ?", None)
    );
    // A track without a BPM, once nothing waits for it.
    rig.app.settings.sync.pending = None;
    rig.frame(Vec::new());
    assert_eq!(deck(&rig).0, "OWN · NO BPM");
    // Paused or stopped with SYNC on.
    rig.key(Key::Space, Modifiers::SHIFT);
    rig.until(|r| r.app.position.state == PlayState::Stopped, "stopped");
    assert_eq!(deck(&rig).0, "SYNC ON");
}

#[test]
fn a_track_out_of_range_plays_at_its_own_tempo_and_says_so() {
    let mut rig = Rig::new("sync-own", Vec::new(), |_| {});
    let ids = rig.fill_playlist(3);
    let p = rig.app.crates.shown_mut();
    for (id, bpm) in ids.iter().zip([Some(122), Some(140), None]) {
        p.entries_mut().find(|e| e.id == *id).unwrap().bpm = bpm;
    }
    rig.app.settings.sync = crate::sync::SyncSettings {
        on: true,
        target: 126,
        target_cents: None,
        wide: false,
        pending: None,
    };
    let out = rig.frame(Vec::new());
    let own: Vec<String> = text_list(&out)
        .into_iter()
        .filter(|t| t.ends_with("(own tempo)"))
        .collect();
    assert_eq!(own.len(), 2, "140 BPM and no BPM: {own:?}");
    assert!(own[0].contains("(140 BPM)"), "{own:?}");
    // Playing the first, fitted: it starts at 126/122; the next is set before it starts.
    rig.app.play_entry(PLAYLIST, ids[0]);
    rig.until(
        |r| r.app.position.state == PlayState::Playing,
        "the first plays",
    );
    rig.pump();
    let e = rig.engine();
    assert!((e.speed(0) - 126.0 / 122.0).abs() < 1e-9);
    assert_eq!((e.speed(1), e.speed(2)), (1.0, 1.0));
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
fn a_crate_that_isnt_shown_exports_without_being_shown() {
    let mut rig = Rig::new("export-other", Vec::new(), |_| {});
    let friday = rig.crate_with("Friday", &["tone.mp3", "tone.ogg"]);
    let other = rig.crate_with("Lowtide Tapes", &["tone.wav"]);
    rig.app.apply(Action::ShowCrate(other), &rig.ctx.clone());
    rig.app.crates.shown_mut().select_all();
    let selected = rig.app.crates.shown().selected_ids();
    let m3u = rig.app.m3u_of(friday).expect("Friday reads");
    assert!(
        m3u.contains("tone.mp3") && m3u.contains("tone.ogg"),
        "{m3u}"
    );
    assert!(!m3u.contains("tone.wav"), "only Friday's entries");
    assert_eq!(rig.app.crates.shown_id(), other, "the shown crate stays");
    assert_eq!(rig.app.crates.shown().selected_ids(), selected);
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
    let b_tracks: Vec<TrackRef> = rig.entries(b).iter().map(|e| e.track.clone()).collect();
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
        shows(&out, "Waiting for Nightcraft: Glasshouse (downloading 40%)"),
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
fn command_line_files_replace_only_the_playlist_crate() {
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
        r.entries(c)
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
    assert!(rig.entries(lt)[0].track.0.ends_with("tone.mp3"));

    // Cmd+O adds to the shown Playlist crate, never replaces it.
    rig.app.add_paths(vec![fixture("tone.m4a")], Open::Add);
    assert_eq!(rig.app.crates.shown().len(), 3);
    assert!(rig.entries(lt)[0].track.0.ends_with("tone.mp3"));
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
    assert_eq!(color_of("3. Nightcraft: Glasshouse"), Some(dim));
    assert_eq!(color_of("4. Nightcraft: B-side"), Some(dim));
    // Their states are icons now, with the words in the tooltip.
    assert!(!shows(&out, "downloading 40%") && !shows(&out, "no clip"));
    // The download is a bar: 40% of its 30 pixels filled, in the dimmed colour.
    let row = Rect::from_min_size(rig.row(2) - vec2(60.0, 6.5), vec2(243.0, 13.0));
    let filled: Vec<Rect> = rects(&out)
        .into_iter()
        .filter(|(r, c)| *c == dim && row.contains_rect(*r) && (r.height() - 3.0).abs() < 0.01)
        .map(|(r, _)| r)
        .collect();
    assert_eq!(filled.len(), 1, "{filled:?}");
    assert!((filled[0].width() - 12.0).abs() < 0.01, "{filled:?}");
    assert_eq!(color_of("2. garbage"), Some(Color32::from_rgb(170, 60, 60)));
    let normal = color_of("1. M83: Midnight_City · Hurry_Up");
    assert!(normal == Some(color(colors.pl_text)) || normal == Some(color(colors.pl_current)));
}

#[test]
fn hovering_an_entry_shows_everything_known_about_it() {
    let mut rig = Rig::new("tooltip", Vec::new(), |_| {});
    rig.ctx.global_style_mut(|s| {
        s.interaction.tooltip_delay = 0.0;
        s.interaction.tooltip_grace_time = 0.0;
    });
    rig.app.crates.shown_mut().add_waiting(
        "Nightcraft",
        "Glasshouse",
        None,
        Some(Origin {
            label: "Lowtide Tapes".into(),
            catno: "LT-012".into(),
            position: "A1".into(),
            ..Default::default()
        }),
        "downloading 40%",
    );
    let at = rig.row(0);
    let mut out = rig.frame(vec![Event::PointerMoved(at)]);
    for _ in 0..10 {
        if shows(&out, "Lowtide Tapes") {
            break;
        }
        out = rig.frame(vec![Event::PointerMoved(at)]);
    }
    for t in [
        "(LT-012) Nightcraft: Glasshouse",
        "Lowtide Tapes",
        "A1",
        "downloading 40%",
    ] {
        assert!(shows(&out, t), "{t}: {:?}", text_list(&out));
    }
}

/// Frames over `at` until `text` shows (tooltips take a frame or two to appear).
fn hover_until(rig: &mut Rig, at: Pos2, text: &str) -> egui::FullOutput {
    let mut out = rig.frame(vec![Event::PointerMoved(at)]);
    for _ in 0..10 {
        if shows(&out, text) {
            break;
        }
        out = rig.frame(vec![Event::PointerMoved(at)]);
    }
    out
}

#[test]
fn a_track_under_an_open_record_leaves_the_record_facts_to_its_card() {
    let mut rig = Rig::new("tooltip-open-record", Vec::new(), |_| {});
    rig.ctx.global_style_mut(|s| {
        s.interaction.tooltip_delay = 0.0;
        s.interaction.tooltip_grace_time = 0.0;
    });
    for (side, title) in [("A1", "Glasshouse"), ("A2", "Tidepool")] {
        rig.app.crates.shown_mut().add_waiting(
            "Nightcraft",
            title,
            None,
            Some(Origin {
                release: Some(12),
                album: "Glasshouse EP".into(),
                label: "Lowtide Tapes".into(),
                catno: "LT-012".into(),
                position: side.into(),
                ..Default::default()
            }),
            "queued",
        );
    }
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::ToggleGrouped, &ctx);
    rig.app.apply(
        Action::ToggleRecord(crate::playlist::AlbumKey::Release(12)),
        &ctx,
    );
    // The record row takes two rows; its first track is the third.
    let at = rig.row(2);
    let out = hover_until(&mut rig, at, "A1");
    assert!(shows(&out, "A1"), "{:?}", text_list(&out));
    for t in ["Lowtide Tapes", "LT-012", "Glasshouse EP"] {
        assert!(!shows(&out, t), "{t} is the card's: {:?}", text_list(&out));
    }
}

/// A grouped crate of "Glasshouse EP" (three tracks, release 12, playable), the single
/// "Lowlight" (release 13) and "Undertow EP" (two tracks, release 14), tooltips at once.
/// Record rows take two rows: 0, 2 and 4.
fn card_rig(name: &str) -> Rig {
    let mut rig = Rig::new(name, Vec::new(), |_| {});
    // Tall, so the card (centred) sits below the rows the tests click.
    rig.screen = vec2(480.0, 900.0);
    rig.ctx.global_style_mut(|s| {
        s.interaction.tooltip_delay = 0.0;
        s.interaction.tooltip_grace_time = 0.0;
    });
    let tracks = [
        (12, "A1", "Tidepool", "Glasshouse EP"),
        (12, "A2", "Undertow", "Glasshouse EP"),
        (12, "B1", "Ripple", "Glasshouse EP"),
        (13, "A", "Lowlight", "Lowlight"),
        (14, "A", "Shoal", "Undertow EP"),
        (14, "B", "Reef", "Undertow EP"),
    ];
    for (release, side, title, album) in tracks {
        let id = rig.app.crates.shown_mut().add_waiting(
            "Nightcraft",
            title,
            None,
            Some(Origin {
                release: Some(release),
                album: album.into(),
                artist: "Nightcraft".into(),
                label: "Lowtide Tapes".into(),
                catno: format!("LT-0{release}"),
                year: Some(1994),
                styles: "Deep House, Minimal".into(),
                position: side.into(),
                ..Default::default()
            }),
            "queued",
        );
        // Each its own file, so the app tells them apart as playback moves on.
        let file = match side {
            "A1" => "tone.flac",
            "A2" => "tone.wav",
            _ => "tone.ogg",
        };
        if release == 12 {
            let t = TrackRef::new(fixture(file).to_string_lossy());
            rig.app.set_audio(PLAYLIST, id, t);
        }
    }
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::ToggleGrouped, &ctx);
    rig.frame(Vec::new());
    rig
}

/// Enough frames (egui's clock moves 1/60 s a frame here) that the next click isn't the second
/// of a double-click, which plays.
fn past_double_click(rig: &mut Rig) {
    for _ in 0..20 {
        rig.frame(Vec::new());
    }
}

/// Repeat off (the rig plays with repeat one), so playback moves on as tracks end.
fn repeat_off(rig: &mut Rig) {
    let ctx = rig.ctx.clone();
    while rig.app.settings.repeat != Repeat::Off {
        rig.app.apply(Action::Repeat, &ctx);
    }
}

/// The pressing line the card of release `n` shows.
fn pressing(n: u64) -> String {
    format!("Lowtide Tapes · LT-0{n} · 1994")
}

fn card_on(rig: &Rig) -> Option<AlbumKey> {
    rig.app.card.as_ref().map(|c| c.key.clone())
}

/// The ⏵/⏸ on the card's line for `title`.
fn card_button(out: &egui::FullOutput, title: &str) -> Pos2 {
    let all = texts(out);
    let line = all
        .iter()
        .find(|t| t.text == title)
        .unwrap_or_else(|| panic!("{title:?} not on the card: {:?}", text_list(out)))
        .rect;
    all.iter()
        .filter(|t| t.text == "⏵" || t.text == "⏸")
        .find(|t| (t.rect.center().y - line.center().y).abs() < 3.0 && t.rect.right() < line.left())
        .unwrap_or_else(|| panic!("no play button on {title:?}"))
        .rect
        .center()
}

#[test]
fn every_card_is_the_same_height_whatever_its_record_holds() {
    let height = |n: usize| {
        let mut rig = Rig::new(&format!("record-card-lines-{n}"), Vec::new(), |_| {});
        rig.screen.y = 800.0;
        for i in 0..n {
            rig.app.crates.shown_mut().add_waiting(
                "Nightcraft",
                format!("T{i}"),
                None,
                Some(Origin {
                    release: Some(12),
                    album: "Glasshouse EP".into(),
                    position: format!("A{i}"),
                    ..Default::default()
                }),
                "queued",
            );
        }
        let ctx = rig.ctx.clone();
        rig.app.apply(Action::ToggleGrouped, &ctx);
        rig.frame(Vec::new());
        rig.click(rig.row(0));
        rig.frame(Vec::new());
        rig.app.card_rect.expect("the card").height()
    };
    let eight = height(8);
    for n in [1, 3, 14] {
        let h = height(n);
        assert!((h - eight).abs() < 0.5, "{n} tracks: {h} vs {eight}");
    }
}

#[test]
fn hovering_a_record_row_shows_no_card() {
    let mut rig = card_rig("record-card-hover");
    let at = rig.row(0);
    let mut out = rig.frame(vec![Event::PointerMoved(at)]);
    for _ in 0..10 {
        out = rig.frame(vec![Event::PointerMoved(at)]);
    }
    assert!(!shows(&out, &pressing(12)), "{:?}", text_list(&out));
    assert!(!shows(&out, "Side"), "nor a tooltip");
    assert_eq!(card_on(&rig), None);
}

#[test]
fn a_click_on_a_record_row_opens_its_card() {
    let mut rig = card_rig("record-card-click");
    rig.click(rig.row(0));
    let out = rig.frame(Vec::new());
    for t in [
        "Nightcraft",
        "Glasshouse EP",
        pressing(12).as_str(),
        "Deep House",
        "Minimal",
        "A1",
        "Tidepool",
        "B1",
        "Ripple",
    ] {
        assert!(shows(&out, t), "{t}: {:?}", text_list(&out));
    }
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(12)));
    assert_eq!(
        rig.app.crates.shown().selected_ids().len(),
        3,
        "the record is selected"
    );
    // While it's open, nothing outside it takes the pointer: another record row's click
    // does nothing.
    rig.click(rig.row(2));
    assert_eq!(
        card_on(&rig),
        Some(AlbumKey::Release(12)),
        "the card stays on its record"
    );
    assert_eq!(
        rig.app.crates.shown().selected_ids().len(),
        3,
        "the click didn't reach the row"
    );
    // × closes it; then the single, a record of one entry, has its card too.
    rig.click_text("×");
    assert_eq!(card_on(&rig), None);
    past_double_click(&mut rig);
    rig.click(rig.row(2));
    let out = rig.frame(Vec::new());
    assert!(shows(&out, &pressing(13)), "{:?}", text_list(&out));
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(13)));
}

#[test]
fn a_shift_or_cmd_click_only_selects() {
    let mut rig = card_rig("record-card-mods");
    rig.click_with_mods(rig.row(0), Modifiers::SHIFT);
    assert_eq!(card_on(&rig), None);
    rig.click_with_mods(rig.row(4), Modifiers::COMMAND);
    assert_eq!(card_on(&rig), None);
    assert_eq!(
        rig.app.crates.shown().selected_ids().len(),
        5,
        "both records"
    );
}

#[test]
fn the_card_closes_when_its_record_goes() {
    let mut rig = card_rig("record-card-gone");
    let ctx = rig.ctx.clone();
    rig.click(rig.row(0));
    rig.app.apply(Action::ToggleGrouped, &ctx);
    rig.frame(Vec::new());
    assert_eq!(card_on(&rig), None, "ungrouped");
    rig.app.apply(Action::ToggleGrouped, &ctx);
    rig.frame(Vec::new());
    rig.click(rig.row(2));
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(13)));
    rig.app.apply(Action::RemoveSelected, &ctx);
    rig.frame(Vec::new());
    assert_eq!(card_on(&rig), None, "removed");
}

#[test]
fn only_the_card_takes_the_pointer_and_esc_or_its_x_close_it() {
    let mut rig = card_rig("record-card-close");
    rig.click(rig.row(0));
    rig.key(Key::Escape, Modifiers::NONE);
    assert_eq!(card_on(&rig), None);
    assert_eq!(
        rig.app.crates.shown().selected_ids().len(),
        3,
        "selection kept"
    );
    past_double_click(&mut rig);
    rig.click(rig.row(0));
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(12)));
    // The player band, above the playlist: dimmed, it takes nothing, and the card stays.
    rig.click(pos2(PL_LEFT + 20.0, 20.0));
    assert_eq!(
        card_on(&rig),
        Some(AlbumKey::Release(12)),
        "outside: nothing"
    );
    // A wheel over the list doesn't scroll it.
    rig.set_rows(4);
    rig.frame(Vec::new());
    let over = rig.row(1);
    rig.frame(vec![
        Event::PointerMoved(over),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, -400.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        },
    ]);
    for _ in 0..30 {
        rig.frame(vec![Event::PointerMoved(over)]);
    }
    assert_eq!(rig.app.pl_scroll, 0, "the list didn't scroll");
    // Tall again, so the whole card is on screen.
    rig.screen.y = 900.0;
    // A click inside the card (its pressing line) keeps it open.
    let out = rig.frame(Vec::new());
    let line = texts(&out)
        .into_iter()
        .find(|t| t.text == pressing(12))
        .expect("the pressing line")
        .rect;
    rig.click(line.center());
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(12)), "inside");
    rig.click_text("×");
    assert_eq!(card_on(&rig), None, "× closes it");
}

#[test]
fn the_card_is_centred_whichever_row_opened_it_and_its_x_closes_it() {
    let mut rig = card_rig("record-card-centre");
    rig.click(rig.row(0));
    rig.frame(Vec::new());
    let centre = rig.ctx.content_rect().center();
    let first = rig.app.card_rect.expect("open").center();
    rig.click_text("×");
    assert_eq!(card_on(&rig), None, "× closes it");
    assert_eq!(
        rig.app.crates.shown().selected_ids().len(),
        3,
        "selection kept"
    );
    rig.click(rig.row(4));
    rig.frame(Vec::new());
    let last = rig.app.card_rect.expect("open").center();
    for c in [first, last] {
        assert!((c - centre).length() < 1.0, "{c:?} vs {centre:?}");
    }
}

#[test]
fn the_cards_bar_seeks_the_playing_track_of_its_record() {
    let mut rig = card_rig("record-card-seek");
    let a1 = rig.app.crates.shown().entries()[0].id;
    rig.app.apply(Action::PlayEntry(a1), &rig.ctx.clone());
    // Its length read: the bar needs it.
    rig.until(
        |r| {
            r.app.position.state == PlayState::Playing
                && r.app
                    .now_playing_names()
                    .is_some_and(|(_, _, d)| d.is_some())
        },
        "A1 plays, its length known",
    );
    rig.click(rig.row(0));
    let out = rig.frame(Vec::new());
    let all = texts(&out);
    let time = all
        .iter()
        .find(|t| t.text.ends_with("/ 0:02"))
        .unwrap_or_else(|| panic!("the bar's time: {:?}", text_list(&out)))
        .rect;
    let side = all.iter().find(|t| t.text == "A1").expect("A1").rect;
    // The bar runs from the tracklist's left to the time's box, on the time's line.
    let left = side.left() - 16.0 + 2.0;
    let right = time.right() - 62.0;
    rig.click(pos2((left + right) / 2.0, time.center().y));
    rig.until(
        |r| (r.app.position.seconds() - 1.0).abs() < 0.25,
        "seeked to the middle of the 2 s track",
    );
    assert_eq!(rig.app.crates.shown().current(), Some(a1), "the same track");
    // Another record's card: an empty bar, no time, no seeking.
    rig.key(Key::ArrowRight, Modifiers::NONE);
    let out = rig.frame(Vec::new());
    assert!(
        !text_list(&out).iter().any(|t| t.contains(" / ")),
        "{:?}",
        text_list(&out)
    );
}

#[test]
fn the_card_follows_playback_into_the_next_record() {
    let mut rig = card_rig("record-card-follow");
    let ids = rig.ids(PLAYLIST);
    let t = TrackRef::new(fixture("tone.mp3").to_string_lossy());
    rig.app.set_audio(PLAYLIST, ids[3], t);
    // B1, the record's last track, plays with its card open.
    rig.app.apply(Action::PlayEntry(ids[2]), &rig.ctx.clone());
    rig.until(|r| r.app.position.state == PlayState::Playing, "B1 plays");
    repeat_off(&mut rig);
    rig.click(rig.row(0));
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(12)));
    rig.until(
        |r| r.app.crates.shown().current() == Some(ids[3]),
        "the next record's track plays",
    );
    rig.frame(Vec::new());
    assert_eq!(
        card_on(&rig),
        Some(AlbumKey::Release(13)),
        "the card followed"
    );
    assert_eq!(rig.app.crates.shown().selected_ids(), [ids[3]]);
    assert!(
        rig.app
            .card
            .as_ref()
            .and_then(|c| c.moved)
            .is_some_and(|(_, d)| d == 1),
        "it slides in from the right"
    );
}

#[test]
fn a_card_on_another_record_stays_while_playback_moves_on() {
    let mut rig = card_rig("record-card-stay");
    let ids = rig.ids(PLAYLIST);
    rig.app.apply(Action::PlayEntry(ids[0]), &rig.ctx.clone());
    rig.until(|r| r.app.position.state == PlayState::Playing, "A1 plays");
    repeat_off(&mut rig);
    rig.click(rig.row(4));
    rig.until(
        |r| r.app.crates.shown().current() == Some(ids[1]),
        "A2 plays",
    );
    rig.frame(Vec::new());
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(14)), "it stays");
}

#[test]
fn stepping_slides_the_next_record_in_from_the_right() {
    let mut rig = card_rig("record-card-slide");
    rig.click(rig.row(0));
    rig.frame(Vec::new());
    let before = rig.app.card_rect.expect("open");
    assert_eq!(
        rig.app.card.as_ref().unwrap().moved,
        None,
        "opened: no slide"
    );
    rig.click_text("›");
    let moved = rig.app.card.as_ref().unwrap().moved;
    assert!(moved.is_some_and(|(_, d)| d == 1), "{moved:?}");
    rig.frame(Vec::new());
    let after = rig.app.card_rect.expect("open");
    assert!(
        (after.center() - before.center()).length() < 0.5
            && (after.size() - before.size()).length() < 0.5,
        "the card itself doesn't move or resize: {before:?} {after:?}"
    );
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    let moved = rig.app.card.as_ref().unwrap().moved;
    assert!(moved.is_some_and(|(_, d)| d == -1), "back: from the left");
}

#[test]
fn the_card_plays_pauses_and_resumes_its_tracks() {
    let mut rig = card_rig("record-card-play");
    rig.click(rig.row(0));
    let out = rig.frame(Vec::new());
    rig.click(card_button(&out, "Undertow"));
    let a2 = rig.app.crates.shown().entries()[1].id;
    rig.until(
        |r| {
            r.app.position.state == PlayState::Playing && r.app.crates.shown().current() == Some(a2)
        },
        "A2 plays",
    );
    assert_eq!(
        card_on(&rig),
        Some(AlbumKey::Release(12)),
        "the card stays open"
    );
    let out = rig.frame(Vec::new());
    let pause = card_button(&out, "Undertow");
    assert!(
        texts(&out)
            .iter()
            .any(|t| t.text == "⏸" && t.rect.contains(pause)),
        "A2's line shows ⏸"
    );
    rig.click(pause);
    rig.until(|r| r.app.position.state == PlayState::Paused, "paused");
    let out = rig.frame(Vec::new());
    rig.click(card_button(&out, "Undertow"));
    rig.until(|r| r.app.position.state == PlayState::Playing, "resumed");
    assert_eq!(rig.app.crates.shown().current(), Some(a2));
}

#[test]
fn a_track_not_downloaded_yet_plays_from_the_card_and_one_with_no_clip_has_no_button() {
    let mut rig = card_rig("record-card-arm");
    let ids = rig.ids(PLAYLIST);
    rig.app
        .crates
        .shown_mut()
        .entries_mut()
        .nth(5)
        .unwrap()
        .status = EntryStatus::Unavailable(crate::playlist::UnavailableKind::NoClip);
    rig.frame(Vec::new());
    rig.click(rig.row(4));
    let out = rig.frame(Vec::new());
    assert!(
        shows(&out, "queued"),
        "how its audio is coming: {:?}",
        text_list(&out)
    );
    rig.click(card_button(&out, "Shoal"));
    assert_eq!(
        rig.app.armed,
        Some((PLAYLIST, ids[4])),
        "fetched first, played on arrival"
    );
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "next"), "{:?}", text_list(&out));
    let reef = texts(&out)
        .into_iter()
        .find(|t| t.text == "Reef")
        .expect("Reef")
        .rect;
    assert!(
        !texts(&out).iter().any(|t| (t.text == "⏵" || t.text == "⏸")
            && (t.rect.center().y - reef.center().y).abs() < 3.0),
        "no button on a track with no clip"
    );
}

#[test]
fn the_card_steps_through_records_with_its_arrows_and_the_keys() {
    let mut rig = card_rig("record-card-step");
    rig.click(rig.row(0));
    rig.click_text("›");
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(13)));
    let p = rig.app.crates.shown();
    assert_eq!(
        p.selected_ids(),
        [p.entries()[3].id],
        "the next record is selected"
    );
    rig.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(14)));
    rig.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(
        card_on(&rig),
        Some(AlbumKey::Release(14)),
        "the last record: nothing"
    );
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(12)));
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(
        card_on(&rig),
        Some(AlbumKey::Release(12)),
        "the first: nothing"
    );
    // Closed, → is the list's again: it doesn't step a card.
    rig.key(Key::Escape, Modifiers::NONE);
    rig.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(card_on(&rig), None);
}

#[test]
fn stepping_scrolls_the_list_to_the_record_and_never_changes_what_plays() {
    let mut rig = card_rig("record-card-scroll");
    // Room for two record rows: the third is out of view.
    rig.set_rows(4);
    rig.frame(Vec::new());
    let a1 = rig.app.crates.shown().entries()[0].id;
    rig.app.apply(Action::PlayEntry(a1), &rig.ctx.clone());
    rig.until(|r| r.app.position.state == PlayState::Playing, "A1 plays");
    rig.click(rig.row(0));
    assert_eq!(rig.app.pl_scroll, 0);
    rig.key(Key::ArrowRight, Modifiers::NONE);
    rig.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(14)));
    assert_eq!(rig.app.pl_scroll, 1, "scrolled by one record");
    assert_eq!(rig.app.crates.shown().current(), Some(a1), "A1 plays on");
    assert_eq!(rig.app.position.state, PlayState::Playing);
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
    assert_eq!(
        rig.app.now_playing_line().as_deref(),
        Some("1. Nightcraft: Glasshouse (0:02)")
    );
    let e = &rig.app.crates.shown().entries()[0];
    assert_eq!(e.display_name(), "Nightcraft: Glasshouse");
    assert!(
        e.duration.is_some_and(|d| (d - 2.0).abs() < 0.05),
        "takes the duration"
    );
}

#[test]
fn a_click_or_tab_gives_a_side_the_arrow_keys() {
    let mut rig = Rig::new("focus", Vec::new(), |_| {});
    let ids = rig.fill_playlist(12);
    rig.frame(Vec::new());
    assert_eq!(rig.app.focus, Focus::Player, "the player at launch");
    let v = rig.app.settings.volume;
    rig.key(Key::ArrowUp, Modifiers::NONE);
    assert!(rig.app.settings.volume > v, "↑ is volume");
    assert_eq!(rig.app.crates.shown().cursor(), None);

    rig.click(rig.row(2));
    assert_eq!(rig.app.focus, Focus::Playlist);
    let v = rig.app.settings.volume;
    rig.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.app.settings.volume, v, "↓ no longer touches the volume");
    assert_eq!(rig.app.crates.shown().cursor(), Some(ids[3]));
    rig.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(rig.app.crates.shown().selected_ids(), &ids[3..=4]);

    // Tab hands the keyboard back, and egui's own widget focus doesn't swallow it.
    rig.key(Key::Tab, Modifiers::NONE);
    assert_eq!(rig.app.focus, Focus::Player);
    assert!(!rig.ctx.egui_wants_keyboard_input());
    rig.key(Key::ArrowDown, Modifiers::NONE);
    assert!(rig.app.settings.volume < v);
    assert_eq!(rig.app.crates.shown().cursor(), Some(ids[4]));
    rig.key(Key::Tab, Modifiers::NONE);
    assert_eq!(rig.app.focus, Focus::Playlist);

    // A click on the player (on a bare spot of its panel) takes it back.
    rig.click(pos2(190.0, 45.0));
    assert_eq!(rig.app.focus, Focus::Player);
}

#[test]
fn the_cursor_scrolls_the_list_and_enter_plays_it() {
    let mut rig = Rig::new("cursor", Vec::new(), |_| {});
    rig.set_rows(10);
    let ids = rig.fill_playlist(40);
    rig.click(rig.row(8));
    rig.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.app.pl_scroll, 0, "entry 9 is still on screen");
    rig.key(Key::ArrowDown, Modifiers::NONE);
    rig.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.app.crates.shown().cursor(), Some(ids[11]));
    assert_eq!(
        rig.app.pl_scroll, 2,
        "scrolled just enough to show entry 12"
    );
    rig.key(Key::End, Modifiers::NONE);
    assert_eq!(rig.app.pl_scroll, 30);
    rig.key(Key::PageUp, Modifiers::NONE);
    assert_eq!(rig.app.crates.shown().cursor(), Some(ids[29]));
    rig.key(Key::Home, Modifiers::NONE);
    assert_eq!(rig.app.pl_scroll, 0);
    rig.key(Key::Enter, Modifiers::NONE);
    rig.until(
        |r| r.app.crates.playing().current() == Some(ids[0]),
        "the cursor's entry plays",
    );
}

#[test]
fn p_shows_the_playing_entry_and_the_list_follows_it_only_from_view() {
    let mut rig = Rig::new("follow", Vec::new(), |_| {});
    rig.set_rows(10);
    let ids = rig.fill_playlist(40);
    rig.app.crates.shown_mut().set_current(Some(ids[25]));
    rig.frame(Vec::new());
    rig.key(Key::P, Modifiers::NONE);
    assert_eq!(rig.app.crates.shown().cursor(), Some(ids[25]));
    assert_eq!(rig.app.pl_scroll, 16, "entry 26 in the last row");

    // The next track, one row below the view: the list follows.
    rig.app.crates.shown_mut().set_current(Some(ids[26]));
    rig.frame(Vec::new());
    assert_eq!(rig.app.pl_scroll, 17);
    // Scrolled away by the user: the list stays.
    rig.app.pl_scroll = 0;
    rig.frame(Vec::new());
    rig.app.crates.shown_mut().set_current(Some(ids[27]));
    rig.frame(Vec::new());
    assert_eq!(rig.app.pl_scroll, 0);
}

#[test]
fn a_sort_reorders_the_crate_and_next_follows_it() {
    let mut rig = Rig::new(
        "sort",
        vec![
            fixture("tone.flac"),
            fixture("tone.wav"),
            fixture("tone.ogg"),
        ],
        |_| {},
    );
    rig.until(
        |r| r.app.position.state == PlayState::Playing,
        "the first file plays",
    );
    let ids = rig.ids(PLAYLIST);
    let set = |r: &mut Rig, i: usize, title: &str| {
        let e = r
            .app
            .crates
            .shown_mut()
            .entries_mut()
            .find(|e| e.id == ids[i])
            .unwrap();
        e.title = title.into();
    };
    set(&mut rig, 0, "b");
    set(&mut rig, 1, "c");
    set(&mut rig, 2, "a");
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Sort(Field::Title), &ctx);
    assert_eq!(rig.ids(PLAYLIST), [ids[2], ids[0], ids[1]]);
    assert_eq!(
        rig.app.crates.shown().sorted(),
        Some((Field::Title, Dir::Asc))
    );
    rig.app.apply(Action::Sort(Field::Title), &ctx);
    assert_eq!(
        rig.ids(PLAYLIST),
        [ids[1], ids[0], ids[2]],
        "again: the other way"
    );
    // "b" keeps playing, and "a" (now after it) is next.
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.playing().current(), Some(ids[0]));
    assert_eq!(rig.app.position.state, PlayState::Playing);
    rig.app.apply(Action::Next, &ctx);
    rig.until(
        |r| r.app.crates.playing().current() == Some(ids[2]),
        "next follows the new order",
    );
}

#[test]
fn a_wide_playlist_shows_columns_that_sort_and_hide() {
    let mut rig = Rig::new("columns", vec![fixture("tone.wav")], |_| {});
    let p = rig.app.crates.shown_mut();
    let dig = p.add_waiting(
        "Nightcraft",
        "Glasshouse",
        None,
        Some(Origin {
            catno: "LT-012".into(),
            position: "A1".into(),
            year: Some(1994),
            ..Default::default()
        }),
        "listed",
    );
    p.entries_mut().find(|e| e.id == dig).unwrap().bpm = Some(124);
    rig.screen.x = 700.0;
    let out = rig.frame(Vec::new());
    for t in [
        "#", "Cat#", "Artist", "Title", "BPM", "Side", "Year", "For sale", "Time",
    ] {
        assert!(shows(&out, t), "header {t}: {:?}", text_list(&out));
    }
    for t in [
        "LT-012",
        "Nightcraft",
        "Glasshouse",
        "124",
        "A1",
        "1994",
        "2.",
    ] {
        assert!(shows(&out, t), "cell {t}: {:?}", text_list(&out));
    }
    assert!(!shows(&out, "2. (LT-012) Nightcraft: Glasshouse (124 BPM)"));

    // A click on BPM sorts (the tone has no tempo: it goes last); again, the other way.
    rig.click_text("BPM");
    assert_eq!(
        rig.app.crates.shown().sorted(),
        Some((Field::Bpm, Dir::Asc))
    );
    assert_eq!(rig.app.crates.shown().entries()[0].id, dig);
    rig.click_text("BPM");
    assert_eq!(
        rig.app.crates.shown().sorted(),
        Some((Field::Bpm, Dir::Desc))
    );
    // The direction is a drawn triangle, not a character the font may lack.
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "BPM"));
    assert!(
        !text_list(&out)
            .iter()
            .any(|t| t.contains('▼') || t.contains('▲'))
    );

    // Right-click the header to hide a column.
    let year = texts(&rig.frame(Vec::new()))
        .into_iter()
        .find(|t| t.text == "Year")
        .unwrap()
        .rect
        .center();
    rig.click_with(year, PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(
        shows(&out, "Cat#") && shows(&out, "Side"),
        "{:?}",
        text_list(&out)
    );
    // The menu's checkbox is drawn after the header's own "Year".
    let check = texts(&out)
        .into_iter()
        .rfind(|t| t.text == "Year")
        .unwrap()
        .rect
        .center();
    assert_ne!(check, year, "the menu shows");
    rig.click(check);
    assert!(!rig.app.settings.columns.shows(Field::Year));
    rig.key(Key::Escape, Modifiers::NONE);
    assert!(!shows(&rig.frame(Vec::new()), "1994"));

    // Narrow again: the single line, no header.
    rig.screen.x = 400.0;
    let out = rig.frame(Vec::new());
    assert!(!shows(&out, "Cat#"));
    assert!(
        shows(&out, "(LT-012) Nightcraft: Glasshouse (124 BPM)"),
        "{:?}",
        text_list(&out)
    );
}

#[test]
fn trackpad_scrolling_moves_the_list_and_forgets_leftovers_off_it() {
    let mut rig = Rig::new("scroll", Vec::new(), |_| {});
    rig.fill_playlist(40);
    let over = rig.row(3);
    // Slow two-finger scrolling: many small steps, each far less than a row.
    for _ in 0..30 {
        rig.frame(vec![
            Event::PointerMoved(over),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -2.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            },
        ]);
    }
    for _ in 0..30 {
        rig.frame(vec![Event::PointerMoved(over)]);
    }
    assert!(rig.app.pl_scroll >= 3, "moved {} rows", rig.app.pl_scroll);
    rig.app.pl_scroll_acc = 10.0;
    rig.frame(vec![Event::PointerMoved(pos2(150.0, 45.0))]);
    assert_eq!(rig.app.pl_scroll_acc, 0.0, "dropped off the list");
}

#[test]
fn the_mini_player_is_the_player() {
    let mut rig = Rig::new(
        "mini-player",
        vec![fixture("tone.flac"), fixture("tone.wav")],
        |_| {},
    );
    rig.screen.x = 1100.0;
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    let ids = rig.ids(PLAYLIST);
    let rows = rig.app.pl_geometry().1;
    // The waveform is always in the band: W no longer does anything, and the rows don't move.
    rig.key(Key::W, Modifiers::NONE);
    assert_eq!(rig.app.pl_geometry().1, rows);
    // Pause is the third button of the second line, play the second.
    let button = |i: f32| pos2(4.0 + 23.0 * i + 11.0, BAND_TOP + TRANSPORT_Y + 9.0);
    let player_w = crate::layout::band_panels(rig.screen.x).player.1 as f32;
    rig.click(button(2.0));
    rig.until(
        |r| r.app.position.state == PlayState::Paused,
        "pause pauses",
    );
    rig.click(button(1.0));
    rig.until(
        |r| r.app.position.state == PlayState::Playing,
        "play resumes",
    );
    // Shuffle and repeat, at the end of the button row after the volume slider.
    let shuffle = pos2(shuffle_x(player_w) + 8.0, BAND_TOP + BUTTON_ROW_Y + 6.0);
    rig.click(shuffle);
    assert!(rig.app.settings.shuffle);
    rig.click(shuffle);
    assert!(!rig.app.settings.shuffle);
    let repeat = pos2(
        shuffle_x(player_w) + ICON_W + ICON_GAP + 8.0,
        BAND_TOP + BUTTON_ROW_Y + 6.0,
    );
    let was = rig.app.settings.repeat;
    rig.click(repeat);
    assert_eq!(rig.app.settings.repeat, was.cycle());
    for _ in 0..2 {
        rig.click(repeat);
    }
    assert_eq!(rig.app.settings.repeat, was, "around the three");
    // SYNC, on the turntable beside the player, turns SYNC on and says what it does.
    rig.click(deck_sync(&rig));
    assert!(rig.app.settings.sync.on);
    let said = rig.app.message.as_ref().map(|(t, _)| t.clone());
    assert!(
        said.as_deref()
            .is_some_and(|t| t.starts_with("SYNC on: every track")),
        "{said:?}"
    );
    assert_eq!(rig.app.pl_geometry().1, rows, "the band doesn't change");
    rig.click(deck_sync(&rig));
    assert!(!rig.app.settings.sync.on);
    assert_eq!(rig.app.pl_geometry().1, rows);
    // Next is the fifth.
    rig.click(button(4.0));
    rig.until(
        |r| r.app.crates.playing().current() == Some(ids[1]),
        "the mini player's next starts the next track",
    );
    // The LCD row's buttons: ▤, EQ, spectrum.
    let icon = |i: f32| {
        pos2(
            lcd_icons_x(player_w) + 19.0 * i + 8.0,
            BAND_TOP + DIGITS_Y + 6.0,
        )
    };
    rig.click(icon(1.0));
    assert!(rig.app.settings.show_eq, "the EQ window opens");
    rig.click(icon(1.0));
    assert!(!rig.app.settings.show_eq);
    rig.click(icon(2.0));
    assert!(rig.app.spectro_open, "the spectrogram opens, as with S");
    // (Here it is drawn over the band, having no window of its own: S closes it.)
    rig.key(Key::S, Modifiers::NONE);
    assert!(!rig.app.spectro_open);
    // ▤ groups the crate shown in the playlist, and ungroups it.
    assert!(!rig.app.crates.shown().is_grouped());
    rig.click(icon(0.0));
    assert!(rig.app.crates.shown().is_grouped(), "▤ groups by record");
    rig.click(icon(0.0));
    assert!(!rig.app.crates.shown().is_grouped());
    // A click on the time shows what remains.
    rig.click(pos2(40.0, BAND_TOP + 15.0));
    assert!(rig.app.settings.time_remaining);
}

#[test]
fn the_mini_players_title_line_flashes_a_verdict() {
    let mut rig = Rig::new("flash", vec![fixture("tone.flac")], |_| {});
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    // (The LCD is drawn in the skin's bitmap font, so this checks what the line holds.)
    rig.app.flash(Verdict::Wanted, 1);
    rig.frame(Vec::new());
    assert_eq!(rig.app.flash.as_ref().map(|f| f.0.as_str()), Some("WANTED"));
    // It ends by itself, and the track line comes back.
    rig.app.flash = Some(("WANTED".into(), Instant::now() - FLASH_SECS));
    rig.frame(Vec::new());
    assert!(rig.app.flash.is_none(), "the mini player ends it");
}

#[test]
fn the_buttons_fit_in_the_mini_player() {
    let d = LoadedSkin::default_skin().def;
    // Narrowest (the default window's third) and wider: the rows end together, with even gaps.
    for w in [227.0_f32, 300.0, 353.0, 360.0] {
        let vol_x = VOLUME_X;
        let vol_w = shuffle_x(w) - 4.0 - vol_x;
        assert!(
            vol_w >= 60.0,
            "{w}: the volume keeps 60 pixels, has {vol_w}"
        );
        let row = [
            (4.0, 5.0 * 23.0),
            (vol_x, vol_w),
            (shuffle_x(w), ICON_W),
            (shuffle_x(w) + ICON_W + ICON_GAP, ICON_W),
        ];
        for pair in row.windows(2) {
            let gap = pair[1].0 - (pair[0].0 + pair[0].1);
            assert!(
                (ICON_GAP..=4.0).contains(&gap),
                "{w}: {pair:?}: a gap of {gap}"
            );
        }
        let (x, rw) = row[row.len() - 1];
        assert_eq!(x + rw, w - 4.0, "{w}: ends 4 pixels in, like the LCD icons");
        // The LCD's icons: ▤, EQ and spectrum end there too, and the track line keeps room.
        let x0 = lcd_icons_x(w);
        assert_eq!(x0 + 3.0 * (ICON_W + ICON_GAP) - ICON_GAP, w - 4.0, "{w}");
        let chars = ((x0 - 3.0 - 74.0) / d.font.advance as f32).floor();
        assert!(chars >= 15.0, "{w}: {chars} characters of title");
    }
    // Everything in the button row shares the transport's centre line, to half a pixel.
    let centre = TRANSPORT_Y + 9.0;
    for h in [12.0, 13.0] {
        assert!((BUTTON_ROW_Y + h / 2.0 - centre).abs() <= 0.5, "{h}");
    }
    // The seek bar sits under the transport and inside the band (checked when compiled).
    const {
        assert!(
            SEEK_Y - 2.0 >= TRANSPORT_Y + 18.0,
            "its hit area starts under the transport"
        );
        let seek_h = crate::skin::generate::SEEK_H as f32;
        assert!(
            SEEK_Y + seek_h + 1.0 <= crate::layout::BAND_H as f32,
            "and ends in the band"
        );
    }
    // The LCD's rows fit its box and the band.
    assert!(LINE2_Y + 7.0 <= LCD_Y + LCD_H && TRANSPORT_Y + 18.0 <= crate::layout::BAND_H as f32);
}

/// The centre of the header's button in slot `n`, counted from the right (0 close, 1 minimize,
/// 2 the Options gear, 3 help), in a window `rig.screen.x` wide.
fn header_button(rig: &Rig, n: f32) -> Pos2 {
    let hb = crate::skin::generate::HB as f32;
    pos2(
        rig.screen.x - HB_MARGIN - hb - n * (hb + HB_GAP) + hb / 2.0,
        HB_Y + hb / 2.0,
    )
}

/// The header's commands in `out`.
fn commands(out: &egui::FullOutput) -> Vec<ViewportCommand> {
    out.viewport_output
        .values()
        .flat_map(|v| v.commands.clone())
        .collect()
}

#[test]
fn the_header_has_help_the_gear_minimize_and_close() {
    let mut rig = Rig::new("window-header", Vec::new(), |_| {});
    rig.screen.x = 1100.0;
    rig.frame(Vec::new());
    let press = |rig: &mut Rig, at: Pos2| {
        rig.frame(vec![Event::PointerMoved(at)]);
        rig.press(at, PointerButton::Primary, true);
        commands(&rig.press(at, PointerButton::Primary, false))
    };
    // Minimize.
    let min = header_button(&rig, 1.0);
    assert!(press(&mut rig, min).contains(&ViewportCommand::Minimized(true)));
    // Without digging there are no options: the gear's slot does nothing, nor does a click
    // in any gap, which never quits.
    let gear = header_button(&rig, 2.0);
    assert!(press(&mut rig, gear).is_empty());
    assert!(!egui::Popup::is_any_open(&rig.ctx));
    let hb = crate::skin::generate::HB as f32;
    for n in [0.0, 1.0, 2.0] {
        let gap = header_button(&rig, n) - vec2(hb / 2.0 + HB_GAP / 2.0, 0.0);
        assert!(press(&mut rig, gap).is_empty(), "the gap after slot {n}");
    }
    // Help opens the shortcuts, as H does, and closes them again.
    let help = header_button(&rig, 3.0);
    assert!(!rig.app.help);
    press(&mut rig, help);
    assert!(rig.app.help, "help opens");
    press(&mut rig, help);
    assert!(!rig.app.help);
    // Close keeps its place at the right and quits.
    let close = header_button(&rig, 0.0);
    assert!(
        press(&mut rig, close).contains(&ViewportCommand::Close),
        "close quits"
    );
}

#[test]
fn the_header_drags_the_window_and_a_double_click_maximizes_it() {
    let mut rig = Rig::new("header-drag", Vec::new(), |_| {});
    rig.screen.x = 1100.0;
    rig.frame(Vec::new());
    // Empty header, left of the caption.
    let at = pos2(200.0, 7.0);
    rig.frame(vec![Event::PointerMoved(at)]);
    let mut sent = commands(&rig.press(at, PointerButton::Primary, true));
    for dx in [4.0, 10.0, 16.0] {
        sent.extend(commands(
            &rig.frame(vec![Event::PointerMoved(at + vec2(dx, 0.0))]),
        ));
    }
    assert!(sent.contains(&ViewportCommand::StartDrag), "{sent:?}");
    rig.press(at + vec2(16.0, 0.0), PointerButton::Primary, false);
    // A double-click on the caption maximizes (the test window isn't maximized yet).
    let caption = pos2(550.0, 7.0);
    rig.frame(vec![Event::PointerMoved(caption)]);
    let mut all = Vec::new();
    for _ in 0..2 {
        all.extend(commands(&rig.press(caption, PointerButton::Primary, true)));
        all.extend(commands(&rig.press(caption, PointerButton::Primary, false)));
    }
    assert!(
        all.contains(&ViewportCommand::Maximized(true)),
        "double-click: {all:?}"
    );
}

#[test]
fn the_header_is_lit_with_the_player_and_dimmed_with_the_playlist() {
    let mut rig = Rig::new("header-focus", Vec::new(), |_| {});
    rig.screen.x = 1100.0;
    rig.frame(Vec::new());
    assert_eq!(
        rig.app.focus,
        Focus::Player,
        "the player starts with the keyboard"
    );
    // A click in the playlist gives it the keyboard.
    rig.click(rig.row(0));
    assert_eq!(rig.app.focus, Focus::Playlist);
    // A click in the band gives it back to the player.
    rig.click(pos2(120.0, BAND_TOP + 72.0));
    assert_eq!(rig.app.focus, Focus::Player);
}

#[test]
fn the_old_visuals_folder_is_deleted_after_launch() {
    let mut keep = None;
    let mut rig = Rig::new("old-visuals", Vec::new(), |store| {
        std::fs::create_dir_all(store.dir().join("visuals/variants")).unwrap();
        keep = Some(store.dir().to_path_buf());
    });
    let dir = keep.unwrap();
    rig.until(
        |_| !dir.join("visuals").exists(),
        "the visuals folder is gone",
    );
    assert!(dir.exists(), "the config folder stays");
}

#[test]
fn space_plays_and_pauses_and_shift_space_stops() {
    let mut rig = Rig::new("space", Vec::new(), |_| {});
    rig.fill_playlist(2);
    let state = |r: &Rig| r.app.position.state;
    rig.key(Key::Space, Modifiers::NONE);
    rig.until(
        |r| state(r) == PlayState::Playing,
        "Space plays from stopped",
    );
    rig.key(Key::Space, Modifiers::NONE);
    rig.until(|r| state(r) == PlayState::Paused, "Space pauses");
    rig.key(Key::Space, Modifiers::NONE);
    rig.until(|r| state(r) == PlayState::Playing, "Space resumes");
    // The same with the playlist focused.
    rig.app.focus = Focus::Playlist;
    rig.key(Key::Space, Modifiers::NONE);
    rig.until(
        |r| state(r) == PlayState::Paused,
        "Space pauses, playlist focused",
    );
    rig.key(Key::Space, Modifiers::SHIFT);
    rig.until(|r| state(r) == PlayState::Stopped, "Shift+Space stops");
}

#[test]
fn the_waveform_sits_beside_the_mini_player_and_seeks() {
    let mut rig = Rig::new("band", vec![fixture("tone.flac")], |_| {});
    rig.screen.x = 1100.0;
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    // Three quarters across the overview row, in the waveform panel: 1.5 s of 2 s.
    let wave = crate::layout::band_panels(1100.0).wave;
    let x = wave.0 as f32 + wave.1 as f32 * 0.75;
    rig.click(pos2(x, BAND_TOP + 6.0));
    rig.until(
        |r| (r.app.position.seconds() - 1.5).abs() < 0.2,
        "seeked to 1.5 s",
    );
}

#[test]
fn the_waveform_is_always_in_the_band() {
    let mut rig = Rig::new("wave-always", Vec::new(), |_| {});
    let d = &rig.app.skin.def;
    assert_eq!(d.sprite("tog_wave_off").w, 0, "no waveform button");
    assert_eq!(
        d.sprite("seek_track").w,
        0,
        "and no seek bar to stand in for it"
    );
    let rows = rig.app.pl_geometry().1;
    rig.key(Key::W, Modifiers::NONE);
    assert_eq!(rig.app.pl_geometry().1, rows, "W does nothing");
    // An older settings file that hid it loads, and the waveform shows all the same.
    let old: Settings = ron::from_str("(scale: 1, show_waveform: false)").unwrap();
    assert_eq!(old.scale, 1);
}

#[test]
fn a_known_tempo_shows_on_every_entry_with_that_audio_and_is_saved() {
    let mut rig = Rig::new("bpm", Vec::new(), |_| {});
    let origin = Origin {
        catno: "LT-012".into(),
        clip: Some("aaaaaaaaaaa".into()),
        ..Default::default()
    };
    let p = rig.app.crates.shown_mut();
    let a = p.add_waiting(
        "Nightcraft",
        "Glasshouse",
        None,
        Some(origin.clone()),
        "listed",
    );
    p.add_waiting("Nightcraft", "Glasshouse", None, Some(origin), "listed");
    let track = TrackRef::new(fixture("tone.flac").to_string_lossy());
    rig.app.set_audio(PLAYLIST, a, track.clone());
    // A preview prepared at half time.
    rig.app.set_track_bpm(&track, 62.1);
    let out = rig.frame(Vec::new());
    // Both rows (after their number and cart pill).
    let row = "(LT-012) Nightcraft: Glasshouse (124 BPM)";
    let rows = texts(&out).iter().filter(|t| t.text == row).count();
    assert_eq!(rows, 2, "{row}: {:?}", text_list(&out));
    rig.app.crates.save_due(true, Duration::ZERO);
    let reopened = Crates::open(&Store::new(rig.dir.join("config")));
    let bpms: Vec<_> = reopened.shown().entries().iter().map(|e| e.bpm).collect();
    assert_eq!(bpms, [Some(124), Some(124)]);
}

#[test]
fn the_playing_tracks_whole_score_gives_its_entry_a_tempo() {
    let mut rig = Rig::new("playing-bpm", vec![fixture("tone.flac")], |_| {});
    rig.until(
        |r| {
            r.app.position.state == PlayState::Playing
                && r.app.track_refs.contains_key(&r.app.position.track)
        },
        "the clock reports the playing track",
    );
    let mut score = analysis::SongScore {
        tempo_segments: vec![analysis::score::TempoSegment {
            start: 0.0,
            end: 2.0,
            t0: 0.0,
            period: 60.0 / 128.0,
            confidence: 1.0,
        }],
        ..Default::default()
    };
    rig.app.score = Some(Arc::new(score.clone()));
    rig.app.show_playing_bpm();
    let bpm = |r: &Rig| r.app.crates.playing().entries()[0].bpm;
    assert_eq!(bpm(&rig), None, "not while the score is partial");
    score.complete = true;
    rig.app.score = Some(Arc::new(score));
    rig.app.show_playing_bpm();
    assert_eq!(bpm(&rig), Some(128));
}

// ---- title bar and crate menu --------------------------------------------------------

#[test]
fn the_title_bar_names_the_crate_in_the_skin_font() {
    let def = LoadedSkin::default_skin().def;
    assert_eq!(crate_title(&def, "Lowtide Tapes", 200.0), "LOWTIDE TAPES");
    assert_eq!(
        crate_title(&def, "Canción ₩5", 200.0),
        "CANCION 5",
        "folded, ₩ skipped"
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
    assert!(
        shows(&out, &crate_menu_label("Playlist", true, false, false)),
        "nothing plays: no ⏵"
    );
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

    // Marks: • on the shown crate; ⏵ on the playing one only while it plays.
    rig.click(rig.title_bar());
    let out = rig.frame(Vec::new());
    assert!(shows(
        &out,
        &crate_menu_label("Keepers", true, false, false)
    ));
    assert!(shows(
        &out,
        &crate_menu_label("Playlist", false, false, false)
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
fn the_lp_knob_sweeps_the_filter_and_double_click_turns_it_off() {
    let with_eq = |store: &Store| {
        store
            .save(
                SETTINGS_FILE,
                &Settings {
                    scale: 1,
                    show_eq: true,
                    repeat: Repeat::One,
                    ..Default::default()
                },
            )
            .unwrap();
    };
    let mut rig = Rig::new("lp-knob", Vec::new(), with_eq);
    // The EQ floats under the band without windows of its own; the knob at (46, 17).
    let knob = pos2(46.0 + 7.0, PL_TOP + 17.0 + 7.0);
    assert_eq!(rig.engine().filter(), audio::filter::OFF, "off at launch");
    rig.frame(vec![Event::PointerMoved(knob)]);
    rig.press(knob, PointerButton::Primary, true);
    for dy in [10.0, 30.0, 50.0] {
        rig.frame(vec![Event::PointerMoved(knob + vec2(0.0, dy))]);
    }
    rig.press(knob + vec2(0.0, 50.0), PointerButton::Primary, false);
    rig.frame(Vec::new());
    let lp = rig.app.lp_knob;
    assert!((lp - 0.5).abs() < 0.11, "dragged down by half a turn: {lp}");
    assert_eq!(rig.engine().filter(), lp, "the engine follows");

    rig.double_click(knob);
    assert_eq!(rig.app.lp_knob, audio::filter::OFF);
    assert_eq!(rig.engine().filter(), audio::filter::OFF);
}

/// Sets the tempo of the shown crate's entries, in order.
fn set_tempos(rig: &mut Rig, bpms: &[u16]) {
    let p = rig.app.crates.shown_mut();
    for (e, &b) in p.entries_mut().zip(bpms) {
        e.bpm = Some(b);
    }
}

#[test]
fn under_a_bpm_filter_rows_selection_and_the_cursor_skip_hidden_entries() {
    let mut rig = Rig::new("bpm-rows", Vec::new(), |_| {});
    let ids = rig.fill_playlist(6);
    set_tempos(&mut rig, &[124, 134, 124, 138, 124, 136]);
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::SetBpmFilter(Some((130, 140))), &ctx);
    let out = rig.frame(Vec::new());
    let drawn: Vec<String> = texts(&out).into_iter().map(|t| t.text).collect();
    let numbered = |n: usize| drawn.iter().any(|t| t.starts_with(&format!("{n}. ")));
    assert!(
        numbered(2) && numbered(4) && numbered(6),
        "crate numbers kept: {drawn:?}"
    );
    assert!(
        !numbered(1) && !numbered(3) && !numbered(5),
        "hidden entries aren't drawn"
    );

    // Row 1 is entry 2; Shift-click on row 3 (entry 6) selects only shown entries.
    rig.click(rig.row(0));
    rig.mods = Modifiers::SHIFT;
    rig.click(rig.row(2));
    rig.mods = Modifiers::NONE;
    assert_eq!(
        rig.app.crates.shown().selected_ids(),
        [ids[1], ids[3], ids[5]]
    );

    // ↓ from entry 2 goes to entry 4.
    rig.click(rig.row(0));
    rig.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.app.crates.shown().cursor(), Some(ids[3]));
    rig.app.apply(Action::SelectAll, &ctx);
    assert_eq!(
        rig.app.crates.shown().selected_ids(),
        [ids[1], ids[3], ids[5]]
    );

    // Export takes the whole crate.
    assert_eq!(files::m3u_entries(rig.app.crates.shown()).len(), 6);

    // P on an entry the filter hides turns the filter off.
    rig.app.crates.shown_mut().set_current(Some(ids[2]));
    rig.frame(Vec::new());
    rig.key(Key::P, Modifiers::NONE);
    assert_eq!(rig.app.crates.shown().bpm_filter(), None);
    assert_eq!(rig.app.crates.shown().cursor(), Some(ids[2]));
}

/// Row `i` under the crate sidebar's CRATES heading (a playlist at least 600 pixels wide).
fn side_row(i: usize) -> Pos2 {
    pos2(
        PL_LEFT + 12.0 + 40.0,
        PL_TOP + LIST_TOP + (i + 1) as f32 * 13.0 + 6.5,
    )
}

/// Row `i` of the list beside the sidebar, under the column header.
fn side_list_row(i: usize) -> Pos2 {
    pos2(
        PL_LEFT + 12.0 + 110.0 + 100.0,
        PL_TOP + LIST_TOP + (i + 1) as f32 * 13.0 + 6.5,
    )
}

/// A wide playlist of three files and two more crates, Keepers and Friday.
fn sidebar_rig(name: &str) -> (Rig, Vec<EntryId>, CrateId, CrateId) {
    let mut rig = Rig::new(name, Vec::new(), |_| {});
    let tracks =
        ["tone.flac", "tone.wav", "tone.ogg"].map(|f| TrackRef::new(fixture(f).to_string_lossy()));
    rig.app.crates.shown_mut().add(tracks);
    let ids = rig.ids(PLAYLIST);
    let keepers = rig.app.crates.create("Keepers").unwrap();
    let friday = rig.app.crates.create("Friday").unwrap();
    rig.screen.x = 700.0;
    rig.frame(Vec::new());
    (rig, ids, keepers, friday)
}

#[test]
fn the_crate_sidebar_shows_crates_when_it_fits() {
    let (mut rig, _, keepers, _) = sidebar_rig("sidebar");
    assert!(rig.app.pl_sidebar(), "on by default at 700 pixels");
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Playlist"), "{:?}", text_list(&out));
    assert!(shows(&out, "Keepers") && shows(&out, "+ New crate"));
    assert!(!shows(&out, "DISCOGS"), "no collection: no Discogs group");

    // A click shows the crate.
    rig.click(side_row(1));
    assert_eq!(rig.app.crates.shown_id(), keepers);
    assert!(shows(&rig.frame(Vec::new()), "•"), "the shown crate's dot");

    // Every crate's menu exports it; the Playlist crate's, beside Clear crate.
    rig.click_with(side_row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    for t in ["Clear crate", "Export M3U…"] {
        assert!(shows(&out, t), "Playlist: {t}: {:?}", text_list(&out));
    }
    rig.key(Key::Escape, Modifiers::NONE);
    rig.click_with(side_row(2), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Export M3U…"), "{:?}", text_list(&out));
    assert_eq!(rig.app.crates.shown_id(), keepers, "the menu shows nothing");
    rig.key(Key::Escape, Modifiers::NONE);

    // Right-click: Rename… opens the name dialog for that crate.
    rig.click_with(side_row(2), PointerButton::Secondary);
    rig.click_text("Rename crate…");
    assert!(rig.app.name_dialog.is_some());
    rig.app.name_dialog = None;
    rig.click(side_row(3));
    assert!(rig.app.name_dialog.is_some(), "+ New crate asks for a name");
    rig.app.name_dialog = None;

    // With the sidebar there, the title bar opens no crate menu; right-click deletes.
    rig.click(rig.title_bar());
    assert!(!rig.menu_open(), "the sidebar does the menu's job");
    let before = rig.app.crates.list().len();
    rig.click_with(side_row(1), PointerButton::Secondary);
    rig.click_text("Delete crate…");
    assert_eq!(
        rig.app.crates.list().len(),
        before - 1,
        "the empty crate is deleted"
    );

    // Narrower than 600 px it hides, and the title bar opens the crate menu again; it
    // comes back when widened: no switch.
    rig.screen.x = 400.0;
    rig.frame(Vec::new());
    assert!(!rig.app.pl_sidebar());
    rig.click(rig.title_bar());
    assert!(rig.menu_open(), "the menu is the way to crates when narrow");
    rig.click(rig.title_bar());
    rig.screen.x = 700.0;
    rig.frame(Vec::new());
    assert!(rig.app.pl_sidebar());
}

#[test]
fn an_old_sidebar_setting_is_ignored() {
    let old = |store: &Store| {
        std::fs::write(
            store.dir().join(SETTINGS_FILE),
            "(scale: 1, show_eq: false, show_waveform: false, crate_sidebar: false, \
             playlist_maximized: true)",
        )
        .unwrap();
    };
    let mut rig = Rig::new("sidebar-old-setting", Vec::new(), old);
    rig.screen.x = 700.0;
    rig.frame(Vec::new());
    assert!(rig.app.pl_sidebar(), "shown when it fits");
}

#[test]
fn control_click_and_the_delete_key_delete_a_sidebar_crate() {
    let (mut rig, ids, keepers, friday) = sidebar_rig("sidebar-delete");
    rig.app.send_to(&ids[..2], friday);

    // Control-click (a Mac's right-click) opens the crate's menu, without showing it.
    rig.mods = Modifiers {
        ctrl: true,
        mac_cmd: false,
        command: false,
        ..Modifiers::NONE
    };
    let mac = cfg!(target_os = "macos");
    rig.click(side_row(1));
    rig.mods = Modifiers::NONE;
    if mac {
        let out = rig.frame(Vec::new());
        assert!(shows(&out, "Delete crate…"), "{:?}", text_list(&out));
        assert_eq!(rig.app.crates.shown_id(), PLAYLIST, "not shown");
        rig.click_text("Delete crate…");
        assert!(
            rig.app.crates.find("Keepers").is_none(),
            "an empty crate goes at once"
        );
    } else {
        rig.app
            .apply(Action::DeleteCrate(keepers), &rig.ctx.clone());
    }

    // A click on a crate, then Delete: it is deleted after confirming (it has entries).
    rig.click(side_row(1));
    assert_eq!(rig.app.crates.shown_id(), friday);
    rig.key(Key::Delete, Modifiers::NONE);
    assert_eq!(rig.app.confirm_delete, Some(friday), "asks first");
    rig.click_text("Delete");
    assert!(rig.app.crates.find("Friday").is_none());

    // After a click in the list, Delete removes entries again.
    rig.click(side_row(0));
    rig.click(side_list_row(0));
    rig.key(Key::Delete, Modifiers::NONE);
    assert_eq!(rig.ids(PLAYLIST).len(), 2, "an entry removed, no crate");

    // The Playlist crate is never deleted from the keyboard.
    rig.click(side_row(0));
    rig.key(Key::Delete, Modifiers::NONE);
    assert!(rig.app.crates.find("Playlist").is_some());
}

#[test]
fn the_sidebar_lists_the_wantlist_above_the_collection_under_discogs() {
    let (mut rig, _, _, _) = sidebar_rig("sidebar-wantlist");
    let coll = rig.app.crates.create("Collection: digger").unwrap();
    rig.app.crates.set_collection(coll);
    let wl = rig.app.crates.create("Wantlist: digger").unwrap();
    let out = rig.frame(Vec::new());
    let y = |out: &egui::FullOutput, name: &str| {
        texts(out)
            .into_iter()
            .find(|t| t.text == name)
            .map(|t| t.rect.center().y)
            .unwrap_or_else(|| panic!("{name} in {:?}", text_list(out)))
    };
    assert!(
        y(&out, "Wantlist: digger") < y(&out, "DISCOGS"),
        "unconnected, one of the user's crates"
    );
    rig.app.crates.set_wantlist(wl, true);
    let out = rig.frame(Vec::new());
    let (heading, w, c) = (
        y(&out, "DISCOGS"),
        y(&out, "Wantlist: digger"),
        y(&out, "Collection: digger"),
    );
    assert!(
        heading < w && w < c,
        "DISCOGS, the wantlist, then the collection"
    );
}

#[test]
fn the_sidebar_pins_the_collection_and_the_playlist_crate_clears() {
    let (mut rig, _, _, _) = sidebar_rig("sidebar-collection");
    let coll = rig.app.crates.create("Collection: digger").unwrap();
    rig.app.crates.set_collection(coll);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "DISCOGS"), "{:?}", text_list(&out));
    // Pinned to the bottom row of the sidebar, under DISCOGS.
    let last = rig.app.pl_rows() - 1;
    let bottom = pos2(
        PL_LEFT + 12.0 + 40.0,
        PL_TOP + LIST_TOP + last as f32 * 13.0 + 6.5,
    );
    rig.click(bottom);
    assert_eq!(rig.app.crates.shown_id(), coll);
    // And still there after the others: + New crate follows the user's crates.
    rig.click(side_row(3));
    assert!(
        rig.app.name_dialog.is_some(),
        "+ New crate right after Friday"
    );
    rig.app.name_dialog = None;

    // Playlist can't be renamed or deleted: its menu clears it instead.
    rig.click_with(side_row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(!shows(&out, "Delete crate…"));
    rig.click_text("Clear crate");
    assert!(rig.entries(PLAYLIST).is_empty());
    assert!(rig.app.crates.find("Playlist").is_some());
}

#[test]
fn a_long_crate_list_scrolls_while_dragging_and_folds() {
    let (mut rig, ids, _, _) = sidebar_rig("sidebar-long");
    let last = (3..40)
        .map(|i| rig.app.crates.create(&format!("Crate {i:02}")).unwrap())
        .last()
        .unwrap();
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "⏷ CRATES (40)") && shows(&out, "+ New crate"));
    assert!(
        !shows(&out, "Crate 39"),
        "past the end: {:?}",
        text_list(&out)
    );

    // Dragging an entry over the list's last row scrolls it, then it drops on Crate 39.
    let edge = texts(&out)
        .into_iter()
        .filter(|t| t.text.starts_with("Crate "))
        .map(|t| t.rect.center())
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap();
    let from = side_list_row(0);
    rig.click(from);
    rig.frame(vec![Event::PointerMoved(from)]);
    rig.press(from, PointerButton::Primary, true);
    for t in [0.3, 0.6, 1.0] {
        rig.frame(vec![Event::PointerMoved(from + (edge - from) * t)]);
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while !shows(&rig.frame(vec![Event::PointerMoved(edge)]), "Crate 39") {
        assert!(Instant::now() < deadline, "the list never reached Crate 39");
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = rig.frame(vec![Event::PointerMoved(edge)]);
    let to = texts(&out)
        .into_iter()
        .find(|t| t.text == "Crate 39")
        .unwrap()
        .rect
        .center();
    rig.frame(vec![Event::PointerMoved(to)]);
    rig.press(to, PointerButton::Primary, false);
    rig.frame(Vec::new());
    assert_eq!(rig.ids(last).len(), 1, "dropped on Crate 39");
    assert_eq!(rig.ids(PLAYLIST), ids, "not reordered");

    // CRATES folds on its own; + New crate stays.
    rig.click_text("⏷ CRATES (40)");
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "⏵ CRATES (40)") && shows(&out, "+ New crate"));
    assert!(!shows(&out, "Keepers"));
    assert!(rig.app.settings.crates_folded);
}

#[test]
fn dragging_entries_onto_a_sidebar_crate_sends_them() {
    let (mut rig, ids, _, friday) = sidebar_rig("sidebar-drop");
    rig.app.send_to(&[ids[1]], friday);
    // Entries 1 to 3 selected; drag entry 1 onto Friday, which holds entry 2 already.
    rig.click(side_list_row(0));
    rig.mods = Modifiers::SHIFT;
    rig.click(side_list_row(2));
    rig.mods = Modifiers::NONE;
    let from = side_list_row(0);
    rig.frame(vec![Event::PointerMoved(from)]);
    rig.press(from, PointerButton::Primary, true);
    for t in [0.3, 0.6, 1.0] {
        let p = from + (side_row(2) - from) * t;
        rig.frame(vec![Event::PointerMoved(p)]);
    }
    rig.press(side_row(2), PointerButton::Primary, false);
    rig.frame(Vec::new());
    assert_eq!(rig.ids(friday).len(), 3, "two added, one already there");
    let msg = rig
        .app
        .message
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert_eq!(msg, "Sent 2 entries to Friday (1 already there)");
    assert_eq!(rig.ids(PLAYLIST), ids, "the list wasn't reordered");

    // Dropped inside the list, an entry is reordered and no crate receives it.
    let from = side_list_row(0);
    rig.frame(vec![Event::PointerMoved(from)]);
    rig.press(from, PointerButton::Primary, true);
    for t in [0.3, 0.6, 1.0] {
        let p = from + (side_list_row(2) - from) * t;
        rig.frame(vec![Event::PointerMoved(p)]);
    }
    rig.press(side_list_row(2), PointerButton::Primary, false);
    rig.frame(Vec::new());
    assert_eq!(rig.ids(PLAYLIST), [ids[1], ids[2], ids[0]]);
    assert_eq!(rig.ids(friday).len(), 3);
}

/// The middle of the filter bar, top to bottom.
const BAR_Y: f32 = PL_TOP + 20.0 + 8.0;

/// The filter bar's ×, at its right end.
fn bar_clear(rig: &Rig) -> Pos2 {
    let w = rig.app.pl_geometry().0 as f32;
    let clear_w = rig.app.def.sprite("bpm_clear").w as f32;
    pos2(PL_LEFT + w - 4.0 - clear_w / 2.0, BAR_Y)
}

/// Just above the list's bottom edge, below the last row of a short crate.
fn below_rows(rig: &Rig) -> Pos2 {
    pos2(
        PL_LEFT + 40.0,
        PL_TOP + LIST_TOP + rig.app.pl_rows() as f32 * 13.0 - 4.0,
    )
}

/// The centre of the Options gear in the window header.
fn corner_gear(rig: &Rig) -> Pos2 {
    header_button(rig, 2.0)
}

#[test]
fn the_playlist_has_no_footer_and_the_list_has_the_crate_menu() {
    let mut rig = Rig::new("footer", Vec::new(), |_| {});
    let ids = rig.fill_playlist(3);
    set_tempos(&mut rig, &[128, 122, 140]);
    rig.frame(Vec::new());
    for name in [
        "pl_add",
        "pl_menu",
        "pl_plus",
        "pl_opts",
        "pl_bottom_l",
        "pl_bottom_r",
    ] {
        assert_eq!(rig.app.def.sprite(name).w, 0, "no {name} any more");
    }
    // Under the last row only the edge (as wide as the left one), then less than a row.
    let edge = rig.app.def.sprite("pl_bottom_edge").h as f32;
    assert_eq!(edge, rig.app.def.sprite("pl_left").w as f32);
    let rows_end = PL_TOP + LIST_TOP + rig.app.pl_rows() as f32 * 13.0 + edge;
    assert!(
        rows_end <= rig.screen.y && rig.screen.y - rows_end < 13.0,
        "{rows_end}"
    );

    // A right-click below the rows opens the crate's menu.
    rig.click_with(below_rows(&rig), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    for t in [
        "Select all",
        "Select none",
        "Invert selection",
        "Remove selected",
        "Clear crate",
        "Sort",
        "Group by record",
        ADD_FILES,
        "Add folder…",
        "Export M3U…",
    ] {
        assert!(shows(&out, t), "{t}: {:?}", text_list(&out));
    }
    assert!(!shows(&out, "Play"), "not a row's menu");
    let before = rig.app.crates.shown().selected_ids().len();
    rig.click_text("Invert selection");
    assert_eq!(rig.app.crates.shown().selected_ids().len(), 3 - before);

    // A row's menu ends with the same items under Crate ▸.
    rig.click_with(rig.row(0), PointerButton::Secondary);
    rig.click_text("Crate");
    let out = rig.frame(Vec::new());
    for t in [ADD_FILES, "Add folder…", "Export M3U…"] {
        assert!(shows(&out, t), "Crate ▸ {t}: {:?}", text_list(&out));
    }
    rig.click_text("Sort");
    rig.click_text("BPM");
    assert_eq!(rig.ids(PLAYLIST), [ids[1], ids[0], ids[2]], "sorted by BPM");
}

#[test]
fn without_digging_there_are_no_options_to_open() {
    let mut rig = Rig::new("options", Vec::new(), |_| {});
    // The title line: no control there, and no menu without digging.
    rig.click_with(pos2(150.0, 15.0), PointerButton::Secondary);
    assert!(!egui::Popup::is_any_open(&rig.ctx), "no menu opens");
    // The gear isn't there to click: its slot is empty and opens nothing.
    rig.screen.x = 1100.0;
    rig.click(corner_gear(&rig));
    rig.frame(Vec::new());
    assert!(!egui::Popup::is_any_open(&rig.ctx), "no menu opens");
    assert!(!rig.app.spectro_open);
}

#[test]
fn cmd_plus_minus_and_zero_set_the_size() {
    let mut rig = Rig::new("size-keys", vec![fixture("tone.flac")], |_| {});
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    assert_eq!(rig.app.settings.scale, 1);
    rig.key(Key::Equals, Modifiers::COMMAND);
    assert_eq!(rig.app.settings.scale, 2, "Cmd+= is double size");
    rig.app.settings_dirty = None;
    rig.key(Key::Plus, Modifiers::COMMAND);
    assert_eq!(rig.app.settings.scale, 2, "never past 2×");
    assert!(
        rig.app.settings_dirty.is_none(),
        "the same size isn't saved again"
    );
    assert_eq!(rig.ctx.zoom_factor(), 1.0, "egui's own zoom stays off");
    rig.key(Key::Minus, Modifiers::COMMAND);
    assert_eq!(rig.app.settings.scale, 1, "Cmd+− is classic size");
    assert!(rig.app.settings_dirty.is_some(), "and it is saved");
    rig.key(Key::Plus, Modifiers::COMMAND);
    rig.key(Key::Num0, Modifiers::COMMAND);
    assert_eq!(rig.app.settings.scale, 1, "Cmd+0 is classic size");
    // Without Cmd they are no size keys.
    rig.key(Key::Equals, Modifiers::NONE);
    assert_eq!(rig.app.settings.scale, 1);
    assert_eq!(
        rig.app.position.state,
        PlayState::Playing,
        "playback goes on"
    );
}

#[test]
fn the_title_bar_counts_what_a_bpm_filter_shows() {
    let def = LoadedSkin::default_skin().def;
    assert_eq!(
        crate_title_with_count(&def, "Lowtide Tapes", Some((2, 4)), 200.0),
        "LOWTIDE TAPES · 2/4"
    );
    assert_eq!(
        crate_title_with_count(&def, "Lowtide Tapes", None, 200.0),
        "LOWTIDE TAPES"
    );
    // Short of room, the name gives way and the count stays.
    let t = crate_title_with_count(&def, "Lowtide Tapes", Some((42, 301)), 100.0);
    assert!(t.ends_with(" · 42/301") && t.starts_with("LOW"), "{t}");
}

#[test]
fn the_bar_bpm_control_sets_the_range_and_the_bar_x_clears_it() {
    let mut rig = Rig::new("bpm-bar", Vec::new(), |_| {});
    let ids = rig.fill_playlist(7);
    rig.frame(Vec::new());
    assert_eq!(rig.app.bpm_slider, None, "no tempo: no control");
    set_tempos(&mut rig, &[124, 128, 137, 139]);
    rig.app.bpm_slider = None;
    rig.frame(Vec::new());
    let (x0, x1, text) = rig.app.bpm_slider.expect("the control is drawn");
    assert!(x1 - x0 == BPM_SLIDER_W && text, "{x0}..{x1}");
    assert_eq!(rig.app.pl_visible_rows(), rig.app.pl_rows(), "no row taken");

    // Drag the right handle to the middle of the track.
    let at = |x: f32| pos2(PL_LEFT + x, BAR_Y);
    rig.frame(vec![Event::PointerMoved(at(x1))]);
    rig.press(at(x1), PointerButton::Primary, true);
    let mid = (x0 + x1) / 2.0;
    for x in [x1 - 5.0, x1 - 15.0, mid] {
        rig.frame(vec![Event::PointerMoved(at(x))]);
    }
    rig.press(at(mid), PointerButton::Primary, false);
    rig.frame(Vec::new());
    let shown = rig.app.crates.shown();
    let (lo, hi) = shown.bpm_filter().expect("a range");
    assert_eq!(lo, 124, "the low handle stays");
    assert!((130..=133).contains(&hi), "the high handle moved to {hi}");
    assert_eq!(shown.shown_rows(), [0, 1]);
    assert_eq!(rig.app.crates.shown().cursor(), None);

    // The bar's × clears it; a double-click on the slider does too.
    rig.click(bar_clear(&rig));
    assert_eq!(rig.app.crates.shown().bpm_filter(), None, "× clears");
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::SetBpmFilter(Some((130, 135))), &ctx);
    rig.frame(Vec::new());
    assert!(rig.app.crates.shown().shown_rows().is_empty());
    // Clicks count by time alone: let the × click above age out first.
    for _ in 0..30 {
        rig.frame(Vec::new());
    }
    rig.double_click(at(mid));
    assert_eq!(
        rig.app.crates.shown().bpm_filter(),
        None,
        "double-click clears"
    );
    assert_eq!(rig.ids(PLAYLIST), ids);
}

#[test]
fn the_bpm_slider_is_short_and_fixed_and_the_search_keeps_its_room() {
    let mut rig = Rig::new("bpm-bar-narrow", Vec::new(), |_| {});
    rig.fill_playlist(3);
    set_tempos(&mut rig, &[124, 128, 139]);
    let p = rig.app.crates.shown_mut();
    for e in p.entries_mut() {
        e.duration = Some(23.0 * 3600.0 + 25.0 * 60.0); // 70 h in all: "0:00/70:15:00"
    }
    rig.frame(Vec::new());
    let (x0, x1, text) = rig.app.bpm_slider.expect("the control is drawn");
    assert_eq!(x1 - x0, BPM_SLIDER_W);
    assert!(text, "the range text fits at the classic width");
    assert!(
        x0 >= 4.0 + SEARCH_MIN_W + 4.0,
        "the search field keeps its room: {x0}"
    );

    // Wider, the slider keeps its width.
    rig.screen.x = 700.0;
    rig.frame(Vec::new());
    let (x0, x1, _) = rig.app.bpm_slider.unwrap();
    assert_eq!(x1 - x0, BPM_SLIDER_W);
}

/// Gives the shown crate's entries a release each (1, 2, …) with these styles.
fn set_styles(rig: &mut Rig, styles: &[&str]) {
    let p = rig.app.crates.shown_mut();
    for (i, (e, st)) in p.entries_mut().zip(styles).enumerate() {
        e.origin = Some(crate::playlist::Origin {
            release: Some(i as u64 + 1),
            clip: Some(format!("c{i}")),
            styles: (*st).into(),
            ..Default::default()
        });
    }
}

#[test]
fn a_locked_grouped_crate_still_sorts_from_its_crate_menu() {
    let mut rig = Rig::new("crate-menu-locked", Vec::new(), |_| {});
    let ids = rig.fill_playlist(3);
    set_tempos(&mut rig, &[128, 122, 140]);
    rig.app.crates.set_collection(PLAYLIST);
    rig.app.apply(Action::ToggleGrouped, &rig.ctx.clone());
    rig.frame(Vec::new());
    assert!(!rig.app.pl_header(), "grouped: no column header to click");

    rig.click_with(below_rows(&rig), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    for t in ["Remove selected", "Clear crate", ADD_FILES, "Add folder…"] {
        assert!(!shows(&out, t), "{t} in a locked crate");
    }
    assert!(shows(&out, "Export M3U…"), "every crate exports");
    rig.click_text("Sort");
    rig.click_text("BPM");
    assert_eq!(rig.ids(PLAYLIST), [ids[1], ids[0], ids[2]], "sorted by BPM");
}

#[test]
fn the_style_filter_shows_chips_in_any_crate_with_records() {
    let mut rig = Rig::new("style-chips", Vec::new(), |_| {});
    rig.fill_playlist(4);
    set_styles(
        &mut rig,
        &["Deep House", "Minimal, Techno", "Electro", "Deep House"],
    );
    rig.screen.x = 700.0;
    // Not a Discogs crate: the control is drawn all the same.
    rig.frame(Vec::new());
    assert!(rig.app.filters_drawn.is_some(), "drawn in any crate");

    as_collection(&mut rig);
    rig.frame(Vec::new());
    let (x, tier) = rig.app.filters_drawn.expect("the control is drawn");
    assert_eq!(
        tier,
        FooterFilters::Chips,
        "4 short styles fit at 700 pixels"
    );
    assert_eq!(rig.app.pl_visible_rows(), rig.app.pl_rows(), "no row taken");
    // The first chip is the style of most records: DEEP HOUSE.
    let chip = pos2(PL_LEFT + x + 4.0, BAR_Y);
    rig.click(chip);
    let shown = rig.app.crates.shown();
    assert!(shown.picked(Facet::Style, "Deep House"));
    assert_eq!(shown.shown_rows().len(), 2, "the two Deep House records");
    for _ in 0..30 {
        rig.frame(Vec::new());
    }
    rig.double_click(chip);
    assert_eq!(
        rig.app.crates.shown().filter(Facet::Style),
        None,
        "double-click clears"
    );
    assert_eq!(rig.app.crates.shown().shown_rows().len(), 4);
}

#[test]
fn many_styles_or_a_narrow_playlist_show_the_styles_button_and_its_list() {
    let mut rig = Rig::new("style-list", Vec::new(), |_| {});
    rig.fill_playlist(4);
    set_styles(
        &mut rig,
        &["Deep House", "Minimal, Techno", "Electro", "Tech House"],
    );
    as_collection(&mut rig);
    rig.frame(Vec::new());
    let (x, tier) = rig.app.filters_drawn.expect("the control is drawn");
    assert_eq!(
        tier,
        FooterFilters::Buttons,
        "the chips don't fit at the classic width"
    );
    rig.click(pos2(PL_LEFT + x + 4.0, BAR_Y));
    let out = rig.frame(Vec::new());
    assert!(
        shows(&out, "Minimal") && shows(&out, "Electro"),
        "{:?}",
        text_list(&out)
    );
    // The list's own "Electro" (record rows name their styles too): the last one drawn.
    let item = texts(&out)
        .into_iter()
        .rev()
        .find(|t| t.text == "Electro")
        .unwrap()
        .rect;
    rig.click(item.center());
    assert!(rig.app.crates.shown().picked(Facet::Style, "Electro"));
    assert_eq!(rig.app.crates.shown().shown_rows().len(), 1);

    // The search narrows the list.
    rig.click_text("Filter styles…");
    rig.frame(vec![Event::Text("house".into())]);
    let out = rig.frame(Vec::new());
    let listed = |t: &str| texts(&out).iter().filter(|x| x.text == t).count();
    // Only Electro's record row shows, so the House styles are drawn by the list alone.
    assert_eq!((listed("Deep House"), listed("Tech House")), (1, 1));
    assert_eq!(
        listed("Electro"),
        1,
        "only its record row: {:?}",
        text_list(&out)
    );
    rig.click_text("Clear");
    assert_eq!(rig.app.crates.shown().filter(Facet::Style), None);
}

/// Gives the shown crate's entries a release each (1, 2, …) with these styles, record
/// artists and labels.
fn set_records(rig: &mut Rig, records: &[(&str, &str, &str)]) {
    let p = rig.app.crates.shown_mut();
    for (i, (e, (st, artist, label))) in p.entries_mut().zip(records).enumerate() {
        e.origin = Some(crate::playlist::Origin {
            release: Some(i as u64 + 1),
            clip: Some(format!("c{i}")),
            styles: (*st).into(),
            artist: (*artist).into(),
            label: (*label).into(),
            ..Default::default()
        });
    }
}

/// Shows crate `c` flat, for tests of a track's own row (crates open grouped by record).
fn flat(rig: &mut Rig, c: CrateId) {
    rig.app.crates.set_grouped(c, false);
}

/// Makes the rig's Playlist the collection crate, grouped by record as it opens (the rig
/// starts it flat).
fn as_collection(rig: &mut Rig) {
    rig.app.crates.set_collection(PLAYLIST);
    rig.app.crates.set_grouped(PLAYLIST, true);
}

/// A collection crate of six records with 3 styles, 4 artists and 3 labels.
fn credited_rig(name: &str) -> Rig {
    let mut rig = Rig::new(name, Vec::new(), |_| {});
    rig.fill_playlist(6);
    set_records(
        &mut rig,
        &[
            ("Deep House", "Theo Parrish", "Sound Signature"),
            (
                "Deep House",
                "Theo Parrish & Marcellus Pittman",
                "Unirhythm",
            ),
            ("Minimal", "Nightcraft", "Lowtide Tapes"),
            ("Minimal", "Nightcraft", "Lowtide Tapes"),
            ("Techno", "Various", "Lowtide Tapes"),
            ("Techno", "Theo Parrish", "Sound Signature"),
        ],
    );
    as_collection(&mut rig);
    rig
}

#[test]
fn the_bar_shows_chips_then_buttons_then_filters_as_it_narrows() {
    let mut rig = credited_rig("filter-tiers");
    rig.screen.x = 700.0;
    rig.frame(Vec::new());
    let (x, tier) = rig.app.filters_drawn.unwrap();
    assert_eq!(tier, FooterFilters::Chips);
    // The search field takes at most half the bar; the time readout, then the filters
    // follow it.
    let (_, rx, rw) = rig
        .app
        .readout_drawn
        .clone()
        .expect("the readout fits at 700");
    assert!(rx <= 4.0 + 350.0 + 4.0, "{rx}");
    assert!(x <= rx + rw + 4.0 + STYLE_GAP, "{x}");
    rig.screen.x = 300.0;
    rig.frame(Vec::new());
    let (x, tier) = rig.app.filters_drawn.unwrap();
    assert_eq!(tier, FooterFilters::Buttons);
    assert!(x >= 4.0 + SEARCH_MIN_W, "after the search field: {x}");
    // Classic width with tempos: one FILTERS button after the BPM control.
    rig.screen.x = 275.0;
    set_tempos(&mut rig, &[124, 128, 132, 136, 138, 140]);
    rig.frame(Vec::new());
    assert_eq!(rig.app.filters_drawn.unwrap().1, FooterFilters::Folded);
    assert!(rig.app.bpm_slider.is_some());
}

#[test]
fn the_filters_button_opens_the_panel_at_classic_width() {
    let mut rig = credited_rig("filter-panel");
    set_tempos(&mut rig, &[124, 128, 132, 136, 138, 140]);
    rig.frame(Vec::new());
    let (x, tier) = rig.app.filters_drawn.unwrap();
    assert_eq!(tier, FooterFilters::Folded);
    rig.click(pos2(PL_LEFT + x + 4.0, BAR_Y));
    let out = rig.frame(Vec::new());
    for t in ["STYLE", "ARTIST", "LABEL"] {
        assert!(shows(&out, t), "{t}: {:?}", text_list(&out));
    }
    // The ARTIST tab: search, then pick.
    rig.click_text("ARTIST");
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Nightcraft"), "{:?}", text_list(&out));
    rig.click_text("Filter artists…");
    rig.frame(vec![Event::Text("parrish".into())]);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Theo Parrish") && shows(&out, "Theo Parrish & Marcellus Pittman"));
    assert!(
        !text_list(&out).iter().any(|t| t == "Nightcraft"),
        "{:?}",
        text_list(&out)
    );
    rig.click_text("Theo Parrish");
    assert_eq!(
        rig.app.crates.shown().shown_rows().len(),
        2,
        "not the joint record"
    );
    // The LABEL tab keeps the artist picked.
    rig.click_text("LABEL");
    rig.click_text("Sound Signature");
    assert!(rig.app.crates.shown().picked(Facet::Artist, "Theo Parrish"));
    assert!(
        rig.app
            .crates
            .shown()
            .picked(Facet::Label, "Sound Signature")
    );
    // The bar's × turns them all off, the BPM range too.
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::SetBpmFilter(Some((130, 140))), &ctx);
    egui::Popup::close_all(&ctx);
    rig.frame(Vec::new());
    rig.click(bar_clear(&rig));
    let p = rig.app.crates.shown();
    assert!(!p.is_filtered(), "everything off");
}

#[test]
fn a_lit_button_counts_its_picks_and_a_double_click_clears_them() {
    let mut rig = credited_rig("filter-button");
    rig.screen.x = 700.0;
    let ctx = rig.ctx.clone();
    rig.app.apply(
        Action::TogglePick(Facet::Label, "Lowtide Tapes".into()),
        &ctx,
    );
    rig.app
        .apply(Action::TogglePick(Facet::Label, "Unirhythm".into()), &ctx);
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.shown().shown_rows().len(), 4);
    // Chips (DEEP HOUSE MINIMAL TECHNO), then ARTISTS, then "LABELS 2".
    let (x, _) = rig.app.filters_drawn.unwrap();
    let advance = rig.app.def.font.advance as f32;
    let before = "DEEP HOUSE MINIMAL TECHNO ARTISTS ".len() as f32 * advance - 4.0 * advance
        + 4.0 * STYLE_GAP;
    let labels = pos2(PL_LEFT + x + before + 6.0, BAR_Y);
    rig.double_click(labels);
    assert_eq!(rig.app.crates.shown().filter(Facet::Label), None);
    assert_eq!(rig.app.crates.shown().shown_rows().len(), 6);
}

#[test]
fn styles_that_arrive_later_join_the_list_and_the_counts() {
    // A seller crate: the last record's details haven't come yet, so it has no style.
    let mut rig = Rig::new("late-styles", Vec::new(), |_| {});
    rig.fill_playlist(4);
    set_styles(&mut rig, &["Electro", "Electro", "Techno", ""]);
    rig.frame(Vec::new());
    let ctx = rig.ctx.clone();
    rig.app
        .apply(Action::OnlyPick(Facet::Style, "Electro".into()), &ctx);
    assert_eq!(rig.app.crates.shown().shown_rows(), [0, 1]);

    // Its details arrive: it shows, and Electro counts it.
    let last = rig.app.crates.shown_mut().entries_mut().last().unwrap();
    last.origin.as_mut().unwrap().styles = "Electro".into();
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.shown().shown_rows(), [0, 1, 3]);
    let counts = rig.app.facet_counts(Facet::Style);
    assert_eq!(counts.first(), Some(&("Electro".to_owned(), 3)));
}

#[test]
fn the_year_grid_picks_years_and_shift_picks_a_run() {
    let mut rig = Rig::new("year-grid", Vec::new(), |_| {});
    rig.fill_playlist(5);
    set_records(&mut rig, &[("Electro", "A", "Analogical Force"); 5]);
    for (e, y) in rig
        .app
        .crates
        .shown_mut()
        .entries_mut()
        .zip([1991, 1994, 1994, 1997, 2003])
    {
        e.origin.as_mut().unwrap().year = Some(y);
    }
    rig.screen.x = 400.0;
    rig.frame(Vec::new());
    assert_eq!(rig.app.offered_facets(), [Facet::Year]);
    // YEARS opens the panel on its grid: a cell per year, with its records ("·": none).
    let (x, _) = rig.app.filters_drawn.unwrap();
    rig.click(pos2(PL_LEFT + x + 4.0, BAR_Y));
    let out = rig.frame(Vec::new());
    assert!(
        shows(&out, "1994\n2") && shows(&out, "1992\n·"),
        "{:?}",
        text_list(&out)
    );
    rig.click_text("1991\n1");
    assert_eq!(rig.app.crates.shown().shown_rows(), [0]);
    // Shift-click 1997: 1991 to 1997 (the years with records in it).
    let cell = texts(&rig.frame(Vec::new()))
        .into_iter()
        .find(|t| t.text == "1997\n1")
        .unwrap()
        .rect;
    rig.click_with_mods(cell.center(), Modifiers::SHIFT);
    assert_eq!(rig.app.crates.shown().shown_rows(), [0, 1, 2, 3]);
    assert!(rig.app.crates.shown().picked(Facet::Year, "1994"));
    // The bar's × clears it.
    rig.click(bar_clear(&rig));
    assert_eq!(rig.app.crates.shown().filter(Facet::Year), None);
}

#[test]
fn a_label_crate_offers_every_filter_but_label() {
    let mut rig = Rig::new("label-facets", Vec::new(), |_| {});
    rig.fill_playlist(4);
    set_records(
        &mut rig,
        &[
            ("Electro", "A", "Analogical Force"),
            ("Electro", "B", "Analogical Force"),
            ("Techno", "C", "Analogical Force"),
            ("Techno", "D", "Analogical Force"),
        ],
    );
    rig.frame(Vec::new());
    assert_eq!(rig.app.offered_facets(), [Facet::Style, Facet::Artist]);
}

#[test]
fn a_crate_of_one_style_artist_and_label_offers_only_the_format_filter() {
    let mut rig = Rig::new("format-filter", Vec::new(), |_| {});
    rig.fill_playlist(4);
    set_records(&mut rig, &[("Electro", "A", "Analogical Force"); 4]);
    for (e, f) in
        rig.app
            .crates
            .shown_mut()
            .entries_mut()
            .zip(["Vinyl", "File", "Vinyl, CD", "File"])
    {
        e.origin.as_mut().unwrap().formats = f.into();
    }
    rig.screen.x = 400.0;
    rig.frame(Vec::new());
    assert_eq!(rig.app.offered_facets(), [Facet::Format]);
    // No styles here: just the FORMATS button, opening the panel's only tab.
    let (x, tier) = rig.app.filters_drawn.unwrap();
    assert_eq!(tier, FooterFilters::Buttons);
    rig.click(pos2(PL_LEFT + x + 4.0, BAR_Y));
    let out = rig.frame(Vec::new());
    assert!(
        shows(&out, "FORMAT") && !shows(&out, "STYLE"),
        "{:?}",
        text_list(&out)
    );
    assert!(
        shows(&out, "Vinyl") && shows(&out, "File"),
        "{:?}",
        text_list(&out)
    );
    rig.click_text("Vinyl");
    assert_eq!(
        rig.app.crates.shown().shown_rows(),
        [0, 2],
        "vinyl records only"
    );
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::ClearPicks(None), &ctx);
    assert!(!rig.app.crates.shown().is_filtered());
}

#[test]
fn a_row_menu_keeps_only_its_label_or_searches_it() {
    // The collection crate: Only this label picks it, and LABELS counts it.
    let mut rig = credited_rig("row-only");
    rig.frame(Vec::new());
    rig.click_with(rig.row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Only this artist"), "{:?}", text_list(&out));
    rig.click_text("Only this label");
    let p = rig.app.crates.shown();
    let picked: Vec<&String> = p.filter(Facet::Label).unwrap().iter().collect();
    assert_eq!(picked, ["Sound Signature"]);
    assert_eq!(p.shown_rows().len(), 2);

    // Any crate with two labels, not only a Discogs one: Only this label.
    let mut rig = Rig::new("row-seller", Vec::new(), |_| {});
    rig.fill_playlist(3);
    set_records(
        &mut rig,
        &[
            ("Techno", "A", "Lowtide Tapes"),
            ("Techno", "B", "Other"),
            ("Techno", "C", "Lowtide Tapes"),
        ],
    );
    rig.frame(Vec::new());
    rig.click_with(rig.row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(
        !shows(&out, "Search Lowtide Tapes"),
        "{:?}",
        text_list(&out)
    );
    rig.click_text("Only this label");
    assert_eq!(rig.app.crates.shown().shown_rows(), [0, 2]);

    // A label crate offers no label filter: Search ‹catno› puts it in the search field,
    // which takes the keyboard.
    let mut rig = Rig::new("row-search", Vec::new(), |_| {});
    rig.fill_playlist(3);
    set_records(&mut rig, &[("Techno", "A", "Lowtide Tapes"); 3]);
    rig.app
        .crates
        .shown_mut()
        .entries_mut()
        .next()
        .unwrap()
        .origin
        .as_mut()
        .unwrap()
        .catno = "AF014".into();
    rig.frame(Vec::new());
    rig.click_with(rig.row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(!shows(&out, "Only this label"), "not offered here");
    rig.click_text("Search AF014");
    rig.frame(Vec::new());
    assert_eq!(rig.app.pl_search, "AF014");
    assert_eq!(rig.app.crates.shown().shown_rows(), [0]);
    assert!(rig.ctx.memory(|m| m.has_focus(Id::new("pl_search"))));
}

/// The middle of the filter bar's search field.
fn search_at() -> Pos2 {
    pos2(PL_LEFT + 40.0, PL_TOP + 20.0 + 8.0)
}

/// Typing `text` as a keyboard does: each letter's key press and its text.
fn type_keys(rig: &mut Rig, text: &str) {
    for c in text.chars() {
        let key = Key::from_name(&c.to_uppercase().to_string()).expect("a letter key");
        rig.frame(vec![
            Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
            Event::Text(c.to_string()),
        ]);
    }
}

/// Three files playing, credited to A, B and C, the first and last on Lowtide Tapes.
fn search_rig(name: &str) -> (Rig, Vec<EntryId>) {
    let mut rig = Rig::new(
        name,
        vec![
            fixture("tone.flac"),
            fixture("tone.wav"),
            fixture("tone.ogg"),
        ],
        |_| {},
    );
    rig.until(
        |r| r.app.position.state == PlayState::Playing,
        "the first file plays",
    );
    let ids = rig.ids(PLAYLIST);
    set_records(
        &mut rig,
        &[
            ("Techno", "A", "Lowtide Tapes"),
            ("Techno", "B", "Other"),
            ("Techno", "C", "Lowtide Tapes"),
        ],
    );
    (rig, ids)
}

#[test]
fn typing_in_the_search_bar_never_triggers_the_letter_shortcuts() {
    let (mut rig, _) = search_rig("search-keys");
    rig.click(search_at());
    type_keys(&mut rig, "xcvfnih");
    assert_eq!(rig.app.pl_search, "xcvfnih");
    assert_eq!(
        rig.app.position.state,
        PlayState::Playing,
        "not paused or stopped"
    );
    assert!(!rig.app.help, "no help");
    assert!(
        rig.app.crates.shown().shown_rows().is_empty(),
        "nothing matches"
    );
}

#[test]
fn search_label_clears_a_bpm_range_set_before_it() {
    let (mut rig, _) = search_rig("search-clears");
    for (e, b) in rig.app.crates.shown_mut().entries_mut().zip([80, 120, 150]) {
        e.bpm = Some(b);
    }
    rig.app.crates.shown_mut().set_bpm_filter(Some((92, 171)));
    assert!(rig.app.crates.shown().bpm_filter().is_some());
    rig.app
        .crates
        .shown_mut()
        .set_pick(Facet::Artist, "C", true);
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::SearchFor("lowtide".into()), &ctx);
    let p = rig.app.crates.shown();
    assert_eq!(p.bpm_filter(), None);
    assert!(!p.picked(Facet::Artist, "C"));
    assert_eq!(p.shown_rows(), [0, 2], "the whole crate is searched");
}

#[test]
fn a_selected_search_is_drawn_once_in_the_skin() {
    let (mut rig, _) = search_rig("search-select");
    rig.click(search_at());
    type_keys(&mut rig, "vigne");
    let out = rig.key(Key::A, Modifiers::COMMAND);
    // egui's own glyphs stay invisible, selected or not.
    fn visible_glyphs(shape: &egui::Shape) -> bool {
        match shape {
            egui::Shape::Text(t) => t
                .galley
                .rows
                .iter()
                .any(|r| r.row.visuals.mesh.vertices.iter().any(|v| v.color.a() > 0)),
            egui::Shape::Vec(v) => v.iter().any(visible_glyphs),
            _ => false,
        }
    }
    assert!(!out.shapes.iter().any(|s| visible_glyphs(&s.shape)));
    let lcd = color(rig.app.def.colors.lcd);
    let field = Rect::from_center_size(search_at(), vec2(60.0, 16.0));
    let blocks = |out: &egui::FullOutput| {
        rects(out)
            .into_iter()
            .filter(|(r, c)| *c == lcd && r.width() > 2.0 && field.intersects(*r))
            .count()
    };
    assert_eq!(blocks(&out), 1, "the selection is an LCD block");
    type_keys(&mut rig, "a");
    let out = rig.frame(Vec::new());
    assert_eq!(rig.app.pl_search, "a");
    assert_eq!(blocks(&out), 0, "nothing selected after typing over it");
}

#[test]
fn the_search_filters_as_typed_and_its_keys_work() {
    let (mut rig, ids) = search_rig("search-live");
    // Cmd+F gives the field the keyboard.
    rig.key(Key::F, Modifiers::COMMAND);
    rig.frame(Vec::new());
    assert!(rig.ctx.memory(|m| m.has_focus(Id::new("pl_search"))));
    type_keys(&mut rig, "lowtide");
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.shown().shown_rows(), [0, 2]);
    assert_eq!(rig.app.queue, [ids[0], ids[2]], "next follows the search");
    // Esc clears only the search; the label filter stays.
    let ctx = rig.ctx.clone();
    rig.app
        .apply(Action::TogglePick(Facet::Artist, "C".into()), &ctx);
    rig.key(Key::Escape, Modifiers::NONE);
    rig.frame(Vec::new());
    assert!(rig.app.pl_search.is_empty());
    assert!(rig.app.crates.shown().search().is_empty());
    assert_eq!(
        rig.app.crates.shown().shown_rows(),
        [2],
        "the artist filter stays"
    );
    rig.app.apply(Action::ClearPicks(None), &ctx);
    // Enter plays the first shown entry.
    rig.click(search_at());
    type_keys(&mut rig, "other");
    rig.key(Key::Enter, Modifiers::NONE);
    rig.until(
        |r| r.app.crates.shown().current() == Some(ids[1]),
        "the first shown entry plays",
    );
    // P on the hidden playing entry clears the search and the filters.
    rig.click(search_at());
    type_keys(&mut rig, " zzz");
    rig.key(Key::Escape, Modifiers::NONE);
    rig.click(search_at());
    type_keys(&mut rig, "lowtide");
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.shown().shown_rows(), [0, 2]);
    rig.app.show_playing_entry();
    rig.frame(Vec::new());
    assert!(!rig.app.crates.shown().is_filtered());
    assert!(rig.app.pl_search.is_empty(), "the field follows");
}

#[test]
fn the_rows_light_what_the_search_matches() {
    let (mut rig, _) = search_rig("search-light");
    rig.app
        .crates
        .shown_mut()
        .entries_mut()
        .nth(1)
        .unwrap()
        .title = "Night Moves".into();
    rig.click(search_at());
    type_keys(&mut rig, "moves");
    let out = rig.frame(Vec::new());
    let hl = color(rig.app.def.colors.pl_current);
    // The row's text in runs: the matched one in the current-entry colour.
    let mut lit = Vec::new();
    fn walk(shape: &egui::Shape, hl: Color32, lit: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) if t.galley.job.sections.len() > 1 => {
                let job = &t.galley.job;
                for sec in job.sections.iter().filter(|s| s.format.color == hl) {
                    lit.push(job.text[sec.byte_range.start.0..sec.byte_range.end.0].to_owned());
                }
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, hl, lit)),
            _ => {}
        }
    }
    for s in &out.shapes {
        walk(&s.shape, hl, &mut lit);
    }
    assert_eq!(lit, ["Moves"], "{:?}", text_list(&out));
}

#[test]
fn showing_another_crate_clears_the_search() {
    let (mut rig, _) = search_rig("search-switch");
    rig.click(search_at());
    type_keys(&mut rig, "lowtide");
    rig.frame(Vec::new());
    assert!(rig.app.crates.shown().is_filtered());
    let other = rig.app.crates.create("Keepers").unwrap();
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::ShowCrate(other), &ctx);
    rig.frame(Vec::new());
    assert!(rig.app.pl_search.is_empty());
    rig.app.apply(Action::ShowCrate(PLAYLIST), &ctx);
    rig.frame(Vec::new());
    assert!(
        !rig.app.crates.shown().is_filtered(),
        "not narrowed when shown again"
    );
}

#[test]
fn next_and_p_follow_the_label_filter() {
    let mut rig = Rig::new(
        "label-next",
        vec![
            fixture("tone.flac"),
            fixture("tone.wav"),
            fixture("tone.ogg"),
        ],
        |_| {},
    );
    rig.until(
        |r| r.app.position.state == PlayState::Playing,
        "the first file plays",
    );
    let ids = rig.ids(PLAYLIST);
    set_records(
        &mut rig,
        &[
            ("Techno", "A", "Lowtide Tapes"),
            ("Techno", "B", "Other"),
            ("Techno", "C", "Lowtide Tapes"),
        ],
    );
    let ctx = rig.ctx.clone();
    rig.app.apply(
        Action::TogglePick(Facet::Label, "Lowtide Tapes".into()),
        &ctx,
    );
    rig.frame(Vec::new());
    assert_eq!(
        rig.app.queue,
        [ids[0], ids[2]],
        "the other label is left out"
    );
    // Only "Other": the playing entry is hidden, and P turns every filter off.
    rig.app.apply(Action::ClearPicks(None), &ctx);
    rig.app
        .apply(Action::TogglePick(Facet::Label, "Other".into()), &ctx);
    rig.app
        .apply(Action::TogglePick(Facet::Artist, "B".into()), &ctx);
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.shown().shown_rows(), [1]);
    rig.app.show_playing_entry();
    assert!(!rig.app.crates.shown().is_filtered());
}

#[test]
fn next_follows_the_style_filter_and_p_clears_it() {
    let mut rig = Rig::new(
        "style-next",
        vec![
            fixture("tone.flac"),
            fixture("tone.wav"),
            fixture("tone.ogg"),
        ],
        |_| {},
    );
    rig.until(
        |r| r.app.position.state == PlayState::Playing,
        "the first file plays",
    );
    let ids = rig.ids(PLAYLIST);
    set_styles(&mut rig, &["Deep House", "Electro", "Deep House"]);
    let ctx = rig.ctx.clone();
    rig.app
        .apply(Action::TogglePick(Facet::Style, "Deep House".into()), &ctx);
    rig.frame(Vec::new());
    assert_eq!(rig.app.queue, [ids[0], ids[2]], "Electro is left out");
    // Electro only: the playing entry is hidden, and P shows it again.
    rig.app.apply(Action::ClearPicks(None), &ctx);
    rig.app
        .apply(Action::TogglePick(Facet::Style, "Electro".into()), &ctx);
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.shown().shown_rows(), [1]);
    rig.app.show_playing_entry();
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.shown().filter(Facet::Style), None);
}

#[test]
fn next_and_a_hidden_playing_track_follow_the_bpm_filter() {
    let mut rig = Rig::new(
        "bpm-next",
        vec![
            fixture("tone.flac"),
            fixture("tone.wav"),
            fixture("tone.ogg"),
        ],
        |_| {},
    );
    rig.until(
        |r| r.app.position.state == PlayState::Playing,
        "the first file plays",
    );
    let ids = rig.ids(PLAYLIST);
    set_tempos(&mut rig, &[134, 124, 138]);
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::SetBpmFilter(Some((130, 140))), &ctx);
    rig.frame(Vec::new());
    assert_eq!(rig.app.queue, [ids[0], ids[2]], "124 BPM is left out");
    rig.app.apply(Action::Next, &ctx);
    rig.until(
        |r| r.app.crates.playing().current() == Some(ids[2]),
        "next skips the hidden entry",
    );

    // A range that hides the playing entry: it plays on, and nothing else is left.
    rig.app.apply(Action::SetBpmFilter(Some((130, 135))), &ctx);
    rig.frame(Vec::new());
    assert_eq!(rig.app.position.state, PlayState::Playing);
    assert_eq!(rig.app.crates.playing().current(), Some(ids[2]));
    assert_eq!(
        rig.app.queue,
        [ids[0], ids[2]],
        "the playing entry stays queued"
    );
    // ALL: everything plays again.
    rig.app.apply(Action::SetBpmFilter(None), &ctx);
    rig.frame(Vec::new());
    assert_eq!(rig.app.queue, ids);
}

/// Entries 1 to 6 from releases 1, 1, 2, 1, (none) and 3; returns their ids.
fn album_crate(rig: &mut Rig) -> Vec<EntryId> {
    let p = rig.app.crates.shown_mut();
    for (i, release) in [Some(1), Some(1), Some(2), Some(1), None, Some(3)]
        .into_iter()
        .enumerate()
    {
        let origin = Origin {
            release,
            album: release.map(|r| format!("Album {r}")).unwrap_or_default(),
            clip: Some(format!("clip{i:07}")),
            ..Default::default()
        };
        p.add_waiting(
            "Nightcraft",
            format!("Track {i}"),
            None,
            Some(origin),
            "queued",
        );
    }
    let ids = p.entries().iter().map(|e| e.id).collect();
    rig.frame(Vec::new());
    ids
}

#[test]
fn right_clicking_an_entry_tints_the_rest_of_its_album() {
    let mut rig = Rig::new("menu-album-tint", Vec::new(), |_| {});
    let ids = album_crate(&mut rig);
    // Entries 3 to 5 selected; right-click entry 2 (outside the selection).
    rig.click(rig.row(2));
    rig.mods = Modifiers::SHIFT;
    rig.click(rig.row(4));
    rig.mods = Modifiers::NONE;
    rig.click_with(rig.row(1), PointerButton::Secondary);
    rig.frame(Vec::new());
    assert_eq!(rig.app.pl_tint, [ids[0], ids[3]]);
    assert_eq!(
        rig.app.crates.shown().selected_ids(),
        [ids[1]],
        "the selection is the clicked entry, as without albums"
    );
    // Closing the menu clears the tint.
    rig.escape();
    rig.frame(Vec::new());
    assert!(rig.app.pl_tint.is_empty());
    // An entry without an album tints nothing.
    rig.click_with(rig.row(4), PointerButton::Secondary);
    rig.frame(Vec::new());
    assert!(rig.app.pl_tint.is_empty());
}

#[test]
fn the_entry_menu_removes_or_selects_a_whole_album() {
    let mut rig = Rig::new("menu-album", Vec::new(), |_| {});
    let ids = album_crate(&mut rig);
    // Select album: exactly the album, the cursor on the clicked entry.
    rig.click_with(rig.row(3), PointerButton::Secondary);
    rig.click_text("Select album");
    let shown = rig.app.crates.shown();
    assert_eq!(shown.selected_ids(), [ids[0], ids[1], ids[3]]);
    assert_eq!(shown.cursor(), Some(ids[3]));

    // A single's items are there but disabled; an entry without an album has none.
    rig.click_with(rig.row(2), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    let on_screen: Vec<String> = texts(&out).into_iter().map(|t| t.text).collect();
    assert!(on_screen.iter().any(|t| t == "Remove album (1 track)"));
    rig.click_text("Remove album (1 track)");
    assert_eq!(rig.ids(PLAYLIST).len(), 6, "disabled: nothing removed");
    rig.escape();
    rig.click_with(rig.row(4), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(
        !texts(&out).iter().any(|t| t.text == "Select album"),
        "no album, no album items"
    );
    rig.escape();

    // Remove album: all of it, wherever it is; the rest keep their order.
    rig.click_with(rig.row(1), PointerButton::Secondary);
    rig.click_text("Remove album (3 tracks)");
    assert_eq!(rig.ids(PLAYLIST), [ids[2], ids[4], ids[5]]);
}

#[test]
fn removing_the_playing_album_lets_its_track_finish() {
    let mut rig = Rig::new(
        "album-playing",
        vec![
            fixture("tone.flac"),
            fixture("tone.wav"),
            fixture("tone.ogg"),
        ],
        |_| {},
    );
    rig.until(
        |r| r.app.position.state == PlayState::Playing,
        "the first file plays",
    );
    let ids = rig.ids(PLAYLIST);
    for &id in &ids[..2] {
        let e = rig
            .app
            .crates
            .shown_mut()
            .entries_mut()
            .find(|e| e.id == id)
            .unwrap();
        (e.artist, e.album) = ("Tone".into(), "Tests".into());
    }
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::RemoveAlbum(ids[1]), &ctx);
    assert_eq!(rig.ids(PLAYLIST), [ids[2]]);
    rig.frame(Vec::new());
    assert_eq!(
        rig.app.position.state,
        PlayState::Playing,
        "the removed track plays on"
    );
    rig.app.apply(Action::Next, &ctx);
    rig.until(
        |r| r.app.crates.playing().current() == Some(ids[2]),
        "next is the next remaining entry",
    );
}

#[test]
fn remove_in_the_entry_menu_takes_the_selection_or_just_the_clicked_entry() {
    let mut rig = Rig::new("menu-remove", Vec::new(), |_| {});
    let ids = rig.fill_playlist(12);
    rig.frame(Vec::new());
    // Entries 3 to 6 selected; right-click entry 4: all four go.
    rig.click(rig.row(2));
    rig.mods = Modifiers::SHIFT;
    rig.click(rig.row(5));
    rig.mods = Modifiers::NONE;
    rig.click_with(rig.row(3), PointerButton::Secondary);
    rig.click_text("Remove");
    let left = |r: &Rig| -> Vec<EntryId> {
        r.app
            .crates
            .shown()
            .entries()
            .iter()
            .map(|e| e.id)
            .collect()
    };
    let mut expected: Vec<EntryId> = ids.clone();
    expected.drain(2..6);
    assert_eq!(left(&rig), expected);

    // Entries 1 and 2 selected; right-click entry 6 (outside): only it goes, and it was
    // selected first.
    rig.click(rig.row(0));
    rig.mods = Modifiers::SHIFT;
    rig.click(rig.row(1));
    rig.mods = Modifiers::NONE;
    rig.click_with(rig.row(5), PointerButton::Secondary);
    assert_eq!(rig.app.crates.shown().selected_ids(), [expected[5]]);
    rig.click_text("Remove");
    expected.remove(5);
    assert_eq!(left(&rig), expected);
}

#[test]
fn arm_in_the_entry_menu_arms_a_waiting_entry() {
    let mut rig = Rig::new("menu-arm", Vec::new(), |_| {});
    let waiting = rig.app.crates.shown_mut().add_waiting(
        "Nightcraft",
        "Glasshouse",
        None,
        None,
        "downloading 40%",
    );
    rig.frame(Vec::new());
    rig.click_with(rig.row(0), PointerButton::Secondary);
    rig.click_text("Arm");
    assert_eq!(rig.app.armed, Some((PLAYLIST, waiting)));
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
        .entries(keepers)
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
    assert_eq!(rig.entries(gig).len(), 3);
    assert_eq!(
        rig.app.crates.shown_id(),
        PLAYLIST,
        "the source stays shown"
    );
}

#[test]
fn the_time_readout_follows_the_search_field_and_counts_the_whole_crate() {
    let mut rig = Rig::new("readout", Vec::new(), |_| {});
    let ids = rig.fill_playlist(3);
    // 7:30 + 1:00:00 + 1:06:35 = 2:14:05.
    for (e, (title, secs)) in rig.app.crates.shown_mut().entries_mut().zip([
        ("Parrish", 450.0),
        ("Other", 3600.0),
        ("Third", 3995.0),
    ]) {
        e.title = title.into();
        e.duration = Some(secs);
    }
    rig.app.crates.shown_mut().select_only(&[ids[0]], ids[0]);
    rig.screen.x = 700.0;
    rig.frame(Vec::new());
    let (text, x, _) = rig.app.readout_drawn.clone().expect("drawn at 700");
    assert_eq!(text, "7:30/2:14:05");
    assert!(
        x >= 4.0 + SEARCH_MIN_W + 4.0,
        "right after the search field: {x}"
    );
    // A search narrows the list, not the readout: it counts the whole crate.
    rig.app
        .apply(Action::SetSearch("parrish".into()), &rig.ctx.clone());
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.shown().shown_rows().len(), 1);
    assert_eq!(rig.app.readout_drawn.clone().unwrap().0, "7:30/2:14:05");
    // A duration not known yet: the total ends with +.
    rig.app
        .crates
        .shown_mut()
        .entries_mut()
        .nth(2)
        .unwrap()
        .duration = None;
    rig.frame(Vec::new());
    assert_eq!(rig.app.readout_drawn.clone().unwrap().0, "7:30/1:07:30+");
}

#[test]
fn the_time_readout_gives_way_first_as_the_bar_narrows() {
    let mut rig = credited_rig("readout-narrow");
    set_tempos(&mut rig, &[124, 128, 132, 136, 138, 140]);
    for e in rig.app.crates.shown_mut().entries_mut() {
        e.duration = Some(3995.0);
    }
    let clear_w = rig.app.def.sprite("bpm_clear").w as f32;
    let mut seen = (false, false, false);
    for w in (275..=1000).step_by(5) {
        rig.screen.x = w as f32;
        rig.frame(Vec::new());
        match rig.app.readout_drawn.clone() {
            Some((text, x, rw)) => {
                if text.contains('/') {
                    seen.0 = true;
                } else {
                    seen.1 = true;
                }
                assert!(x >= 4.0 + SEARCH_MIN_W + 4.0, "{w}: the search keeps 60");
                assert!(x + rw <= w as f32 - 8.0 - clear_w, "{w}: inside the bar");
                if let Some((fx, _)) = rig.app.filters_drawn {
                    assert!(
                        fx >= x + rw,
                        "{w}: the filters follow it ({fx} < {})",
                        x + rw
                    );
                }
            }
            None => seen.2 = true,
        }
        assert!(rig.app.bpm_slider.is_some(), "{w}: the BPM control stays");
    }
    assert_eq!(seen, (true, true, true), "full, total only, and none");
}

#[test]
fn a_grouped_crate_without_albums_keeps_its_column_header() {
    let mut rig = Rig::new("header-loose", Vec::new(), |_| {});
    rig.fill_playlist(3);
    for (e, artist) in rig.app.crates.shown_mut().entries_mut().zip([
        "Moodymann",
        "Anthony Shake Shakir",
        "Theo Parrish",
    ]) {
        e.artist = artist.into();
    }
    rig.app.crates.set_grouped(PLAYLIST, true);
    rig.screen.x = 700.0;
    let out = rig.frame(Vec::new());
    assert!(rig.app.pl_header(), "loose tracks: the header stays");
    assert!(shows(&out, "Artist"), "{:?}", text_list(&out));
    rig.click_text("Artist");
    let artists: Vec<String> = rig
        .app
        .crates
        .shown()
        .entries()
        .iter()
        .map(|e| e.artist.clone())
        .collect();
    assert_eq!(
        artists,
        ["Anthony Shake Shakir", "Moodymann", "Theo Parrish"]
    );
    // A search that hides every row doesn't take the header away.
    rig.app
        .apply(Action::SetSearch("nothing".into()), &rig.ctx.clone());
    rig.frame(Vec::new());
    assert!(rig.app.pl_header());
    // One album in the crate: record rows, and no header.
    rig.app
        .crates
        .shown_mut()
        .entries_mut()
        .next()
        .unwrap()
        .album = "Silentintroduction".into();
    rig.frame(Vec::new());
    assert!(!rig.app.pl_header());
}

#[test]
fn the_players_second_line_is_the_album_of_a_file_and_empty_when_nothing_plays() {
    let mut rig = Rig::new("second-line", vec![fixture("tone.flac")], |_| {});
    // Nothing plays yet: no detail, whatever the entry holds.
    assert_eq!(rig.app.now_playing_detail(), None);
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    rig.app
        .crates
        .playing_mut()
        .entries_mut()
        .for_each(|e| e.album = "Silentintroduction".into());
    assert_eq!(
        rig.app.now_playing_detail().as_deref(),
        Some("Silentintroduction")
    );
    rig.app
        .crates
        .playing_mut()
        .entries_mut()
        .for_each(|e| e.album.clear());
    assert_eq!(rig.app.now_playing_detail(), None, "no tag, no line");
    // The verdict flash takes the first line only: the second stays.
    rig.app
        .crates
        .playing_mut()
        .entries_mut()
        .for_each(|e| e.album = "Silentintroduction".into());
    rig.app.flash(Verdict::Wanted, 1);
    rig.frame(Vec::new());
    assert!(rig.app.flash.is_some());
    assert!(rig.app.now_playing_detail().is_some());
    rig.key(Key::Space, Modifiers::SHIFT);
    rig.until(|r| r.app.position.state == PlayState::Stopped, "stopped");
    assert_eq!(rig.app.now_playing_detail(), None, "stopped: empty");
}

#[test]
fn a_fitted_track_flashes_the_rim_and_counts_to_the_target() {
    let mut rig = Rig::new(
        "deck-effects",
        vec![fixture("tone.flac"), fixture("tone.wav")],
        |_| {},
    );
    rig.screen.x = 720.0;
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    let ids = rig.ids(PLAYLIST);
    for (e, bpm) in rig
        .app
        .crates
        .playing_mut()
        .entries_mut()
        .zip([126u16, 130])
    {
        e.bpm = Some(bpm);
    }
    // SYNC on at 126 over the 126 BPM record: it locks and the rim flashes; the digits already
    // read the target, so there is nothing to count.
    rig.app.settings.sync = crate::sync::SyncSettings {
        on: true,
        target: 126,
        target_cents: None,
        wide: false,
        pending: None,
    };
    rig.frame(Vec::new());
    assert!(rig.app.deck_flash.is_some(), "the rim flashes as it locks");
    assert!(rig.app.deck_count.is_none());
    // The flash ends by itself.
    rig.app.deck_flash = Some(Instant::now() - Duration::from_secs(1));
    rig.frame(Vec::new());
    assert!(rig.app.deck_flash.is_none());
    // The next track is 130 BPM, fitted at -3.1%: the rim flashes and the digits count from 130
    // to 126.
    rig.app.play_entry(PLAYLIST, ids[1]);
    rig.until(
        |r| r.app.crates.playing().current() == Some(ids[1]),
        "the second plays",
    );
    // (The fit waits for the clock to say it plays.)
    rig.until(|r| r.app.deck_flash.is_some(), "the rim flashes");
    let (from, to, _) = rig.app.deck_count.expect("it counts");
    assert_eq!((from, to), (13_000, 12_600));
    assert_eq!(
        deck(&rig).1,
        "130",
        "the count starts at the record's own BPM"
    );
    // It ends by itself, on the target.
    rig.app.deck_count = Some((13_000, 12_600, Instant::now() - Duration::from_secs(1)));
    rig.frame(Vec::new());
    assert!(rig.app.deck_count.is_none());
    assert_eq!(deck(&rig).1, "126");
    // Back to the 126 BPM record: the rim flashes, and there is nothing to count.
    rig.app.deck_flash = None;
    rig.app.play_entry(PLAYLIST, ids[0]);
    rig.until(
        |r| r.app.crates.playing().current() == Some(ids[0]),
        "the first plays again",
    );
    rig.until(|r| r.app.deck_flash.is_some(), "the rim flashes again");
    assert!(rig.app.deck_count.is_none(), "same tempo: nothing to count");
}

#[test]
fn dragging_the_band_anywhere_that_is_not_a_control_moves_the_window() {
    let mut rig = Rig::new("band-drag", Vec::new(), |_| {});
    rig.screen.x = 1100.0;
    rig.frame(Vec::new());
    let panels = crate::layout::band_panels(1100.0);
    let drags = |rig: &mut Rig, at: Pos2| {
        rig.frame(vec![Event::PointerMoved(at)]);
        let mut sent = start_drag(&rig.press(at, PointerButton::Primary, true));
        for dx in [4.0, 10.0, 16.0] {
            sent |= start_drag(&rig.frame(vec![Event::PointerMoved(at + vec2(dx, 0.0))]));
        }
        rig.press(at + vec2(16.0, 0.0), PointerButton::Primary, false);
        sent
    };
    // The turntable's margin, and under the player's transport.
    assert!(
        drags(&mut rig, pos2(panels.deck.0 as f32 + 3.0, BAND_TOP + 70.0)),
        "the turntable"
    );
    assert!(drags(&mut rig, pos2(120.0, BAND_TOP + 72.0)), "the player");
    // A control that takes drags keeps them: pulling the pitch fader doesn't move the window.
    let fader = deck_at(&rig, "deck_pitch", 0.5, 0.5);
    assert!(!drags(&mut rig, fader), "the fader is a control");
    // And a control keeps its click.
    rig.click(deck_sync(&rig));
    assert!(rig.app.settings.sync.on);
}

/// A point on the player's seek bar at `fraction` of its travel, in a window `rig.screen.x` wide.
fn seek_at(rig: &Rig, fraction: f32) -> Pos2 {
    let w = crate::layout::band_panels(rig.screen.x).player.1 as f32;
    let thumb = rig.app.def.sprite("seek_thumb").w as f32;
    let travel = w - 8.0 - thumb;
    pos2(
        4.0 + thumb / 2.0 + fraction * travel,
        BAND_TOP + SEEK_Y + 3.0,
    )
}

#[test]
fn the_seek_bar_follows_playback_and_seeks_on_release() {
    let mut rig = Rig::new("seek-bar", vec![fixture("tone.flac")], |_| {});
    rig.ctx.global_style_mut(|s| {
        s.interaction.tooltip_delay = 0.0;
        s.interaction.tooltip_grace_time = 0.0;
    });
    rig.until(
        |r| {
            r.app.position.state == PlayState::Playing
                && r.app
                    .now_playing
                    .as_ref()
                    .is_some_and(|i| i.duration_secs.is_some())
        },
        "playing, its length known",
    );
    // The thumb follows playback.
    rig.until(
        |r| r.app.seek_drawn.is_some_and(|v| v > 0.05),
        "the thumb moves",
    );
    // A click at 75% of a 2 s track seeks to 1.5 s, and the thumb goes there at once and
    // stays: it never jumps back to the old position while the seek is on its way.
    rig.click(seek_at(&rig, 0.75));
    for _ in 0..20 {
        rig.pump();
        let v = rig.app.seek_drawn.unwrap();
        assert!(v > 0.7, "the thumb stays at the target: {v}");
    }
    rig.until(
        |r| (r.app.position.seconds() - 1.5).abs() < 0.2,
        "seeked to 1.5 s",
    );
    // A drag: the thumb follows the pointer, and the seek waits for the release.
    let from = seek_at(&rig, 0.9);
    rig.frame(vec![Event::PointerMoved(from)]);
    rig.press(from, PointerButton::Primary, true);
    for f in [0.7, 0.5, 0.25] {
        rig.frame(vec![Event::PointerMoved(seek_at(&rig, f))]);
    }
    assert!(rig.app.seek_drag.is_some_and(|v| (v - 0.25).abs() < 0.02));
    assert!(rig.app.seek_drawn.is_some_and(|v| (v - 0.25).abs() < 0.02));
    rig.press(seek_at(&rig, 0.25), PointerButton::Primary, false);
    assert!(rig.app.seek_drag.is_none(), "released: the seek is sent");
    rig.until(
        |r| (r.app.position.seconds() - 0.5).abs() < 0.2,
        "seeked to 0.5 s",
    );
    // The tooltip names the time under the pointer: 50% of 2 s.
    let mid = seek_at(&rig, 0.5);
    let mut seen = false;
    for _ in 0..10 {
        let out = rig.frame(vec![Event::PointerMoved(mid)]);
        seen |= shows(&out, "0:01");
    }
    assert!(seen, "the tooltip names 0:01");
    // Stopped: no thumb, and a click seeks nothing.
    rig.key(Key::Space, Modifiers::SHIFT);
    rig.until(|r| r.app.position.state == PlayState::Stopped, "stopped");
    rig.frame(Vec::new());
    assert_eq!(rig.app.seek_drawn, None);
    rig.click(seek_at(&rig, 0.75));
    assert!(rig.app.seek_drag.is_none());
    assert_eq!(rig.app.position.state, PlayState::Stopped);
}

/// A playing track at `bpm` with SYNC on at the record's tempo, in a window wide enough for
/// the turntable.
fn fine_tempo_rig(name: &str, bpm: f64) -> Rig {
    let mut rig = Rig::new(name, vec![fixture("tone.flac")], |_| {});
    rig.screen.x = 720.0;
    rig.until(
        |r| {
            r.app.position.state == PlayState::Playing
                && r.app.track_refs.contains_key(&r.app.position.track)
        },
        "the track plays",
    );
    let track = rig.app.track_refs[&rig.app.position.track].clone();
    rig.app.set_track_bpm(&track, bpm);
    rig.app.settings.sync = crate::sync::SyncSettings {
        on: true,
        target: bpm.round() as u16,
        target_cents: None,
        wide: false,
        pending: None,
    };
    rig.frame(Vec::new());
    rig
}

#[test]
fn the_tempo_shows_and_steps_in_hundredths() {
    let mut rig = fine_tempo_rig("fine-steps", 124.0);
    // The big digits and the small hundredths.
    rig.app.settings.sync.set_cents(12_635);
    rig.frame(Vec::new());
    assert_eq!(deck(&rig).1, "126");
    assert_eq!(rig.app.deck_fine_drawn, Some(35));
    // Shift+drag on the fader: 0.01 BPM per skin pixel from where it was grabbed.
    rig.app.settings.sync.set_cents(12_400);
    let mid = deck_at(&rig, "deck_pitch", 0.5, 0.5);
    rig.mods = Modifiers::SHIFT;
    rig.frame(vec![Event::PointerMoved(mid)]);
    rig.press(mid, PointerButton::Primary, true);
    for dy in [10.0, 20.0, 35.0] {
        rig.frame(vec![Event::PointerMoved(mid - vec2(0.0, dy))]);
    }
    rig.press(mid - vec2(0.0, 35.0), PointerButton::Primary, false);
    assert_eq!(
        rig.app.settings.sync.cents(),
        12_435,
        "Shift+drag up 35 pixels"
    );
    // Shift+scroll: 0.01 BPM per 4 skin pixels.
    rig.app.settings.sync.set_cents(12_600);
    rig.frame(vec![
        Event::PointerMoved(mid),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 8.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::SHIFT,
        },
    ]);
    for _ in 0..30 {
        rig.frame(Vec::new());
    }
    rig.mods = Modifiers::NONE;
    assert_eq!(
        rig.app.settings.sync.cents(),
        12_602,
        "Shift+scroll 8 pixels"
    );
    // Alt+↑ three times: 0.03 BPM.
    rig.app.settings.sync.set_cents(12_600);
    for _ in 0..3 {
        rig.key(Key::ArrowUp, Modifiers::ALT);
    }
    assert_eq!(rig.app.settings.sync.cents(), 12_603);
    // At the top of the record's range, a step stays there.
    rig.app.settings.sync.set_cents(13_392);
    rig.key(Key::ArrowUp, Modifiers::ALT);
    assert_eq!(rig.app.settings.sync.cents(), 13_392, "kept within ±8%");
    // Shift+↑ is still 1 BPM, keeping the hundredths.
    rig.app.settings.sync.set_cents(12_635);
    rig.key(Key::ArrowUp, Modifiers::SHIFT);
    assert_eq!(rig.app.settings.sync.cents(), 12_735);
    // With SYNC on, the plain arrows step the tempo by 0.1 BPM and leave the volume alone.
    let volume = rig.app.settings.volume;
    rig.app.settings.sync.set_cents(12_600);
    rig.key(Key::ArrowUp, Modifiers::NONE);
    rig.key(Key::ArrowUp, Modifiers::NONE);
    rig.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.app.settings.sync.cents(), 12_610);
    assert_eq!(rig.app.settings.volume, volume);
    // With SYNC off, they are the volume again.
    rig.app.settings.sync.on = false;
    rig.key(Key::ArrowDown, Modifiers::NONE);
    assert!(rig.app.settings.volume < volume, "the volume goes down");
    assert_eq!(rig.app.settings.sync.cents(), 12_610);
}

#[test]
fn the_tempo_can_be_typed_into_the_digits() {
    let mut rig = fine_tempo_rig("fine-typing", 124.0);
    let digits = deck_at(&rig, "deck_target", 0.5, 0.5);
    // Type 126.359: the third decimal is refused; Backspace deletes; Space types nothing and
    // doesn't pause; Enter sets it.
    rig.click(digits);
    assert_eq!(rig.app.deck_typing.as_deref(), Some(""));
    rig.frame(vec![Event::Text("126,359".into())]);
    assert_eq!(rig.app.deck_typing.as_deref(), Some("126.35"));
    assert_eq!(deck(&rig).1, "126.35", "the digits show what is typed");
    rig.key(Key::Backspace, Modifiers::NONE);
    assert_eq!(rig.app.deck_typing.as_deref(), Some("126.3"));
    rig.frame(vec![Event::Text("5".into())]);
    rig.key(Key::Space, Modifiers::NONE);
    assert_eq!(rig.app.position.state, PlayState::Playing, "Space waits");
    assert_eq!(rig.app.deck_typing.as_deref(), Some("126.35"));
    rig.key(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.app.deck_typing, None);
    assert_eq!(rig.app.settings.sync.cents(), 12_635);
    // Out of the record's range: kept within it, and said.
    rig.click(digits);
    rig.frame(vec![Event::Text("150".into())]);
    rig.key(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.app.settings.sync.cents(), 13_392);
    let said = rig.app.message.as_ref().map(|(t, _)| t.clone());
    assert_eq!(
        said.as_deref(),
        Some("Kept within ±8% of this record: 133.92 BPM")
    );
    // Esc cancels, and so does a click anywhere else.
    rig.click(digits);
    rig.frame(vec![Event::Text("99".into())]);
    rig.key(Key::Escape, Modifiers::NONE);
    assert_eq!(rig.app.deck_typing, None);
    assert_eq!(rig.app.settings.sync.cents(), 13_392);
    rig.click(digits);
    rig.click(rig.row(0));
    assert_eq!(rig.app.deck_typing, None, "a click elsewhere");
    // With SYNC off the digits don't open.
    rig.app.settings.sync.on = false;
    rig.frame(Vec::new());
    rig.click(digits);
    assert_eq!(rig.app.deck_typing, None);
}

#[test]
fn holding_the_platter_stops_the_record_and_letting_go_spins_it_up() {
    let mut rig = Rig::new("platter-hold", vec![fixture("tone.flac")], |_| {});
    rig.screen.x = 720.0;
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    assert!(!rig.app.settings.sync.on, "it works with SYNC off");
    let platter = deck_at(&rig, "deck_platter", 0.5, 0.5);
    rig.frame(vec![Event::PointerMoved(platter)]);
    rig.press(platter, PointerButton::Primary, true);
    assert!(rig.app.platter_held);
    assert!(rig.engine().held(), "the engine holds the record");
    rig.until(|r| r.app.position.speed == 0.0, "the record stops");
    let at = rig.app.position.frame;
    for _ in 0..5 {
        rig.pump();
    }
    assert_eq!(rig.app.position.frame, at, "the time stops");
    assert!(rig.engine().held(), "still held while the button is down");
    rig.press(platter, PointerButton::Primary, false);
    assert!(!rig.app.platter_held);
    assert!(!rig.engine().held());
    rig.until(|r| r.app.position.speed == 1.0, "it spins back up");
    rig.until(
        |r| r.app.position.frame > at,
        "and plays on from where it stopped",
    );
    // Nothing playing: pressing the platter does nothing.
    rig.key(Key::Space, Modifiers::SHIFT);
    rig.until(|r| r.app.position.state == PlayState::Stopped, "stopped");
    rig.press(platter, PointerButton::Primary, true);
    assert!(!rig.engine().held());
    rig.press(platter, PointerButton::Primary, false);
}
