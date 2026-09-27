//! Modulation: routes from music signals through shaper chains to scene parameters, global
//! macros, and the order in which base values, faders, macros and routes combine.

use analysis::SectionKind;
use serde::{Deserialize, Serialize};

use crate::signals::Signals;

/// Where a route's value comes from. Triggers give 1.0 in the frame they fire, else 0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Source {
    Kick,
    Snare,
    Hat,
    BeatHit,
    Downbeat,
    SectionChange,
    Drop,
    BeatPhase,
    BarPhase,
    PhrasePhase,
    SectionProgress,
    Energy,
    Bass,
    Mid,
    Treble,
    Brightness,
    Tension,
    /// Beats until the next known drop (large when none).
    DropIn,
    Const(f32),
}

impl Source {
    fn trigger(self, s: &Signals) -> Option<bool> {
        let t = &s.triggers;
        Some(match self {
            Source::Kick => t.kick.fired(),
            Source::Snare => t.snare.fired(),
            Source::Hat => t.hat.fired(),
            Source::BeatHit => t.beat.fired(),
            Source::Downbeat => t.downbeat.fired(),
            Source::SectionChange => t.section_change.fired(),
            Source::Drop => t.drop.fired(),
            _ => return None,
        })
    }

    pub fn value(self, s: &Signals) -> f32 {
        if let Some(fired) = self.trigger(s) {
            return fired as u8 as f32;
        }
        match self {
            Source::BeatPhase => s.beat,
            Source::BarPhase => s.bar,
            Source::PhrasePhase => s.phrase,
            Source::SectionProgress => s.section_progress,
            Source::Energy => s.energy,
            Source::Bass => s.bass,
            Source::Mid => s.mid,
            Source::Treble => s.treble,
            Source::Brightness => s.brightness,
            Source::Tension => s.tension,
            Source::DropIn => s.drop_in,
            Source::Const(c) => c,
            _ => 0.0,
        }
    }
}

/// A duration in beats (follows the tempo) or milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Time {
    Beats(f32),
    Ms(f32),
}

