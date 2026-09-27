//! The visual engine: implements the host's `VisualScene`. Each frame: signals → director →
//! modulation → GPU compositor, then the egui layer (overlay, fader deck, toasts). Keys: `D`
//! deck, `M` / `Shift+M` mutate, `K` keep, `Backspace` undo, `1`–`5` rate.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use eframe::egui_wgpu;
use egui::{Align2, Color32, Key, Modifiers, Rect, RichText, Ui, vec2};
use ui::fullscreen::{SceneFrame, VisualScene};

use crate::codegen::{MUSIC_BANDS, MUSIC_FLOATS, music_index, pack_params, param_slots};
use crate::compositor::{Governor, Transition, TransitionKind, lerp_params};
use crate::deck::{Fader, FaderEvent, FaderView, Return, fader_ui};
use crate::director::{Command, DEFAULT_RULES, Director, LookInfo, Rules, Value};
use crate::gpu::{Gpu, Layer, LayerDraw, PostParams, SceneProgram, Targets};
use crate::library::{self, Changes, LoadError, SceneSource, Watcher};
use crate::manifest::{ParamType, SceneManifest};
use crate::modulation::{
    Contribution, Macro, Macros, RouteState, Source, compose, resolve, snap_speed,
};
use crate::overlay::{self, Fade};
use crate::signals::{SignalBus, Signals, track_seed};
use crate::variants::{BIG_SIGMA, History, Rng, SMALL_SIGMA, Variant, auto_name};

pub(crate) const ACCENT: Color32 = Color32::from_rgb(0, 230, 110);
const MAX_PARAM_FADERS: usize = 12;
const BENCH_SECS: f64 = 15.0;
/// Refreshes between sampling the clock and the frame being on screen: the host queues up to
/// three frames in fullscreen; a full queue shows a new frame after the ones ahead of it.
const PRESENT_LEAD_FRAMES: f32 = 2.5;

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
    })
}

struct Scene {
    src: SceneSource,
    program: Option<Rc<SceneProgram>>,
}

#[derive(Clone)]
struct Look {
    variant: Variant,
    /// On disk (saved or bundled), as opposed to an unsaved mutation from this session.
    saved: bool,
}

/// A look being drawn (A, or B during a crossfade).
struct Slot {
    look: usize,
    layer: Option<Layer>,
    routes: Vec<RouteState>,
    base: Vec<[f32; 4]>,
    live: Vec<[f32; 4]>,
}

impl Slot {
    fn new(look: usize) -> Self {
        Self {
            look,
            layer: None,
            routes: Vec::new(),
            base: Vec::new(),
            live: Vec::new(),
        }
    }
}

/// What automation says a macro should be.
#[derive(Debug, Clone, Copy)]
struct MacroAuto {
    from: f32,
    to: f32,
    start: f64,
    glide: f32,
    signal: Option<Source>,
}

impl Default for MacroAuto {
    fn default() -> Self {
        Self {
            from: 0.5,
            to: 0.5,
            start: 0.0,
            glide: 0.0,
            signal: None,
        }
    }
}

impl MacroAuto {
    fn value(&self, s: &Signals) -> f32 {
        if let Some(src) = self.signal {
            return src.value(s).clamp(0.0, 1.0);
        }
        if self.glide <= 0.0 {
            return self.to;
        }
        let t = ((s.beats - self.start) / self.glide as f64).clamp(0.0, 1.0) as f32;
        self.from + (self.to - self.from) * t
    }
}

/// Benchmark mode (`WINAMP_VISUAL_BENCH=1`): 15 s with no visuals (the host's own frame
/// rate), then each scene for 15 s, with frame times, CPU time spent in the engine, GPU time,
/// and render scale.
struct Bench {
    scenes: Vec<String>,
    idx: usize,
    started: f64,
    rows: Vec<BenchRow>,
}

/// Label of the benchmark's first phase, which renders nothing.
const HOST_ONLY: &str = "(no visuals)";

#[derive(Default)]
struct BenchRow {
    name: String,
    /// Frame interval, seconds.
    dt: Vec<f32>,
    scale: Vec<f32>,
    /// CPU milliseconds: all of `paint`, the GPU encode+submit inside it, and `ui`.
    paint: Vec<f32>,
    submit: Vec<f32>,
    ui: Vec<f32>,
    /// GPU milliseconds from submit to completion (as observed at the next poll: an upper bound).
    gpu: Vec<f32>,
}

/// CPU timings of the last frame, for the benchmark.
#[derive(Default, Clone, Copy)]
struct FrameCost {
    paint: f32,
    submit: f32,
    ui: f32,
}

pub struct VisualEngine {
    dir: PathBuf,
    gpu: Option<Gpu>,
    targets: Option<Targets>,
    prelude: Vec<(String, String)>,
    scenes: BTreeMap<String, Scene>,
    looks: Vec<Look>,
    infos: Vec<LookInfo>,
    a: Option<Slot>,
    b: Option<Slot>,
    transition: Option<Transition>,
    morph_from: Option<Vec<[f32; 4]>>,
    bus: SignalBus,
    signals: Signals,
    director: Director,
    macros: Macros,
    macro_auto: [MacroAuto; 6],
    macro_faders: [Fader; 6],
    macro_base: [f32; 6],
    /// (param index, component, fader) for the current scene.
    param_faders: Vec<(usize, usize, Fader)>,
    faders_scene: String,
    global_return: Return,
    deck: bool,
    fade: Fade,
    toasts: Vec<(String, f64, bool)>,
    history: History,
    rng: Rng,
    governor: Governor,
    epoch: Instant,
    last_frame: Option<f64>,
    frame: u64,
    /// Beats since kick, snare, hat, downbeat, section change, drop.
    since: [f32; 6],
    motion: f64,
    last_beats: Option<f64>,
    flash: Option<(f64, f32)>,
    watcher: Option<Watcher>,
    track: Option<u64>,
    seed: u64,
    ppp: f32,
    pointer: Option<egui::Pos2>,
    bench: Option<Bench>,
    cost: FrameCost,
    loaded: bool,
    /// The director is off and the look stays put (offline `--look`).
    pinned: bool,
    /// Crossfade position of layer B this frame.
    mix_b: f32,
}

impl Default for VisualEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualEngine {
    /// Uses `<config>/winamp_rust/visuals`. Cheap: assets are installed, loaded and validated
    /// on the first `init` (entering fullscreen), not at app startup.
    pub fn new() -> Self {
        Self::with_dir(&library::default_dir())
    }

