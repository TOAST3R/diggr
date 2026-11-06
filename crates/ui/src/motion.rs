//! Motion: how things appear and disappear, shared by the whole UI so it feels like one
//! instrument rather than a set of screens.
//!
//! Motion is drawing only. Whatever opens is laid out at its final place from its first frame,
//! so it takes clicks there at once; the motion moves and fades its *shapes*, after egui has
//! placed them. Whatever closes has already done what closing does; its last look is kept and
//! faded out in a layer that is only painted, so it can never take a click.
//!
//! Every popup, menu and window in egui draws into a layer of its own, so one pass after the
//! frame's UI ([`Motion::end_frame`]) finds the layers that appeared or went away and animates
//! them: no call site has to opt in. Separate OS windows can't be faded by the OS from here, so
//! they fade their content in ([`Motion::fade`]).
//!
//! With reduce motion on, everything is at its end state at once, as before this module.

use std::collections::HashMap;

use egui::emath::TSTransform;
use egui::epaint::{ClippedShape, Color32, Shape};
use egui::layers::ShapeIdx;
use egui::{Context, Id, LayerId, Order, Rect, Vec2, vec2};

/// Presses and hovers, menus opening.
pub const FAST: f32 = 0.12;
/// Dialogs and windows.
pub const MEDIUM: f32 = 0.18;
/// Panels.
pub const SLOW: f32 = 0.24;

/// How far a menu rises into place, in points.
const RISE: f32 = 4.0;
/// How far a panel slides into place, in points.
const SLIDE: f32 = 12.0;
/// The size a dialog grows from.
const DIALOG_FROM: f32 = 0.96;

/// `t` (0..=1) eased out: fast at first, arriving softly.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// `t` (0..=1) eased in and out.
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// How a layer moves as it appears (`p` 0 → 1) and disappears (1 → 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// Menus and popups: fade and rise a few points.
    Popup,
    /// Dialogs: fade and grow from a little smaller, about their centre.
    Dialog,
    /// A panel dropping from a control: fade and slide into place.
    Panel,
    /// Only fade (the dark layer behind the record card).
    Fade,
}

impl Kind {
    /// Opening and closing times, in seconds.
    pub fn durations(self) -> (f32, f32) {
        match self {
            Kind::Popup => (FAST, 0.08),
            Kind::Dialog | Kind::Fade => (MEDIUM, 0.12),
            Kind::Panel => (SLOW, 0.16),
        }
    }

    /// Where the shapes are drawn at progress `p`, for a layer at `rect`.
    pub fn transform(self, p: f32, rect: Rect) -> TSTransform {
        let rest = 1.0 - p.clamp(0.0, 1.0);
        match self {
            Kind::Popup => TSTransform::from_translation(vec2(0.0, RISE * rest)),
            Kind::Panel => TSTransform::from_translation(vec2(0.0, SLIDE * rest)),
            Kind::Dialog => {
                let s = DIALOG_FROM + (1.0 - DIALOG_FROM) * (1.0 - rest);
                let c = rect.center().to_vec2();
                TSTransform::from_translation(c)
                    * TSTransform::from_scaling(s)
                    * TSTransform::from_translation(-c)
            }
            Kind::Fade => TSTransform::IDENTITY,
        }
    }
}

/// A value chasing a target like a mass on a spring, for things that follow a moving target.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spring {
    pub x: f32,
    pub v: f32,
}

impl Spring {
    pub fn at(x: f32) -> Self {
        Self { x, v: 0.0 }
    }

    /// Advances `dt` seconds toward `target` with natural frequency `omega` (rad/s) and damping
    /// ratio `zeta` (1 is critically damped: no overshoot). Returns whether it has settled, in
    /// which case it sits exactly on the target.
    pub fn step(&mut self, target: f32, dt: f32, omega: f32, zeta: f32) -> bool {
        // Small fixed steps keep it stable whatever the frame time.
        const H: f32 = 1.0 / 480.0;
        let mut left = dt.clamp(0.0, 0.1);
        while left > 0.0 {
            let h = left.min(H);
            let a = -omega * omega * (self.x - target) - 2.0 * zeta * omega * self.v;
            self.v += a * h;
            self.x += self.v * h;
            left -= h;
        }
        let scale = target.abs().max(1.0);
        if (self.x - target).abs() < 1e-3 * scale && self.v.abs() < 1e-2 * scale {
            *self = Self::at(target);
            true
        } else {
            false
        }
    }
}