impl Time {
    pub fn seconds(self, tempo_bpm: f32) -> f32 {
        match self {
            Time::Beats(b) => b * 60.0 / tempo_bpm.max(1.0),
            Time::Ms(ms) => ms / 1000.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Curve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Exp,
    Step,
    Sine,
}

impl Curve {
    pub fn apply(self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        match self {
            Curve::Linear => x,
            Curve::EaseIn => x * x,
            Curve::EaseOut => 1.0 - (1.0 - x) * (1.0 - x),
            Curve::EaseInOut => x * x * (3.0 - 2.0 * x),
            Curve::Exp => (2f32.powf(8.0 * x) - 1.0) / 255.0,
            Curve::Step => (x >= 0.5) as u8 as f32,
            Curve::Sine => 0.5 - 0.5 * (std::f32::consts::TAU * x).cos(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Period {
    Beat,
    Bar,
    Phrase,
}

impl Period {
    fn beats(self) -> f64 {
        match self {
            Period::Beat => 1.0,
            Period::Bar => 4.0,
            Period::Phrase => 32.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Shaper {
    /// Punchy decay: jumps up when the input rises past 0.5 (a trigger), then falls.
    Envelope {
        attack: Time,
        release: Time,
    },
    /// Glide toward the input.
    Smooth(Time),
    Curve(Curve),
    /// 0..1 → lo..hi.
    Range(f32, f32),
    /// Only take a new value every `beats` beats.
    Quantize(f32),
    /// Hold the value, sampling the input only when `on` fires.
    SampleHold(Source),
    /// A deterministic random value, new every period (ignores the input).
    Random(Period),
    /// A step pattern (1 = on) clocked every `step` beats (ignores the input).
    StepSeq {
        steps: Vec<u8>,
        step: f32,
    },
    /// Pass only during sections of these kinds; otherwise the route has no effect.
    Gate(Vec<SectionKind>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Mode {
    Set,
    #[default]
    Add,
    Mul,
}

fn one() -> f32 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub source: Source,
    #[serde(default)]
    pub shapers: Vec<Shaper>,
    #[serde(default)]
    pub mode: Mode,
    /// Parameter name, optionally with a component: `"zoom"`, `"julia_c.x"`.
    pub target: String,
    #[serde(default = "one")]
    pub depth: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct ShaperState {
    level: f32,
    prev_in: f32,
    smooth: Option<f32>,
    held: Option<f32>,
    q_index: Option<i64>,
}

/// Per-route memory between frames.
#[derive(Debug, Clone, Default)]
pub struct RouteState {
    shapers: Vec<ShaperState>,
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Deterministic 0..1 from (seed, route, shaper position, period index).
pub fn random01(seed: u64, route_id: u64, shaper: usize, index: i64) -> f32 {
    let h = splitmix(
        seed ^ splitmix(route_id.wrapping_mul(31) ^ (shaper as u64) << 48) ^ splitmix(index as u64),
    );
    (h >> 40) as f32 / (1u64 << 24) as f32
}

impl Route {
    /// This frame's value, or `None` when a gate closes the route.
    pub fn eval(
        &self,
        st: &mut RouteState,
        s: &Signals,
        seed: u64,
        route_id: u64,
        dt: f32,
    ) -> Option<f32> {
        st.shapers
            .resize(self.shapers.len(), ShaperState::default());
        let mut v = self.source.value(s);
        for (k, sh) in self.shapers.iter().enumerate() {
            let state = &mut st.shapers[k];
            v = match sh {
                Shaper::Envelope { attack, release } => {
                    let rising = v >= 0.5 && state.prev_in < 0.5;
                    state.prev_in = v;
                    let (a, r) = (
                        attack.seconds(s.tempo_bpm),
                        release.seconds(s.tempo_bpm).max(1e-3),
                    );
                    if rising {
                        state.level = if a <= dt { 1.0 } else { state.level + dt / a };
                    } else {
                        state.level = (state.level - dt / r).max(0.0);
                    }
                    state.level
                }
                Shaper::Smooth(t) => {
                    let tau = t.seconds(s.tempo_bpm).max(1e-4);
                    let cur = state.smooth.unwrap_or(v);
                    let next = cur + (v - cur) * (1.0 - (-dt / tau).exp());
                    state.smooth = Some(next);
                    next
                }
                Shaper::Curve(c) => c.apply(v),
                Shaper::Range(lo, hi) => lo + v * (hi - lo),
                Shaper::Quantize(beats) => {
                    let idx = (s.beats / (*beats as f64).max(1e-3)).floor() as i64;
                    if state.q_index != Some(idx) {
                        state.q_index = Some(idx);
                        state.held = Some(v);
                    }
                    state.held.unwrap_or(v)
                }
                Shaper::SampleHold(on) => {
                    if state.held.is_none() || on.value(s) >= 0.5 {
                        state.held = Some(v);
                    }
                    state.held.unwrap_or(v)
                }
                Shaper::Random(p) => {
                    random01(seed, route_id, k, (s.beats / p.beats()).floor() as i64)
                }
                Shaper::StepSeq { steps, step } => {
                    if steps.is_empty() {
                        0.0
                    } else {
                        let i = (s.beats / (*step as f64).max(1e-3)).floor() as i64;
                        (steps[i.rem_euclid(steps.len() as i64) as usize] > 0) as u8 as f32
                    }
                }
                Shaper::Gate(kinds) => {
                    if !s.section_kind.is_some_and(|k| kinds.contains(&k)) {
                        return None;
                    }
                    v
                }
            };
        }
        Some(v)
    }
}

/// The six global macros; every scene maps them onto its own parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Macro {
    Intensity,
    Chaos,
    Stretch,
    Speed,
    Hue,
    Feedback,
}

impl Macro {
    pub const ALL: [Macro; 6] = [
        Macro::Intensity,
        Macro::Chaos,
        Macro::Stretch,
        Macro::Speed,
        Macro::Hue,
        Macro::Feedback,
    ];

    pub fn index(self) -> usize {
        Macro::ALL.iter().position(|&m| m == self).expect("listed")
    }

    pub fn name(self) -> &'static str {
        match self {
            Macro::Intensity => "INT",
            Macro::Chaos => "CHAOS",
            Macro::Stretch => "STR",
            Macro::Speed => "SPD",
            Macro::Hue => "HUE",
            Macro::Feedback => "FDBK",
        }
    }
}

/// Speed macro steps; the fader snaps to one of these.
pub const SPEEDS: [f32; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

/// Snaps a 0..1 speed fader position to the nearest step and returns (position, multiplier).
pub fn snap_speed(v: f32) -> (f32, f32) {
    let i = (v.clamp(0.0, 1.0) * 4.0).round() as usize;
    (i as f32 / 4.0, SPEEDS[i])
}

/// Macro values 0..1 (0.5 is neutral).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Macros(pub [f32; 6]);

impl Default for Macros {
    fn default() -> Self {
        Self([0.5; 6])
    }
}

impl Macros {
    pub fn get(&self, m: Macro) -> f32 {
        self.0[m.index()]
    }

    pub fn set(&mut self, m: Macro, v: f32) {
        self.0[m.index()] = v.clamp(0.0, 1.0);
    }

    /// Musical-time multiplier from the (snapped) speed macro.
    pub fn speed(&self) -> f32 {
        snap_speed(self.get(Macro::Speed)).1
    }

    /// Route depth multiplier from intensity (0 → none, 0.5 → as authored, 1 → double).
    pub fn intensity_gain(&self) -> f32 {
        2.0 * self.get(Macro::Intensity)
    }
}

/// How a macro moves a parameter: `amount` × (macro − 0.5) × the parameter's range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroMap {
    #[serde(rename = "macro")]
    pub which: Macro,
    pub param: String,
    pub amount: f32,
}

/// A parameter slot: up to 4 components, with bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub name: String,
    pub components: usize,
    pub min: [f32; 4],
    pub max: [f32; 4],
}

/// Where a route or macro writes: a parameter and optionally one component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub param: usize,
    pub component: Option<usize>,
}

pub fn resolve(slots: &[Slot], target: &str) -> Option<Target> {
    let (name, comp) = match target.split_once('.') {
        Some((n, c)) => (
            n,
            Some(match c {
                "x" | "r" => 0,
                "y" | "g" => 1,
                "z" | "b" => 2,
                "w" | "a" => 3,
                _ => return None,
            }),
        ),
        None => (target, None),
    };
    let param = slots.iter().position(|s| s.name == name)?;
    if comp.is_some_and(|c| c >= slots[param].components) {
        return None;
    }
    Some(Target {
        param,
        component: comp,
    })
}

/// One route's contribution this frame.
#[derive(Debug, Clone, Copy)]
pub struct Contribution {
    pub target: Target,
    pub mode: Mode,
    pub value: f32,
    pub depth: f32,
}

/// Final parameter values: variant base → manual fader override → macros → routes → clamp.
pub fn compose(
    slots: &[Slot],
    base: &[[f32; 4]],
    manual: &[Option<[f32; 4]>],
    macros: &Macros,
    macro_maps: &[(Target, Macro, f32)],
    routes: &[Contribution],
) -> Vec<[f32; 4]> {
    let mut out: Vec<[f32; 4]> = slots
        .iter()
        .enumerate()
        .map(|(i, _)| {
            manual
                .get(i)
                .copied()
                .flatten()
                .unwrap_or(base.get(i).copied().unwrap_or([0.0; 4]))
        })
        .collect();
    let comps = |t: &Target, n: usize| -> std::ops::Range<usize> {
        match t.component {
            Some(c) => c..c + 1,
            None => 0..n,
        }
    };
    for &(t, m, amount) in macro_maps {
        let slot = &slots[t.param];
        for c in comps(&t, slot.components) {
            out[t.param][c] += amount * (macros.get(m) - 0.5) * (slot.max[c] - slot.min[c]);
        }
    }
    let gain = macros.intensity_gain();
    for r in routes {
        let slot = &slots[r.target.param];
        for c in comps(&r.target, slot.components) {
            let x = &mut out[r.target.param][c];
            let d = r.depth * gain;
            *x = match r.mode {
                Mode::Set => *x + (r.value - *x) * d.min(1.0),
                Mode::Add => *x + r.value * d,
                Mode::Mul => *x * (1.0 + (r.value - 1.0) * d),
            };
        }
    }
    for (v, slot) in out.iter_mut().zip(slots) {
        for (c, x) in v.iter_mut().enumerate().take(slot.components) {
            *x = x.clamp(slot.min[c], slot.max[c]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{Trigger, Triggers};

    fn sig(beats: f64) -> Signals {
        Signals {
            playing: true,
            beats,
            tempo_bpm: 120.0,
            ..Default::default()
        }
    }

    fn fired(mut s: Signals, f: impl Fn(&mut Triggers) -> &mut Trigger) -> Signals {
        *f(&mut s.triggers) = Trigger { count: 1, ago: 0.0 };
        s
    }

    fn route(source: Source, shapers: Vec<Shaper>) -> Route {
        Route {
            source,
            shapers,
            mode: Mode::Add,
            target: "zoom".into(),
            depth: 1.0,
        }
    }

    #[test]
    fn kick_envelope_jumps_and_decays_in_120_ms() {
        let r = route(
            Source::Kick,
            vec![
                Shaper::Envelope {
                    attack: Time::Ms(5.0),
                    release: Time::Ms(120.0),
                },
                Shaper::Range(0.0, 0.35),
            ],
        );
        let mut st = RouteState::default();
        let dt = 1.0 / 60.0;
        let hit = r
            .eval(&mut st, &fired(sig(0.0), |t| &mut t.kick), 1, 1, dt)
            .unwrap();
        assert!((hit - 0.35).abs() < 1e-6, "full punch on the kick frame");
        let mut v = hit;
        let mut frames = 0;
        while v > 0.0 {
            v = r.eval(&mut st, &sig(0.0), 1, 1, dt).unwrap();
            frames += 1;
        }
        assert!(
            (6..=8).contains(&frames),
            "back to base after ~120 ms ({frames} frames)"
        );
    }

    #[test]
    fn per_bar_sample_and_hold_changes_only_on_downbeats() {
        let r = route(
            Source::Const(0.0),
            vec![
                Shaper::Random(Period::Beat),
                Shaper::SampleHold(Source::Downbeat),
            ],
        );
        let mut st = RouteState::default();
        let mut values = Vec::new();
        for beat in 0..12 {
            let mut s = sig(beat as f64 + 0.5);
            if beat % 4 == 0 {
                s = fired(s, |t| &mut t.downbeat);
            }
            values.push(r.eval(&mut st, &s, 7, 3, 0.5).unwrap());
        }
        for bar in values.chunks(4) {
            assert!(
                bar.iter().all(|&v| v == bar[0]),
                "held for the bar: {bar:?}"
            );
        }
        assert_ne!(values[0], values[4], "a new value on the next downbeat");
    }

    #[test]
    fn step_sequencer_pattern() {
        let pattern = vec![1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0];
        let r = route(
            Source::Const(0.0),
            vec![Shaper::StepSeq {
                steps: pattern,
                step: 0.25,
            }],
        );
        let mut st = RouteState::default();
        let on: Vec<usize> = (0..16)
            .filter(|&i| {
                r.eval(&mut st, &sig(i as f64 * 0.25 + 0.01), 1, 1, 0.01)
                    .unwrap()
                    > 0.5
            })
            .map(|i| i + 1)
            .collect();
        assert_eq!(on, [1, 4, 7, 11, 14]);
    }

    #[test]
    fn gate_closes_outside_listed_sections() {
        let r = route(
            Source::Const(1.0),
            vec![Shaper::Gate(vec![SectionKind::Drop])],
        );
        let mut st = RouteState::default();
        let mut s = sig(0.0);
        s.section_kind = Some(SectionKind::Breakdown);
        assert_eq!(r.eval(&mut st, &s, 1, 1, 0.01), None);
        s.section_kind = Some(SectionKind::Drop);
        assert_eq!(r.eval(&mut st, &s, 1, 1, 0.01), Some(1.0));
    }

    #[test]
    fn smooth_quantize_and_curves() {
        let r = route(Source::Energy, vec![Shaper::Smooth(Time::Beats(0.5))]);
        let mut st = RouteState::default();
        let mut s = sig(0.0);
        s.energy = 0.0;
        r.eval(&mut st, &s, 1, 1, 0.01);
        s.energy = 1.0;
        let a = r.eval(&mut st, &s, 1, 1, 0.1).unwrap(); // 0.5 beat = 0.25 s at 120 BPM
        assert!(a > 0.3 && a < 0.4, "{a}");
        let q = route(Source::Energy, vec![Shaper::Quantize(1.0)]);
        let mut qs = RouteState::default();
        let mut s = sig(0.1);
        s.energy = 0.2;
        assert_eq!(q.eval(&mut qs, &s, 1, 1, 0.01), Some(0.2));
        s.energy = 0.9;
        s.beats = 0.8;
        assert_eq!(
            q.eval(&mut qs, &s, 1, 1, 0.01),
            Some(0.2),
            "held within the beat"
        );
        s.beats = 1.1;
        assert_eq!(q.eval(&mut qs, &s, 1, 1, 0.01), Some(0.9));
        for c in [
            Curve::Linear,
            Curve::EaseIn,
            Curve::EaseOut,
            Curve::EaseInOut,
            Curve::Exp,
            Curve::Sine,
        ] {
            assert!(
                c.apply(0.0).abs() < 1e-6 && (c.apply(1.0) - 1.0).abs() < 1e-3 || c == Curve::Sine,
                "{c:?}"
            );
        }
    }

    #[test]
    fn randomness_is_deterministic_per_seed_and_period() {
        assert_eq!(random01(9, 2, 0, 17), random01(9, 2, 0, 17));
        assert_ne!(random01(9, 2, 0, 17), random01(9, 2, 0, 18));
        assert_ne!(random01(9, 2, 0, 17), random01(10, 2, 0, 17));
        let vals: Vec<f32> = (0..1000).map(|i| random01(1, 1, 0, i)).collect();
        let mean = vals.iter().sum::<f32>() / 1000.0;
        assert!((mean - 0.5).abs() < 0.05 && vals.iter().all(|v| (0.0..1.0).contains(v)));
    }

    fn slots() -> Vec<Slot> {
        vec![
            Slot {
                name: "zoom".into(),
                components: 1,
                min: [0.2, 0.0, 0.0, 0.0],
                max: [4.0, 0.0, 0.0, 0.0],
            },
            Slot {
                name: "julia_c".into(),
                components: 2,
                min: [-1.0, -1.0, 0.0, 0.0],
                max: [1.0, 1.0, 0.0, 0.0],
            },
        ]
    }

    #[test]
    fn composition_order_manual_then_macros_then_routes_then_clamp() {
        let s = slots();
        let zoom = resolve(&s, "zoom").unwrap();
        let kick = Contribution {
            target: zoom,
            mode: Mode::Add,
            value: 0.3,
            depth: 1.0,
        };
        // Spec: fader held at 2.0, kick route adds 0.3 → 2.3.
        let out = compose(
            &s,
            &[[1.0; 4], [0.0; 4]],
            &[Some([2.0, 0.0, 0.0, 0.0]), None],
            &Macros::default(),
            &[],
            &[kick],
        );
        assert!((out[0][0] - 2.3).abs() < 1e-6);
        // Without the kick it returns to the fader value.
        let out = compose(
            &s,
            &[[1.0; 4], [0.0; 4]],
            &[Some([2.0, 0.0, 0.0, 0.0]), None],
            &Macros::default(),
            &[],
            &[],
        );
        assert_eq!(out[0][0], 2.0);
        // Clamped to the declared range.
        let big = Contribution {
            target: zoom,
            mode: Mode::Add,
            value: 10.0,
            depth: 1.0,
        };
        assert_eq!(
            compose(
                &s,
                &[[1.0; 4], [0.0; 4]],
                &[],
                &Macros::default(),
                &[],
                &[big]
            )[0][0],
            4.0
        );
        // Macros move a parameter by amount × (macro − 0.5) × range; intensity scales routes.
        let mut m = Macros::default();
        m.set(Macro::Chaos, 1.0);
        let out = compose(
            &s,
            &[[1.0; 4], [0.0; 4]],
            &[],
            &m,
            &[(zoom, Macro::Chaos, 0.1)],
            &[],
        );
        assert!((out[0][0] - (1.0 + 0.1 * 0.5 * 3.8)).abs() < 1e-5);
        m.set(Macro::Intensity, 0.0);
        assert_eq!(
            compose(&s, &[[1.0; 4], [0.0; 4]], &[], &m, &[], &[kick])[0][0],
            1.0,
            "intensity 0 mutes routes"
        );
    }

    #[test]
    fn targets_and_components() {
        let s = slots();
        assert_eq!(
            resolve(&s, "julia_c.y"),
            Some(Target {
                param: 1,
                component: Some(1)
            })
        );
        assert_eq!(resolve(&s, "zoom.y"), None, "zoom has one component");
        assert_eq!(resolve(&s, "nope"), None);
        let c = Contribution {
            target: resolve(&s, "julia_c.x").unwrap(),
            mode: Mode::Set,
            value: 0.5,
            depth: 1.0,
        };
        let out = compose(
            &s,
            &[[1.0; 4], [0.0, 0.25, 0.0, 0.0]],
            &[],
            &Macros::default(),
            &[],
            &[c],
        );
        assert_eq!(out[1][..2], [0.5, 0.25]);
    }

    #[test]
    fn speed_snaps_to_musical_steps() {
        assert_eq!(snap_speed(0.5), (0.5, 1.0));
        assert_eq!(snap_speed(0.6), (0.5, 1.0));
        assert_eq!(snap_speed(0.7), (0.75, 2.0));
        assert_eq!(snap_speed(0.0), (0.0, 0.25));
        assert_eq!(snap_speed(1.0), (1.0, 4.0));
    }

    #[test]
    fn routes_parse_from_ron() {
        let text = "(source: Kick, shapers: [Envelope(attack: Ms(5), release: Ms(120)), Range(0.0, 0.35)], mode: Add, target: \"zoom\")";
        let r: Route = ron::from_str(text).unwrap();
        assert_eq!(r.depth, 1.0);
        assert_eq!(r.shapers.len(), 2);
        let g: Route =
            ron::from_str("(source: Const(1), shapers: [Gate([Drop, Build])], target: \"hue\")")
                .unwrap();
        assert_eq!(g.mode, Mode::Add);
    }
}