    pub fn with_dir(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            gpu: None,
            targets: None,
            prelude: Vec::new(),
            scenes: BTreeMap::new(),
            looks: Vec::new(),
            infos: Vec::new(),
            a: None,
            b: None,
            transition: None,
            morph_from: None,
            bus: SignalBus::default(),
            signals: Signals::default(),
            director: Director::new(Rules::parse(DEFAULT_RULES).expect("bundled rules parse")),
            macros: Macros::default(),
            macro_auto: [MacroAuto::default(); 6],
            macro_faders: [Fader::default(); 6],
            macro_base: [0.5; 6],
            param_faders: Vec::new(),
            faders_scene: String::new(),
            global_return: Return::Bar,
            deck: false,
            fade: Fade::default(),
            toasts: Vec::new(),
            history: History::default(),
            rng: Rng(1),
            governor: Governor::default(),
            epoch: Instant::now(),
            last_frame: None,
            frame: 0,
            since: [1e3; 6],
            motion: 0.0,
            last_beats: None,
            flash: None,
            watcher: None,
            track: None,
            seed: 0,
            ppp: 2.0,
            pointer: None,
            bench: None,
            cost: FrameCost::default(),
            loaded: false,
            pinned: false,
            mix_b: 0.0,
        }
    }

    /// Installs missing bundled assets, loads everything, and starts watching for edits.
    fn ensure_loaded(&mut self) {
        self.load(true);
    }

    /// Installs missing bundled assets and loads everything; `watch` starts hot reload.
    fn load(&mut self, watch: bool) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if let Err(err) = library::install(&self.dir) {
            self.toast(
                format!(
                    "Could not install visuals into {}: {err}",
                    self.dir.display()
                ),
                true,
            );
        }
        self.load_all();
        if watch {
            self.watcher = Watcher::new(&self.dir).ok();
        }
    }

    /// Uses `gpu` for everything: compiles every scene and picks the first look. Called again
    /// with a new device, it rebuilds all GPU resources from what is loaded.
    fn attach(&mut self, gpu: Gpu) {
        self.gpu = Some(gpu);
        self.targets = None;
        let ids: Vec<String> = self.scenes.keys().cloned().collect();
        for id in &ids {
            self.compile(id);
        }
        for slot in [&mut self.a, &mut self.b].into_iter().flatten() {
            slot.layer = None;
        }
        if self.a.as_ref().is_none_or(|a| !self.usable(a.look))
            && let Some(first) = self.first_look()
        {
            self.a = Some(Slot::new(first));
        }
        self.refresh_infos();
    }

    /// An engine for offline rendering: the looks in `dir` (bundled ones installed if missing),
    /// no hot reload, drawing on `gpu`.
    pub fn new_offline(dir: &Path, gpu: Gpu) -> Self {
        let mut e = Self::with_dir(dir);
        e.load(false);
        e.attach(gpu);
        e
    }

    /// Shows `scene/variant` for the whole render: the director is off (macros still follow
    /// the `always` rules and modulation still runs).
    pub fn pin_look(&mut self, scene: &str, variant: &str) -> Result<(), String> {
        let look = self
            .looks
            .iter()
            .position(|l| l.variant.scene == scene && l.variant.name == variant)
            .filter(|&l| self.usable(l))
            .ok_or_else(|| {
                let known: Vec<String> = self
                    .looks
                    .iter()
                    .map(|l| format!("{}/{}", l.variant.scene, l.variant.name))
                    .collect();
                format!(
                    "no look {scene}/{variant} (available: {})",
                    known.join(", ")
                )
            })?;
        self.a = Some(Slot::new(look));
        self.b = None;
        self.transition = None;
        self.pinned = true;
        Ok(())
    }

    /// Advances the show to `tick.now` and renders it offscreen at `size`. For offline
    /// rendering: `tick.lead_secs` should be 0 (a file has no display delay). Returns false
    /// when nothing could be drawn.
    pub fn render_offline(&mut self, frame: &SceneFrame, tick: Tick, size: (u32, u32)) -> bool {
        self.step(frame, tick).is_some() && self.draw(frame, size, tick.dt).is_some()
    }

    pub fn gpu(&self) -> Option<&Gpu> {
        self.gpu.as_ref()
    }

    /// The offscreen targets of the last drawn frame.
    pub fn targets(&self) -> Option<&Targets> {
        self.targets.as_ref()
    }

    /// Errors met while loading (for reporting in offline runs).
    pub fn errors(&self) -> Vec<String> {
        self.toasts
            .iter()
            .filter(|(_, _, e)| *e)
            .map(|(t, _, _)| t.clone())
            .collect()
    }

    fn now(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    fn toast(&mut self, text: String, error: bool) {
        let now = self.now();
        if error {
            eprintln!("visuals: {text}");
        }
        self.toasts.retain(|(t, _, _)| *t != text);
        self.toasts.push((text, now, error));
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    fn load_error(&mut self, e: LoadError) {
        self.toast(e.to_string(), true);
    }

    // ---- loading -------------------------------------------------------------------------

    fn load_all(&mut self) {
        self.prelude = library::load_prelude(&self.dir);
        for id in library::scene_ids(&self.dir) {
            match library::load_scene(&self.dir, &id, &self.prelude) {
                Ok(src) => {
                    self.scenes.insert(id, Scene { src, program: None });
                }
                Err(e) => self.load_error(e),
            }
        }
        match library::load_rules(&self.dir) {
            Ok(r) => self.director.rules = r,
            Err(e) => self.load_error(e),
        }
        self.rebuild_looks();
    }

    /// Reloads looks from disk (keeping unsaved ones) and remaps the slots onto them.
    fn rebuild_looks(&mut self) {
        let key = |l: &Look| (l.variant.scene.clone(), l.variant.name.clone());
        let a_key = self.a.as_ref().map(|s| key(&self.looks[s.look]));
        let b_key = self.b.as_ref().map(|s| key(&self.looks[s.look]));
        let session: Vec<Look> = self.looks.iter().filter(|l| !l.saved).cloned().collect();
        let mut looks = Vec::new();
        let ids: Vec<String> = self.scenes.keys().cloned().collect();
        for id in &ids {
            let (vars, errors) = library::load_variants(&self.dir, id);
            for e in errors {
                self.load_error(e);
            }
            if vars.is_empty() {
                looks.push(Look {
                    variant: Variant::defaults(id, &self.scenes[id].src.manifest),
                    saved: false,
                });
            }
            looks.extend(vars.into_iter().map(|variant| Look {
                variant,
                saved: true,
            }));
        }
        for l in session {
            if self.scenes.contains_key(&l.variant.scene)
                && !looks.iter().any(|x| key(x) == key(&l))
            {
                looks.push(l);
            }
        }
        self.looks = looks;
        let find = |k: &(String, String), looks: &[Look]| {
            looks
                .iter()
                .position(|l| key(l) == *k)
                .or_else(|| looks.iter().position(|l| l.variant.scene == k.0))
        };
        match a_key.and_then(|k| find(&k, &self.looks)) {
            Some(i) => self.a.as_mut().expect("had a").look = i,
            None => self.a = None,
        }
        match b_key.and_then(|k| find(&k, &self.looks)) {
            Some(i) => self.b.as_mut().expect("had b").look = i,
            None => {
                self.b = None;
                if self
                    .transition
                    .is_some_and(|t| t.kind == TransitionKind::Crossfade)
                {
                    self.transition = None;
                }
            }
        }
        self.refresh_infos();
    }

    fn refresh_infos(&mut self) {
        self.infos = self
            .looks
            .iter()
            .map(|l| {
                let m = &self.scenes[&l.variant.scene].src.manifest;
                LookInfo {
                    scene: l.variant.scene.clone(),
                    variant: l.variant.name.clone(),
                    tags: m.tags.iter().chain(&l.variant.tags).cloned().collect(),
                    rating: l.variant.rating,
                }
            })
            .collect();
    }

    fn compile(&mut self, id: &str) -> bool {
        let (Some(gpu), Some(scene)) = (&self.gpu, self.scenes.get(id)) else {
            return false;
        };
        match SceneProgram::new(gpu, id, &scene.src.assembled, &scene.src.manifest) {
            Ok(p) => {
                self.scenes.get_mut(id).expect("exists").program = Some(Rc::new(p));
                true
            }
            Err(e) => {
                self.toast(e.to_string(), true);
                false
            }
        }
    }

    fn usable(&self, look: usize) -> bool {
        self.looks.get(look).is_some_and(|l| {
            self.scenes
                .get(&l.variant.scene)
                .is_some_and(|s| s.program.is_some())
        })
    }

    fn apply_changes(&mut self, ch: Changes) {
        if ch.prelude {
            self.prelude = library::load_prelude(&self.dir);
        }
        let ids: Vec<String> = if ch.prelude {
            library::scene_ids(&self.dir)
        } else {
            ch.scenes.iter().cloned().collect()
        };
        let mut looks_changed = !ch.variants.is_empty();
        for id in ids {
            if !self.dir.join("scenes").join(&id).join("scene.ron").exists() {
                continue;
            }
            match library::load_scene(&self.dir, &id, &self.prelude) {
                Ok(src) => {
                    // Keep the old program running unless the new one compiles.
                    let old = self.scenes.insert(id.clone(), Scene { src, program: None });
                    let ok = self.gpu.is_none() || self.compile(&id);
                    if ok {
                        looks_changed |= old.is_none();
                        self.reset_slots_of(&id);
                        self.toast(format!("Reloaded {id}"), false);
                    } else if let Some(old) = old {
                        self.scenes.insert(id.clone(), old);
                    } else {
                        self.scenes.remove(&id);
                    }
                }
                Err(e) => self.load_error(e),
            }
        }
        if looks_changed {
            self.rebuild_looks();
        }
        if ch.director {
            match library::load_rules(&self.dir) {
                Ok(r) => {
                    self.director.rules = r;
                    self.toast("Reloaded director.ron".into(), false);
                }
                Err(e) => self.load_error(e),
            }
        }
    }

    fn reset_slots_of(&mut self, id: &str) {
        for slot in [&mut self.a, &mut self.b].into_iter().flatten() {
            if self
                .looks
                .get(slot.look)
                .is_some_and(|l| l.variant.scene == id)
            {
                slot.layer = None;
                slot.routes.clear();
            }
        }
        if self.faders_scene == id {
            self.faders_scene.clear();
        }
    }

    // ---- looks and transitions -------------------------------------------------------------

    fn scene_of(&self, look: usize) -> &str {
        &self.looks[look].variant.scene
    }

    fn manifest_of(&self, look: usize) -> &SceneManifest {
        &self.scenes[self.scene_of(look)].src.manifest
    }

    fn first_look(&self) -> Option<usize> {
        (0..self.looks.len())
            .filter(|&i| self.usable(i))
            .max_by_key(|&i| (self.looks[i].variant.rating, std::cmp::Reverse(i)))
    }

    fn show(&mut self, look: usize, kind: TransitionKind, beats: f64) {
        if !self.usable(look) {
            return;
        }
        if self.a.is_none() {
            self.a = Some(Slot::new(look));
            return;
        }
        // Finish any crossfade in progress first.
        if let Some(b) = self.b.take() {
            self.a = Some(b);
            self.transition = None;
        }
        let a_look = self.a.as_ref().expect("a exists").look;
        let same_scene = self.scene_of(a_look) == self.scene_of(look);
        let kind = match kind {
            _ if beats <= 0.0 => TransitionKind::Cut,
            TransitionKind::Morph if !same_scene => TransitionKind::Crossfade,
            k => k,
        };
        let now = self.signals.beats;
        let a = self.a.as_mut().expect("a exists");
        match kind {
            TransitionKind::Cut => {
                a.look = look;
                a.routes.clear();
                if !same_scene {
                    a.layer = None;
                }
                self.transition = None;
                self.morph_from = None;
            }
            TransitionKind::Crossfade => {
                self.b = Some(Slot::new(look));
                self.transition = Some(Transition {
                    kind,
                    start_beats: now,
                    length_beats: beats,
                });
                self.morph_from = None;
                self.governor.pre_lower();
            }
            TransitionKind::Morph => {
                self.morph_from = Some(a.base.clone());
                a.look = look;
                a.routes.clear();
                self.transition = Some(Transition {
                    kind,
                    start_beats: now,
                    length_beats: beats,
                });
            }
        }
    }

    fn current_look(&self) -> Option<usize> {
        self.a.as_ref().map(|s| s.look)
    }

    /// The current look's values with manual faders baked in.
    fn current_variant(&mut self) -> Option<Variant> {
        let look = self.current_look()?;
        let m = self.manifest_of(look).clone();
        let mut v = self.looks[look].variant.clone();
        let mut values = v.values(&m);
        let beats = self.signals.beats;
        for (p, c, f) in &mut self.param_faders {
            let (lo, hi) = (m.params[*p].min[*c], m.params[*p].max[*c]);
            let auto = (values[*p][*c] - lo) / (hi - lo).max(1e-9);
            if let Some(n) = f.manual(auto, beats) {
                values[*p][*c] = lo + n * (hi - lo);
            }
        }
        v.set_values(&m, &values);
        Some(v)
    }

    fn add_look(&mut self, variant: Variant, saved: bool) -> usize {
        self.looks.push(Look { variant, saved });
        self.refresh_infos();
        self.looks.len() - 1
    }

    fn mutate(&mut self, big: bool) {
        let Some(cur) = self.current_variant() else {
            return;
        };
        let m = self
            .manifest_of(self.current_look().expect("has look"))
            .clone();
        let child = cur.mutate(
            &m,
            if big { BIG_SIGMA } else { SMALL_SIGMA },
            big,
            &mut self.rng,
        );
        self.history.push(cur);
        let name = child.name.clone();
        let look = self.add_look(child, false);
        for (_, _, f) in &mut self.param_faders {
            f.rearm();
        }
        self.show(look, TransitionKind::Morph, 1.0);
        self.toast(
            format!("{} → {name}", if big { "Big mutate" } else { "Mutate" }),
            false,
        );
    }

    fn keep(&mut self) {
        let Some(mut v) = self.current_variant() else {
            return;
        };
        v.parent = Some(v.name.clone());
        v.name = auto_name(&mut self.rng);
        match library::save_variant(&self.dir, &v) {
            Ok(()) => {
                let name = v.name.clone();
                let look = self.add_look(v, true);
                if let Some(a) = &mut self.a {
                    a.look = look;
                }
                for (_, _, f) in &mut self.param_faders {
                    f.rearm();
                }
                self.toast(format!("Kept {name}"), false);
            }
            Err(e) => self.toast(format!("Could not save variant: {e}"), true),
        }
    }

    fn undo(&mut self) {
        let Some(v) = self.history.pop() else {
            self.toast("Nothing to undo".into(), false);
            return;
        };
        let name = v.name.clone();
        let look = match self.looks.iter().position(|l| {
            l.variant.scene == v.scene && l.variant.name == v.name && l.variant.params == v.params
        }) {
            Some(i) => i,
            None => self.add_look(v, false),
        };
        for (_, _, f) in &mut self.param_faders {
            f.rearm();
        }
        self.show(look, TransitionKind::Cut, 0.0);
        self.toast(format!("Undo → {name}"), false);
    }

    fn rate(&mut self, stars: u8) {
        let Some(look) = self.current_look() else {
            return;
        };
        self.looks[look].variant.rating = stars;
        let l = self.looks[look].clone();
        if l.saved || l.variant.name == "default" {
            if let Err(e) = library::save_variant(&self.dir, &l.variant) {
                self.toast(format!("Could not save rating: {e}"), true);
                return;
            }
            self.looks[look].saved = true;
        }
        self.refresh_infos();
        self.toast(
            format!("{} {}", l.variant.name, "★".repeat(stars as usize)),
            false,
        );
    }

    fn rebuild_faders(&mut self) {
        let Some(look) = self.current_look() else {
            return;
        };
        let scene = self.scene_of(look).to_string();
        if scene == self.faders_scene {
            return;
        }
        let m = self.manifest_of(look);
        self.param_faders = m
            .params
            .iter()
            .enumerate()
            .filter(|(_, p)| p.mutable && p.ty != ParamType::Color)
            .flat_map(|(i, p)| (0..p.ty.components()).map(move |c| (i, c, Fader::default())))
            .take(MAX_PARAM_FADERS)
            .collect();
        self.faders_scene = scene;
    }

    // ---- per frame -----------------------------------------------------------------------

    fn apply(&mut self, cmds: Vec<Command>) {
        for c in cmds {
            match c {
                Command::Transition { kind, look, beats } => self.show(look, kind, beats),
                Command::SetMacro {
                    which,
                    value,
                    glide_beats,
                } => self.set_macro_auto(which, value, glide_beats),
                Command::Flash { beats } => self.flash = Some((self.signals.beats, beats.max(0.1))),
            }
        }
    }

    fn set_macro_auto(&mut self, which: Macro, value: Value, glide: f32) {
        let i = which.index();
        let cur = self.macro_auto[i].value(&self.signals);
        self.macro_auto[i] = match value {
            Value::Const(v) => MacroAuto {
                from: cur,
                to: v.clamp(0.0, 1.0),
                start: self.signals.beats,
                glide,
                signal: None,
            },
            Value::Signal(src) => MacroAuto {
                signal: Some(src),
                ..self.macro_auto[i]
            },
        };
    }

    fn update_triggers(&mut self, dbeats: f32) {
        let s = &self.signals;
        let beats_per_sec = s.tempo_bpm / 60.0;
        let t = &s.triggers;
        for (since, trig) in self.since.iter_mut().zip([
            t.kick,
            t.snare,
            t.hat,
            t.downbeat,
            t.section_change,
            t.drop,
        ]) {
            *since = if trig.fired() {
                trig.ago * beats_per_sec
            } else {
                (*since + dbeats).min(1e3)
            };
        }
    }

    fn music(&self, frame: &SceneFrame, size: (u32, u32), which: Option<usize>) -> Vec<f32> {
        let s = &self.signals;
        let mut m = vec![0.0; MUSIC_FLOATS];
        let flash = self.flash.map_or(0.0, |(start, len)| {
            let t = (s.beats - start) as f32;
            if t < 0.0 { 0.0 } else { (-t / len * 3.0).exp() }
        });
        let seed01 = (which.map_or(self.seed, |i| self.seed ^ fnv(self.scene_of(i))) >> 40) as f32
            / (1u64 << 24) as f32;
        let kind = s.section_kind.map_or(-1.0, |k| {
            analysis::SectionKind::ALL
                .iter()
                .position(|&x| x == k)
                .unwrap_or(0) as f32
        });
        let values: [(&str, f32); 32] = [
            ("time", s.seconds as f32),
            ("beats", s.beats as f32),
            ("motion", self.motion as f32),
            ("beat", s.beat),
            ("bar", s.bar),
            ("phrase", s.phrase),
            ("section_progress", s.section_progress),
            ("tempo", s.tempo_bpm),
            ("energy", s.energy),
            ("bass", s.bass),
            ("mid", s.mid),
            ("treble", s.treble),
            ("brightness", s.brightness),
            ("tension", s.tension),
            ("drop_in", s.drop_in),
            ("beat_confidence", s.beat_confidence),
            ("section_kind", kind),
            ("section_label", s.section_label as f32),
            ("kick", self.since[0]),
            ("snare", self.since[1]),
            ("hat", self.since[2]),
            ("downbeat", self.since[3]),
            ("section_change", self.since[4]),
            ("drop", self.since[5]),
            ("flash", flash),
            ("aspect", size.0 as f32 / size.1.max(1) as f32),
            ("res_x", size.0 as f32),
            ("res_y", size.1 as f32),
            ("playing", s.playing as u8 as f32),
            ("seed", seed01),
            ("frame", self.frame as f32),
            ("hue", self.macros.get(Macro::Hue) - 0.5),
        ];
        for (k, v) in values {
            m[music_index(k)] = v;
        }
        let bands = crate::codegen::MUSIC_FIELDS.len();
        for (i, b) in frame.bars.iter().take(MUSIC_BANDS * 4).enumerate() {
            m[bands + i] = *b;
        }
        m
    }

    fn update_macros(&mut self) {
        for (m, v, g) in self.director.rules.continuous().collect::<Vec<_>>() {
            self.set_macro_auto(m, v, g);
        }
        let beats = self.signals.beats;
        for m in Macro::ALL {
            let i = m.index();
            let auto = self.macro_auto[i].value(&self.signals);
            self.macro_base[i] = auto;
            let v = self.macro_faders[i].value(auto, beats);
            self.macros.set(
                m,
                if m == Macro::Speed {
                    snap_speed(v).0
                } else {
                    v
                },
            );
        }
    }

    /// Composed, packed parameters for a slot.
    fn slot_params(&mut self, slot_b: bool, dt: f32) -> Vec<f32> {
        let seed = self.seed;
        let beats = self.signals.beats;
        let signals = self.signals;
        let morph = match (slot_b, &self.transition, &self.morph_from) {
            (false, Some(t), Some(from)) if t.kind == TransitionKind::Morph => {
                t.progress(beats).map(|p| (from.clone(), p))
            }
            _ => None,
        };
        let slot = if slot_b {
            self.b.as_mut()
        } else {
            self.a.as_mut()
        };
        let Some(slot) = slot else { return Vec::new() };
        let look = &self.looks[slot.look];
        let m = &self.scenes[&look.variant.scene].src.manifest;
        let mut base = look.variant.values(m);
        if let Some((from, p)) = morph
            && from.len() == base.len()
        {
            base = lerp_params(&from, &base, p);
        }
        let mut manual: Vec<Option<[f32; 4]>> = vec![None; base.len()];
        if !slot_b {
            for (p, c, f) in &mut self.param_faders {
                let Some(def) = m.params.get(*p) else {
                    continue;
                };
                let (lo, hi) = (def.min[*c], def.max[*c]);
                let auto = (base[*p][*c] - lo) / (hi - lo).max(1e-9);
                if let Some(n) = f.manual(auto, beats) {
                    let v = manual[*p].get_or_insert(base[*p]);
                    v[*c] = lo + n * (hi - lo);
                }
            }
        }
        let slots = m.slots();
        let routes = look.variant.routes(m);
        slot.routes.resize_with(routes.len(), RouteState::default);
        let scene_seed = seed ^ fnv(&look.variant.scene);
        let mut contributions = Vec::new();
        for (k, r) in routes.iter().enumerate() {
            let id = fnv(&r.target) ^ (k as u64).wrapping_mul(0x9e37_79b9);
            if let (Some(v), Some(target)) = (
                r.eval(&mut slot.routes[k], &signals, scene_seed, id, dt),
                resolve(&slots, &r.target),
            ) {
                contributions.push(Contribution {
                    target,
                    mode: r.mode,
                    value: v,
                    depth: r.depth,
                });
            }
        }
        let out = compose(
            &slots,
            &base,
            &manual,
            &self.macros,
            &m.macro_targets(),
            &contributions,
        );
        let packed = pack_params(&out, param_slots(m));
        slot.base = base;
        slot.live = out;
        packed
    }

    fn feedback(&self, look: usize, dt: f32) -> (f32, f32) {
        let f = self.macros.get(Macro::Feedback);
        match self.manifest_of(look).feedback {
            Some(fb) => (
                (fb.decay * (0.5 + f)).clamp(0.0, 0.985).powf(dt * 60.0),
                fb.warp * (0.5 + f),
            ),
            None => (0.0, 0.0),
        }
    }

    fn bench_step(&mut self, now: f64, dt: f32) {
        let gpu_ms = self.gpu.as_ref().map_or(0.0, |g| g.last_gpu_ms());
        let (cost, scale) = (self.cost, self.governor.scale);
        let Some(b) = &mut self.bench else { return };
        if b.idx >= b.scenes.len() {
            return;
        }
        if now - b.started > 1.0 {
            let row = &mut b.rows[b.idx];
            row.dt.push(dt);
            row.scale.push(scale);
            row.paint.push(cost.paint);
            row.submit.push(cost.submit);
            row.ui.push(cost.ui);
            row.gpu.push(gpu_ms);
        }
        if now - b.started >= BENCH_SECS {
            b.idx += 1;
            b.started = now;
            if b.idx == b.scenes.len() {
                let report = bench_report(&b.rows);
                eprintln!("{report}");
                let path = self.dir.join("bench.txt");
                let _ = std::fs::write(&path, &report);
                self.toast(format!("Benchmark written to {}", path.display()), false);
                self.bench = None;
                return;
            }
        }
        let scene = b.scenes[b.idx].clone();
        let switched = now == b.started;
        if switched {
            self.toast(format!("Benchmark: {scene}"), false);
        }
        if scene != HOST_ONLY
            && self
                .current_look()
                .is_none_or(|l| self.scene_of(l) != scene)
            && let Some(look) = self.looks.iter().position(|l| l.variant.scene == scene)
        {
            self.show(look, TransitionKind::Cut, 0.0);
        }
    }

    fn bench_host_only(&self) -> bool {
        self.bench
            .as_ref()
            .is_some_and(|b| b.scenes.get(b.idx).is_some_and(|s| s == HOST_ONLY))
    }

    fn deck_ui(&mut self, ui: &mut Ui, inset: f32) {
        let Some(look) = self.current_look() else {
            return;
        };
        let beats = self.signals.beats;
        let bar = self.signals.bar;
        let mut events: Vec<(Option<usize>, usize, FaderEvent)> = Vec::new();
        let mut buttons: Vec<&'static str> = Vec::new();
        let scene_name = self.manifest_of(look).name.clone();
        let variant = self.looks[look].variant.clone();
        let m = self.manifest_of(look).clone();
        let a = self.a.as_ref();
        let (base, live) = a
            .map(|s| (s.base.clone(), s.live.clone()))
            .unwrap_or_default();
        egui::Area::new(egui::Id::new("visuals-deck"))
            .anchor(Align2::RIGHT_BOTTOM, vec2(-24.0, -(inset + 24.0)))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(Color32::from_black_alpha(200))
                    .corner_radius(8)
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{scene_name} · {}", variant.name))
                                    .size(13.0)
                                    .color(Color32::WHITE),
                            );
                            let stars = format!(
                                "{}{}",
                                "★".repeat(variant.rating as usize),
                                "☆".repeat(5 - variant.rating.min(5) as usize)
                            );
                            ui.label(RichText::new(stars).color(ACCENT));
                            ui.separator();
                            if ui
                                .button(format!("RETURN {}", self.global_return.name()))
                                .on_hover_text(
                                    "Click to change; right-click a fader to override it",
                                )
                                .clicked()
                            {
                                buttons.push("return");
                            }
                            for (label, id) in
                                [("M", "m"), ("⇧M", "bigm"), ("K", "k"), ("UNDO", "undo")]
                            {
                                if ui.button(label).clicked() {
                                    buttons.push(id);
                                }
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            for mac in Macro::ALL {
                                let i = mac.index();
                                let value = self.macros.get(mac);
                                let readout = (mac == Macro::Speed)
                                    .then(|| format!("{}×", snap_speed(value).1));
                                let view = FaderView {
                                    label: mac.name(),
                                    value,
                                    base: self.macro_base[i],
                                    live: value,
                                    manual: self.macro_faders[i].is_manual(),
                                    ret: self.macro_faders[i].ret,
                                    readout,
                                };
                                if let Some(e) =
                                    fader_ui(ui, egui::Id::new(("macro", i)), &view, ACCENT)
                                {
                                    events.push((None, i, e));
                                }
                            }
                            ui.separator();
                            for (k, (p, c, f)) in self.param_faders.iter().enumerate() {
                                let Some(def) = m.params.get(*p) else {
                                    continue;
                                };
                                let (lo, hi) = (def.min[*c], def.max[*c]);
                                let norm = |v: f32| (v - lo) / (hi - lo).max(1e-9);
                                let b = base.get(*p).map_or(0.0, |v| norm(v[*c]));
                                let l = live.get(*p).map_or(0.0, |v| norm(v[*c]));
                                let mut f2 = *f;
                                let value = f2.manual(b, beats).unwrap_or(b);
                                let label = if def.ty.components() > 1 {
                                    format!("{}.{}", def.name, ["x", "y", "z", "w"][*c])
                                } else {
                                    def.name.clone()
                                };
                                let view = FaderView {
                                    label: &label,
                                    value,
                                    base: b,
                                    live: l,
                                    manual: f.is_manual(),
                                    ret: f.ret,
                                    readout: None,
                                };
                                if let Some(e) =
                                    fader_ui(ui, egui::Id::new(("param", k)), &view, ACCENT)
                                {
                                    events.push((Some(k), k, e));
                                }
                            }
                        });
                    });
            });
        for id in buttons {
            match id {
                "return" => self.global_return = self.global_return.next(),
                "m" => self.mutate(false),
                "bigm" => self.mutate(true),
                "k" => self.keep(),
                "undo" => self.undo(),
                _ => {}
            }
        }
        for (param, i, e) in events {
            let is_speed = param.is_none() && i == Macro::Speed.index();
            let f = match param {
                None => &mut self.macro_faders[i],
                Some(k) => &mut self.param_faders[k].2,
            };
            match e {
                FaderEvent::Touch(v) => f.touch(v),
                FaderEvent::Release => {
                    if is_speed {
                        let v = f.value(0.5, beats);
                        f.touch(snap_speed(v).0);
                    }
                    f.release(beats, bar, self.global_return);
                }
                FaderEvent::CycleReturn => {
                    f.ret = match f.ret {
                        None => Some(Return::Beat),
                        Some(Return::Never) => None,
                        Some(r) => Some(r.next()),
                    }
                }
                FaderEvent::Rearm => f.rearm(),
            }
        }
    }
}