/// A layer seen open: how far it has opened, and its last look (for fading it out).
struct Open {
    kind: Kind,
    /// Linear progress, 0..=1.
    t: f32,
    frame: u64,
    rect: Rect,
    look: Vec<ClippedShape>,
}

/// A layer that closed, fading out from its last look.
struct Ghost {
    layer: LayerId,
    kind: Kind,
    /// Linear progress, from where it was down to 0.
    t: f32,
    rect: Rect,
    look: Vec<ClippedShape>,
}

/// The app's motion: transitions by id, the layers being opened, the ghosts of closed ones.
pub struct Motion {
    reduce: bool,
    now: f64,
    dt: f32,
    frame: u64,
    started: bool,
    animating: bool,
    kinds: HashMap<Id, Kind>,
    fades: HashMap<Id, (f32, f64)>,
    open: HashMap<LayerId, Open>,
    ghosts: Vec<Ghost>,
}

impl Default for Motion {
    fn default() -> Self {
        Self::new()
    }
}

impl Motion {
    pub fn new() -> Self {
        Self {
            reduce: false,
            now: 0.0,
            dt: 0.0,
            frame: 0,
            started: false,
            animating: false,
            kinds: HashMap::new(),
            fades: HashMap::new(),
            open: HashMap::new(),
            ghosts: Vec::new(),
        }
    }

    /// Animates the layer of the area `id` as `kind` (by default, a popup or menu in the
    /// foreground moves as [`Kind::Popup`], a window as [`Kind::Dialog`]).
    pub fn set_kind(&mut self, id: Id, kind: Kind) {
        self.kinds.insert(id, kind);
    }

    /// Starts a frame at the context's time. Reduce motion makes everything jump.
    pub fn begin_frame(&mut self, ctx: &Context, reduce: bool) {
        let now = ctx.input(|i| i.time);
        self.dt = if self.started {
            (now - self.now).clamp(0.0, 0.1) as f32
        } else {
            0.0
        };
        self.now = now;
        self.started = true;
        self.frame += 1;
        self.animating = false;
        if reduce != self.reduce {
            self.reduce = reduce;
            self.open.clear();
            self.ghosts.clear();
        }
        // egui's own fade-in of areas would fade them twice; this module does it.
        if ctx.global_style().animation_time != 0.0 {
            ctx.all_styles_mut(|s| s.animation_time = 0.0);
        }
    }

    /// Whether something is still moving: the app then repaints at the display's rate.
    pub fn animating(&self) -> bool {
        self.animating
    }

    /// Progress (0 closed, 1 open, eased) of whatever `key` names, moving toward `open` over
    /// `dur_in` or `dur_out` seconds. Turning around midway continues from where it is.
    pub fn fade(&mut self, key: Id, open: bool, dur_in: f32, dur_out: f32) -> f32 {
        let target = if open { 1.0 } else { 0.0 };
        if self.reduce {
            self.fades.remove(&key);
            return target;
        }
        let (t, last) = self.fades.entry(key).or_insert((1.0 - target, self.now));
        let dt = (self.now - *last).clamp(0.0, 0.1) as f32;
        *last = self.now;
        *t = if open {
            (*t + dt / dur_in.max(1e-3)).min(1.0)
        } else {
            (*t - dt / dur_out.max(1e-3)).max(0.0)
        };
        if *t != target {
            self.animating = true;
        }
        ease_out_cubic(*t)
    }

    /// Forgets `key`'s progress: the next [`Self::fade`] starts from closed.
    pub fn reset(&mut self, key: Id) {
        self.fades.remove(&key);
    }