impl VisualEngine {
    fn paint_frame(&mut self, rect: Rect, frame: &SceneFrame) -> Option<egui::PaintCallback> {
        let now = self.now();
        let dt = self.last_frame.map_or(1.0 / 60.0, |t| (now - t) as f32);
        self.last_frame = Some(now);
        if let Some(ch) = self.watcher.as_mut().and_then(|w| w.poll()) {
            self.apply_changes(ch);
        }
        self.gpu.as_ref()?;
        // The host queues up to three frames (see `ui::app::surface_config`).
        let tick = Tick {
            now,
            dt,
            lead_secs: self.governor.refresh_interval() * PRESENT_LEAD_FRAMES,
        };
        self.step(frame, tick)?;
        // Render size: the screen at the governor's scale, capped by the scenes on screen.
        let a_look = self.a.as_ref()?.look;
        let b_look = self.b.as_ref().map(|s| s.look);
        self.governor.max_scale = [Some(a_look), b_look]
            .into_iter()
            .flatten()
            .filter_map(|l| self.manifest_of(l).max_scale)
            .fold(1.0, f32::min);
        self.governor.update(dt);
        let screen = (
            (rect.width() * self.ppp).round() as u32,
            (rect.height() * self.ppp).round() as u32,
        );
        let size = self.governor.size(screen);
        self.draw(frame, size, dt)?;
        let gpu = self.gpu.as_ref()?;
        Some(egui_wgpu::Callback::new_paint_callback(
            rect,
            gpu.final_callback(self.targets.as_ref()?),
        ))
    }

    /// Advances the show by one frame: signals, director, transitions, macros. Returns `None`
    /// when there is nothing to draw.
    fn step(&mut self, frame: &SceneFrame, tick: Tick) -> Option<()> {
        let (now, dt) = (tick.now, tick.dt);
        self.frame += 1;
        // Signals and musical time.
        // A frame is seen `lead_secs` after it is drawn (the display queue when live, none
        // offline), so read the audio clock that far ahead to land visuals on the audible beat.
        let position = presented(frame.position, tick.lead_secs);
        self.signals = self
            .bus
            .update(&position, frame.score, frame.bars, dt.min(0.25));
        let dbeats = self
            .last_beats
            .map_or(0.0, |b| (self.signals.beats - b).clamp(0.0, 4.0)) as f32;
        self.last_beats = Some(self.signals.beats);
        self.update_triggers(dbeats);
        let track = frame.position.track;
        if self.track != Some(track) {
            if self.track.is_some() {
                self.fade.poke(now);
            }
            self.track = Some(track);
        }
        self.seed = track_seed(frame.score, frame.title);
        self.rng = Rng(self.seed ^ self.frame);

        // Director (or the benchmark), then transitions.
        if self.bench.is_some() {
            self.bench_step(now, dt);
            if self.bench_host_only() {
                return None;
            }
        } else if !self.pinned
            && let Some(cur) = self.current_look()
        {
            let cmds = self.director.update(
                &self.signals,
                frame.score,
                track,
                self.seed,
                cur,
                &self.infos,
            );
            self.apply(cmds);
        }
        if self.a.is_none() {
            let first = self.first_look()?;
            self.a = Some(Slot::new(first));
        }
        self.mix_b = match self.transition {
            Some(t) => match t.progress(self.signals.beats) {
                Some(p) => p,
                None => {
                    if t.kind == TransitionKind::Crossfade
                        && let Some(b) = self.b.take()
                    {
                        self.a = Some(b);
                    }
                    self.transition = None;
                    self.morph_from = None;
                    0.0
                }
            },
            None => 0.0,
        };
        if self
            .transition
            .is_none_or(|t| t.kind != TransitionKind::Crossfade)
        {
            self.b = None;
        }
        self.rebuild_faders();
        self.update_macros();
        self.motion += dbeats as f64 * self.macros.speed() as f64;

        Some(())
    }