    /// After the frame's UI: animates every popup and window layer that opened, and fades out
    /// the ones that closed. Returns whether anything is still moving.
    pub fn end_frame(&mut self, ctx: &Context) -> bool {
        if self.reduce {
            return self.animating;
        }
        let visible = ctx.memory(|m| m.areas().visible_layer_ids());
        let mut drawn: Vec<LayerId> = visible
            .into_iter()
            .filter(|l| matches!(l.order, Order::Middle | Order::Foreground))
            .filter(|l| ctx.graphics(|g| g.get(*l).is_some_and(|p| !p.is_empty())))
            .collect();
        drawn.sort_by_key(|l| l.id.value());

        for layer in drawn {
            let rect = ctx
                .memory(|m| m.area_rect(layer.id))
                .unwrap_or(Rect::NOTHING);
            let kind = self
                .kinds
                .get(&layer.id)
                .copied()
                .unwrap_or(match layer.order {
                    Order::Middle => Kind::Dialog,
                    _ => Kind::Popup,
                });
            // Opening again while fading out: carry on from the ghost's progress.
            let from = self
                .ghosts
                .iter()
                .position(|g| g.layer == layer)
                .map(|i| self.ghosts.swap_remove(i).t);
            let dt = self.dt;
            let frame = self.frame;
            let open = self.open.entry(layer).or_insert(Open {
                kind,
                t: from.unwrap_or(0.0),
                frame,
                rect,
                look: Vec::new(),
            });
            if open.frame != frame {
                open.t = (open.t + dt / kind.durations().0).min(1.0);
            }
            open.frame = frame;
            open.kind = kind;
            open.rect = rect;
            open.look = ctx.graphics(|g| {
                g.get(layer)
                    .map(|p| p.all_entries().cloned().collect())
                    .unwrap_or_default()
            });
            let p = ease_out_cubic(open.t);
            if p < 1.0 {
                self.animating = true;
                let tr = kind.transform(p, rect);
                ctx.graphics_mut(|g| {
                    if let Some(list) = g.get_mut(layer) {
                        list.transform(tr);
                        fade_list(list, p);
                    }
                });
            }
        }

        // What wasn't drawn this frame has closed: keep its look, fading out.
        let frame = self.frame;
        let closed: Vec<LayerId> = self
            .open
            .iter()
            .filter(|(_, o)| o.frame != frame)
            .map(|(l, _)| *l)
            .collect();
        for layer in closed {
            if let Some(o) = self.open.remove(&layer)
                && o.t > 0.0
                && !o.look.is_empty()
            {
                self.ghosts.push(Ghost {
                    layer,
                    kind: o.kind,
                    t: o.t,
                    rect: o.rect,
                    look: o.look,
                });
            }
        }
        let dt = self.dt;
        self.ghosts.retain_mut(|g| {
            g.t -= dt / g.kind.durations().1;
            g.t > 0.0
        });
        for g in &self.ghosts {
            let p = ease_out_cubic(g.t);
            let tr = g.kind.transform(p, g.rect);
            let paint = LayerId::new(g.layer.order, g.layer.id.with("motion_ghost"));
            ctx.graphics_mut(|gr| {
                let list = gr.entry(paint);
                for cs in &g.look {
                    let mut cs = cs.clone();
                    cs.transform(tr);
                    fade_shape(&mut cs.shape, p);
                    list.add(cs.clip_rect, cs.shape);
                }
            });
            self.animating = true;
        }
        self.animating
    }

    /// How many layers are fading out (for tests).
    pub fn closing(&self) -> usize {
        self.ghosts.len()
    }
}

/// Multiplies every colour in `list` by `opacity`.
fn fade_list(list: &mut egui::layers::PaintList, opacity: f32) {
    let n = list.next_idx().0;
    for i in 0..n {
        list.mutate_shape(ShapeIdx(i), |cs| fade_shape(&mut cs.shape, opacity));
    }
}

fn fade_shape(shape: &mut Shape, opacity: f32) {
    if opacity >= 1.0 {
        return;
    }
    egui::epaint::shape_transform::adjust_colors(shape, move |c| {
        if *c != Color32::PLACEHOLDER {
            *c = c.gamma_multiply(opacity);
        }
    });
}