    /// Renders the current frame offscreen at `size` (scenes, feedback, bloom).
    fn draw(&mut self, frame: &SceneFrame, size: (u32, u32), dt: f32) -> Option<()> {
        let a_look = self.a.as_ref()?.look;
        let b_look = self.b.as_ref().map(|s| s.look);
        let gpu = self.gpu.as_ref()?;
        if self.targets.as_ref().is_none_or(|t| t.size != size) {
            self.targets = Some(Targets::new(gpu, size));
        }

        // Uniforms.
        let params_a = self.slot_params(false, dt);
        let params_b = self.slot_params(true, dt);
        let music_a = self.music(frame, size, Some(a_look));
        let music_b = b_look.map(|l| self.music(frame, size, Some(l)));
        let (decay_a, warp_a) = self.feedback(a_look, dt);
        let (decay_b, warp_b) = b_look.map_or((0.0, 0.0), |l| self.feedback(l, dt));
        let intensity = self.macros.intensity_gain();
        let post = PostParams {
            mix_b: self.mix_b,
            decay_a,
            warp_a,
            decay_b,
            warp_b,
            aspect: size.0 as f32 / size.1.max(1) as f32,
            motion: self.motion as f32,
            chroma: 0.5 * intensity * (-self.since[0] * 5.0).exp(),
            flash: 0.5 * music_a[music_index("flash")],
            frame: self.frame as f32,
            hue: self.macros.get(Macro::Hue) - 0.5,
            ..Default::default()
        };

        // Layers follow their scene's current program (hot reload swaps it).
        for (slot, look) in [(&mut self.a, Some(a_look)), (&mut self.b, b_look)] {
            let (Some(slot), Some(look)) = (slot.as_mut(), look) else {
                continue;
            };
            let program = self.scenes[&self.looks[look].variant.scene]
                .program
                .clone()?;
            if slot
                .layer
                .as_ref()
                .is_none_or(|l| !Rc::ptr_eq(&l.program, &program))
            {
                slot.layer = Some(Layer::new(self.gpu.as_ref()?, program));
                self.governor.settle();
            }
        }
        let gpu = self.gpu.as_mut()?;
        let targets = self.targets.as_mut()?;
        let a_layer = self.a.as_mut()?.layer.as_mut()?;
        let draw_b = match (&mut self.b, &music_b) {
            (Some(b), Some(mb)) => b.layer.as_mut().map(|layer| LayerDraw {
                layer,
                music: mb,
                params: &params_b,
            }),
            _ => None,
        };
        let t_submit = Instant::now();
        gpu.render(
            targets,
            LayerDraw {
                layer: a_layer,
                music: &music_a,
                params: &params_a,
            },
            draw_b,
            &post,
        );
        self.cost.submit = t_submit.elapsed().as_secs_f32() * 1000.0;
        Some(())
    }
}

/// A frame's timing: `now` (seconds on any monotonic base), the time since the last frame, and
/// how long after drawing the frame will be seen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    pub now: f64,
    pub dt: f32,
    pub lead_secs: f32,
}

/// The audible position one presentation delay ahead (only while playing).
fn presented(mut p: audio::Position, lead_secs: f32) -> audio::Position {
    if p.state == audio::PlayState::Playing {
        p.frame += (lead_secs as f64 * p.sample_rate as f64) as u64;
    }
    p
}

fn bench_report(rows: &[BenchRow]) -> String {
    fn mean(v: &[f32]) -> f32 {
        v.iter().sum::<f32>() / v.len().max(1) as f32
    }
    let mut out = String::from(
        "scene              fps   p95 ms | paint ms  submit ms  gpu<= ms  ui ms | scale min/avg\n",
    );
    for r in rows {
        if r.dt.is_empty() {
            continue;
        }
        let mut sorted = r.dt.clone();
        sorted.sort_by(f32::total_cmp);
        let p95 = sorted[(sorted.len() * 95 / 100).min(sorted.len() - 1)];
        let min = r.scale.iter().copied().fold(1.0, f32::min);
        out += &format!(
            "{:<18} {:>5.1} {:>7.2} | {:>8.2} {:>10.2} {:>9.2} {:>6.2} | {min:.2}/{:.2}\n",
            r.name,
            1.0 / mean(&r.dt),
            p95 * 1000.0,
            mean(&r.paint),
            mean(&r.submit),
            mean(&r.gpu),
            mean(&r.ui),
            mean(&r.scale),
        );
    }
    out += "\nfps/p95: frame interval. paint/submit/ui: CPU time in the visual engine per frame.\n";
    out += "gpu<=: time from submitting the visuals to the GPU finishing them (an upper bound).\n";
    out
}