/// The offset `kind` draws a layer at, at progress `p` (for tests and callers that draw
/// themselves).
pub fn offset(kind: Kind, p: f32) -> Vec2 {
    kind.transform(p, Rect::ZERO).translation
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `frames` frames of `dt` through `fade`, returning the progress after each.
    fn run(m: &mut Motion, key: Id, open: bool, frames: usize, dt: f64) -> Vec<f32> {
        (0..frames)
            .map(|_| {
                m.now += dt;
                m.fade(key, open, FAST, 0.08)
            })
            .collect()
    }

    #[test]
    fn a_transition_opens_in_its_time_and_closes_in_its_own() {
        let mut m = Motion::new();
        let key = Id::new("menu");
        let dt = 1.0 / 60.0;
        let p = run(&mut m, key, true, 10, dt);
        // 0.12 s at 60 fps: open on the 8th frame (±1), rising all the way.
        let first = p.iter().position(|&x| x == 1.0).unwrap();
        assert!((7..=8).contains(&first), "open after {first} frames: {p:?}");
        assert!(p.windows(2).all(|w| w[1] >= w[0]));
        assert!(!m.animating() || p.last() == Some(&1.0));
        m.animating = false;
        let q = run(&mut m, key, false, 10, dt);
        let shut = q.iter().position(|&x| x == 0.0).unwrap();
        assert!((4..=5).contains(&shut), "closed after {shut} frames: {q:?}");
    }

    #[test]
    fn turning_around_midway_continues_from_where_it_is() {
        let mut m = Motion::new();
        let key = Id::new("x");
        let p = run(&mut m, key, true, 3, 1.0 / 60.0);
        let q = run(&mut m, key, false, 1, 1.0 / 60.0);
        assert!(q[0] < p[2] && q[0] > 0.0, "{p:?} then {q:?}");
    }

    #[test]
    fn reduce_motion_jumps_at_once() {
        let mut m = Motion::new();
        m.reduce = true;
        let key = Id::new("menu");
        assert_eq!(m.fade(key, true, FAST, 0.08), 1.0);
        assert_eq!(m.fade(key, false, FAST, 0.08), 0.0);
        assert!(!m.animating());
    }

    #[test]
    fn a_critically_damped_spring_settles_without_overshoot() {
        let mut s = Spring::at(0.0);
        let mut peak = 0.0f32;
        let mut settled_at = None;
        for i in 0..120 {
            let done = s.step(1.0, 1.0 / 60.0, 30.0, 1.0);
            peak = peak.max(s.x);
            if done {
                settled_at = Some(i);
                break;
            }
        }
        assert!(settled_at.is_some(), "it settles");
        assert!(peak <= 1.05, "overshot to {peak}");
        assert_eq!(s.x, 1.0);
    }

    #[test]
    fn an_underdamped_spring_overshoots_a_little_and_settles() {
        let mut s = Spring::at(0.0);
        let mut peak = 0.0f32;
        for _ in 0..120 {
            s.step(1.0, 1.0 / 60.0, 25.0, 0.7);
            peak = peak.max(s.x);
        }
        assert!(peak > 1.0 && peak < 1.1, "peak {peak}");
        assert!((s.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn kinds_move_as_described() {
        assert_eq!(offset(Kind::Popup, 0.0), vec2(0.0, RISE));
        assert_eq!(offset(Kind::Popup, 1.0), Vec2::ZERO);
        assert_eq!(offset(Kind::Panel, 0.0), vec2(0.0, SLIDE));
        let r = Rect::from_min_size(egui::pos2(100.0, 100.0), vec2(200.0, 100.0));
        let t = Kind::Dialog.transform(0.0, r);
        assert!((t.scaling - DIALOG_FROM).abs() < 1e-6);
        // About its centre: the centre stays put.
        assert!((t * r.center() - r.center()).length() < 1e-3);
        assert_eq!(Kind::Dialog.transform(1.0, r), TSTransform::IDENTITY);
    }

    #[test]
    fn easing_ends_where_it_should() {
        for f in [ease_out_cubic, ease_in_out] {
            assert_eq!(f(0.0), 0.0);
            assert_eq!(f(1.0), 1.0);
            assert!(f(0.5) > 0.0 && f(0.5) < 1.0);
        }
    }
}