impl VisualScene for VisualEngine {
    fn init(&mut self, rs: &egui_wgpu::RenderState) {
        self.ensure_loaded();
        self.attach(Gpu::from_render_state(rs));
    }

    fn entered(&mut self) {
        let now = self.now();
        self.fade.poke(now);
        if std::env::var_os("WINAMP_VISUAL_BENCH").is_some() && self.bench.is_none() {
            let scenes: Vec<String> = std::iter::once(HOST_ONLY.to_string())
                .chain(
                    self.scenes
                        .iter()
                        .filter(|(_, s)| s.program.is_some())
                        .map(|(k, _)| k.clone()),
                )
                .collect();
            self.bench = Some(Bench {
                rows: scenes
                    .iter()
                    .map(|s| BenchRow {
                        name: s.clone(),
                        ..Default::default()
                    })
                    .collect(),
                scenes,
                idx: 0,
                started: now,
            });
            self.toast(
                format!("Benchmark: {BENCH_SECS} s with no visuals, then {BENCH_SECS} s per scene"),
                false,
            );
        }
    }

    fn paint(&mut self, rect: Rect, frame: &SceneFrame) -> Option<egui::PaintCallback> {
        let t0 = Instant::now();
        let cb = self.paint_frame(rect, frame);
        self.cost.paint = t0.elapsed().as_secs_f32() * 1000.0;
        cb
    }

    fn ui(&mut self, ui: &mut Ui, frame: &SceneFrame) {
        let t0 = Instant::now();
        self.ui_layer(ui, frame);
        self.cost.ui = t0.elapsed().as_secs_f32() * 1000.0;
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) -> bool {
        self.handle_key(key, modifiers)
    }
}

impl VisualEngine {
    fn ui_layer(&mut self, ui: &mut Ui, frame: &SceneFrame) {
        let now = self.now();
        self.ppp = ui.ctx().pixels_per_point();
        let (pointer, any_key) = ui.input(|i| {
            (
                i.pointer.latest_pos(),
                i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Key { pressed: true, .. })),
            )
        });
        if (pointer.is_some() && pointer != self.pointer) || any_key {
            self.fade.poke(now);
        }
        self.pointer = pointer;
        let rect = ui.max_rect();
        let inset = if frame.strip_visible {
            ui::timeline::HEIGHT
        } else {
            0.0
        };
        let alpha = self.fade.alpha(now);
        if alpha > 0.0 {
            let info = overlay::Info {
                artist: frame.artist,
                title: frame.title,
                elapsed: frame.position.seconds(),
                duration: frame.duration,
                score: frame.score,
                bottom_inset: inset,
            };
            overlay::draw(ui.painter(), rect, alpha, &info, ACCENT);
        }
        if self.deck {
            self.deck_ui(ui, inset);
        }
        self.toasts
            .retain(|(_, t, err)| now - t < if *err { 8.0 } else { 2.5 });
        let mut y = rect.top() + 24.0;
        for (text, _, err) in &self.toasts {
            let color = if *err {
                Color32::from_rgb(255, 120, 110)
            } else {
                Color32::from_gray(230)
            };
            let galley = ui.painter().layout(
                text.clone(),
                egui::FontId::monospace(13.0),
                color,
                rect.width() - 96.0,
            );
            let r =
                Rect::from_min_size(egui::pos2(rect.left() + 24.0, y), galley.size()).expand(8.0);
            ui.painter()
                .rect_filled(r, 6.0, Color32::from_black_alpha(190));
            ui.painter()
                .galley(egui::pos2(rect.left() + 24.0, y), galley, color);
            y = r.bottom() + 8.0;
        }
        if self.gpu.is_none() {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "No GPU for visuals",
                egui::FontId::proportional(20.0),
                Color32::GRAY,
            );
        }
    }

    fn handle_key(&mut self, key: Key, modifiers: Modifiers) -> bool {
        match key {
            Key::D => self.deck = !self.deck,
            Key::M => self.mutate(modifiers.shift),
            Key::K => self.keep(),
            Key::Backspace => self.undo(),
            Key::Num1 => self.rate(1),
            Key::Num2 => self.rate(2),
            Key::Num3 => self.rate(3),
            Key::Num4 => self.rate(4),
            Key::Num5 => self.rate(5),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::tests::temp_dir;
    use audio::{PlayState, Position};

    fn engine(name: &str) -> (VisualEngine, PathBuf) {
        let dir = temp_dir(name);
        let mut e = VisualEngine::with_dir(&dir);
        e.ensure_loaded();
        (e, dir)
    }

    fn with_gpu(e: &mut VisualEngine) -> bool {
        let Some(gpu) = crate::gpu::tests::headless() else {
            return false;
        };
        e.gpu = Some(gpu);
        let ids: Vec<String> = e.scenes.keys().cloned().collect();
        for id in &ids {
            assert!(e.compile(id), "{id}");
        }
        e.a = e.first_look().map(Slot::new);
        e.refresh_infos();
        true
    }

    fn frame_at<'a>(secs: f64, bars: &'a [f32; 19]) -> SceneFrame<'a> {
        SceneFrame {
            position: Position {
                track: 1,
                frame: (secs * 48_000.0) as u64,
                sample_rate: 48_000,
                state: PlayState::Playing,
                discontinuity: false,
            },
            bars,
            artist: "M83",
            title: "Midnight City",
            score: None,
            duration: Some(243.0),
            strip_visible: false,
        }
    }

    #[test]
    fn loads_bundled_looks_and_starts_on_the_best_rated() {
        let (mut e, dir) = engine("engine-load");
        assert_eq!(e.scenes.len(), 4);
        assert_eq!(e.looks.len(), 8);
        if with_gpu(&mut e) {
            let first = e.current_look().unwrap();
            assert_eq!(e.looks[first].variant.name, "nave", "rating 5");
        }
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn frames_render_and_keys_mutate_keep_undo_rate() {
        let (mut e, dir) = engine("engine-keys");
        if !with_gpu(&mut e) {
            return;
        }
        let bars = [0.4; 19];
        let rect = Rect::from_min_size(egui::Pos2::ZERO, vec2(160.0, 90.0));
        e.ppp = 1.0;
        for i in 0..5 {
            assert!(e.paint(rect, &frame_at(i as f64 / 60.0, &bars)).is_some());
        }
        let before = e.current_look().unwrap();
        assert!(e.key(Key::M, Modifiers::NONE));
        let mutated = e.current_look().unwrap();
        assert_ne!(before, mutated);
        assert_eq!(
            e.looks[mutated].variant.parent.as_deref(),
            Some(e.looks[before].variant.name.as_str())
        );
        e.paint(rect, &frame_at(0.1, &bars));
        assert!(e.key(Key::K, Modifiers::NONE));
        let kept = e.current_look().unwrap();
        assert!(e.looks[kept].saved);
        let file = dir
            .join("variants")
            .join(&e.looks[kept].variant.scene)
            .join(format!("{}.ron", e.looks[kept].variant.name));
        assert!(file.exists(), "K saves a variant file");
        assert!(e.key(Key::Num4, Modifiers::NONE));
        assert!(
            std::fs::read_to_string(&file)
                .unwrap()
                .contains("rating: 4")
        );
        assert!(e.key(Key::Backspace, Modifiers::NONE));
        assert_eq!(
            e.looks[e.current_look().unwrap()].variant.name,
            e.looks[before].variant.name,
            "undo returns to the look before M"
        );
        assert!(e.key(Key::D, Modifiers::NONE) && e.deck);
        assert!(!e.key(Key::Q, Modifiers::NONE));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn broken_edit_keeps_the_last_good_scene_running() {
        let (mut e, dir) = engine("engine-reload");
        if !with_gpu(&mut e) {
            return;
        }
        let id = "julia_tunnel";
        let before = e.scenes[id].program.clone().unwrap();
        let wgsl = dir.join("scenes").join(id).join("scene.wgsl");
        let good = std::fs::read_to_string(&wgsl).unwrap();
        std::fs::write(
            &wgsl,
            good.replace("let r = length(uv);", "let r = length(uv)"),
        )
        .unwrap();
        e.apply_changes(Changes {
            scenes: [id.to_string()].into(),
            ..Default::default()
        });
        assert!(
            Rc::ptr_eq(&before, e.scenes[id].program.as_ref().unwrap()),
            "old program kept"
        );
        assert!(
            e.toasts
                .iter()
                .any(|(t, _, err)| *err && t.contains("scene.wgsl")),
            "{:?}",
            e.toasts
        );
        std::fs::write(&wgsl, good.replace("0.08", "0.1")).unwrap();
        e.apply_changes(Changes {
            scenes: [id.to_string()].into(),
            ..Default::default()
        });
        assert!(
            !Rc::ptr_eq(&before, e.scenes[id].program.as_ref().unwrap()),
            "valid edit swapped in"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn crossfade_finishes_on_its_downbeat_and_reinit_rebuilds() {
        let (mut e, dir) = engine("engine-xfade");
        if !with_gpu(&mut e) {
            return;
        }
        let bars = [0.2; 19];
        let rect = Rect::from_min_size(egui::Pos2::ZERO, vec2(64.0, 64.0));
        e.ppp = 1.0;
        e.paint(rect, &frame_at(0.0, &bars));
        let target = (0..e.looks.len())
            .find(|&i| e.scene_of(i) != e.scene_of(e.current_look().unwrap()))
            .unwrap();
        e.signals.beats = 0.0;
        e.show(target, TransitionKind::Crossfade, 4.0);
        assert!(e.b.is_some());
        // 120 BPM nominal: 4 beats = 2 s.
        for i in 1..=150 {
            e.last_frame = Some(e.now() - 1.0 / 60.0); // as if frames came at 60 fps
            e.paint(rect, &frame_at(i as f64 / 60.0, &bars));
        }
        assert!(
            e.b.is_none() && e.current_look() == Some(target),
            "B became A"
        );
        // Re-init (device change): all GPU resources are rebuilt and frames keep coming.
        let gpu = crate::gpu::tests::headless().unwrap();
        e.gpu = Some(gpu);
        e.targets = None;
        let ids: Vec<String> = e.scenes.keys().cloned().collect();
        for id in &ids {
            e.compile(id);
        }
        for s in [&mut e.a, &mut e.b].into_iter().flatten() {
            s.layer = None;
        }
        assert!(e.paint(rect, &frame_at(3.0, &bars)).is_some());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn visuals_lead_the_clock_by_the_presentation_delay() {
        let p = Position {
            track: 1,
            frame: 48_000,
            sample_rate: 48_000,
            state: PlayState::Playing,
            discontinuity: false,
        };
        assert!((presented(p, 1.0 / 60.0).seconds() - (1.0 + 1.0 / 60.0)).abs() < 1e-4);
        let paused = Position {
            state: PlayState::Paused,
            ..p
        };
        assert_eq!(
            presented(paused, 1.0 / 60.0).frame,
            48_000,
            "a paused picture stays put"
        );
    }

    #[test]
    fn macros_follow_automation_and_speed_scales_motion() {
        let (mut e, dir) = engine("engine-macros");
        e.signals.tension = 0.8;
        e.update_macros();
        assert!(
            (e.macros.get(Macro::Stretch) - 0.8).abs() < 1e-6,
            "stretch follows tension"
        );
        e.macro_faders[Macro::Speed.index()].touch(1.0);
        e.update_macros();
        assert_eq!(e.macros.speed(), 4.0);
        e.set_macro_auto(Macro::Chaos, Value::Const(1.0), 4.0);
        e.signals.beats = 2.0;
        e.update_macros();
        assert!(
            (e.macros.get(Macro::Chaos) - 0.75).abs() < 1e-6,
            "glides over 4 beats"
        );
        let row = BenchRow {
            name: "flame".into(),
            dt: vec![1.0 / 60.0; 10],
            scale: vec![1.0; 10],
            ..Default::default()
        };
        let report = bench_report(&[row]);
        assert!(
            report.contains("flame") && report.contains("60.0"),
            "{report}"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_kick_at_12_5_s_hits_on_frame_750_at_60_fps() {
        let dir = temp_dir("engine-kick");
        let mut e = VisualEngine::with_dir(&dir);
        let mut score = analysis::SongScore {
            beats: (0..80).map(|i| i as f64 * 0.5).collect(),
            ..Default::default()
        };
        score.coverage.insert(0.0, 40.0);
        score.events.kick = vec![12.5];
        let bars = [0.0; 19];
        let at = |n: u64| -> (SceneFrame<'_>, Tick) {
            let t = n as f64 / 60.0;
            let mut f = frame_at(t, &bars);
            f.score = Some(&score);
            (
                f,
                Tick {
                    now: t,
                    dt: 1.0 / 60.0,
                    lead_secs: 0.0,
                },
            )
        };
        for n in 700..750 {
            let (f, tick) = at(n);
            e.step(&f, tick);
        }
        assert!(e.since[0] > 1.0, "no kick yet on frame 749: {}", e.since[0]);
        let (f, tick) = at(750);
        e.step(&f, tick);
        assert!(
            e.since[0] < 1e-6,
            "the kick lands on frame 750: {}",
            e.since[0]
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
