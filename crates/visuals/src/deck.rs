//! Fader logic for the deck: AUTO (follow automation) vs MANUAL (the user's value), and the
//! musical RETURN glide back to automation that lands on a bar line.

/// How long a released fader holds before it is back on automation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Return {
    Beat,
    #[default]
    Bar,
    Bars4,
    Phrase,
    /// Latch: stay until re-armed.
    Never,
}

impl Return {
    pub const ALL: [Return; 5] = [
        Return::Beat,
        Return::Bar,
        Return::Bars4,
        Return::Phrase,
        Return::Never,
    ];

    pub fn beats(self) -> Option<f64> {
        match self {
            Return::Beat => Some(1.0),
            Return::Bar => Some(4.0),
            Return::Bars4 => Some(16.0),
            Return::Phrase => Some(32.0),
            Return::Never => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Return::Beat => "1 BEAT",
            Return::Bar => "1 BAR",
            Return::Bars4 => "4 BARS",
            Return::Phrase => "PHRASE",
            Return::Never => "∞",
        }
    }

    pub fn next(self) -> Return {
        Return::ALL[(Return::ALL.iter().position(|&r| r == self).unwrap() + 1) % Return::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Auto,
    /// Held by the user (`held`) or released and waiting to glide back.
    Manual {
        value: f32,
        held: bool,
        glide: Option<(f64, f64)>,
    },
}

/// One fader's mode. Values are whatever unit the fader controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fader {
    state: State,
    /// Overrides the deck's global RETURN.
    pub ret: Option<Return>,
}

impl Default for Fader {
    fn default() -> Self {
        Self {
            state: State::Auto,
            ret: None,
        }
    }
}

/// The first bar line at or after `beats`, given the bar phase at `now`.
fn bar_line_at_or_after(beats: f64, now: f64, bar_phase: f32) -> f64 {
    let origin = now - bar_phase as f64 * 4.0;
    origin + ((beats - origin) / 4.0 - 1e-9).ceil() * 4.0
}

impl Fader {
    pub fn is_manual(&self) -> bool {
        matches!(self.state, State::Manual { .. })
    }

    /// The user grabbed or moved the fader.
    pub fn touch(&mut self, value: f32) {
        self.state = State::Manual {
            value,
            held: true,
            glide: None,
        };
    }

    /// The user let go at musical time `now`: hold, then glide back to AUTO, arriving on the
    /// first bar line at least RETURN after release. The glide takes up to one bar.
    pub fn release(&mut self, now: f64, bar_phase: f32, global: Return) {
        if let State::Manual { value, .. } = self.state {
            let glide = self.ret.unwrap_or(global).beats().map(|r| {
                let land = bar_line_at_or_after(now + r, now, bar_phase);
                ((land - r.min(4.0)).max(now), land)
            });
            self.state = State::Manual {
                value,
                held: false,
                glide,
            };
        }
    }

    /// Back to automation now.
    pub fn rearm(&mut self) {
        self.state = State::Auto;
    }

    /// The fader's effective value at `now`, given what automation says.
    pub fn value(&mut self, auto: f32, now: f64) -> f32 {
        match self.state {
            State::Auto => auto,
            State::Manual {
                value,
                glide: Some((start, end)),
                held: false,
            } => {
                if now >= end {
                    self.state = State::Auto;
                    auto
                } else if now <= start {
                    value
                } else {
                    let t = ((now - start) / (end - start)) as f32;
                    value + (auto - value) * t * t * (3.0 - 2.0 * t)
                }
            }
            State::Manual { value, .. } => value,
        }
    }

    /// The manual value while MANUAL (for composing parameters), else `None`.
    pub fn manual(&mut self, auto: f32, now: f64) -> Option<f32> {
        let v = self.value(auto, now);
        self.is_manual().then_some(v)
    }
}

/// What a fader shows, all in 0..1 of its range.
pub struct FaderView<'a> {
    pub label: &'a str,
    /// Where the handle sits: the manual value, or automation.
    pub value: f32,
    /// The base (automation) value: a thin line.
    pub base: f32,
    /// After modulation: a bright tick.
    pub live: f32,
    pub manual: bool,
    /// A per-fader RETURN override, shown under the label.
    pub ret: Option<Return>,
    /// Shown instead of the percentage (e.g. "2×").
    pub readout: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FaderEvent {
    Touch(f32),
    Release,
    /// Right click: cycle this fader's RETURN override.
    CycleReturn,
    /// Double click: back to AUTO now.
    Rearm,
}

pub const FADER_W: f32 = 44.0;
pub const FADER_H: f32 = 150.0;

pub fn fader_ui(
    ui: &mut egui::Ui,
    id: egui::Id,
    v: &FaderView,
    accent: egui::Color32,
) -> Option<FaderEvent> {
    use egui::{Align2, Color32, FontId, Sense, Stroke, pos2, vec2};
    let (rect, _) = ui.allocate_exact_size(vec2(FADER_W, FADER_H), Sense::hover());
    let track = egui::Rect::from_min_max(
        pos2(rect.center().x - 3.0, rect.top() + 8.0),
        pos2(rect.center().x + 3.0, rect.bottom() - 34.0),
    );
    let resp = ui.interact(track.expand2(vec2(14.0, 8.0)), id, Sense::click_and_drag());
    let y_of = |t: f32| track.bottom() - t.clamp(0.0, 1.0) * track.height();
    let p = ui.painter();
    p.rect_filled(track, 3.0, Color32::from_gray(40));
    p.rect_filled(
        egui::Rect::from_min_max(pos2(track.left(), y_of(v.value)), track.max),
        3.0,
        accent.gamma_multiply(0.35),
    );
    p.line_segment(
        [
            pos2(track.left() - 7.0, y_of(v.base)),
            pos2(track.right() + 7.0, y_of(v.base)),
        ],
        Stroke::new(1.0, Color32::from_gray(160)),
    );
    let knob = egui::Rect::from_center_size(pos2(track.center().x, y_of(v.value)), vec2(26.0, 9.0));
    let knob_color = if v.manual {
        Color32::from_rgb(255, 200, 80)
    } else {
        Color32::from_gray(220)
    };
    p.rect_filled(knob, 2.0, knob_color);
    p.circle_filled(pos2(track.right() + 9.0, y_of(v.live)), 3.0, accent);
    let text = v
        .readout
        .clone()
        .unwrap_or_else(|| format!("{:.0}", v.value * 100.0));
    p.text(
        pos2(rect.center().x, rect.bottom() - 30.0),
        Align2::CENTER_TOP,
        text,
        FontId::monospace(10.0),
        Color32::from_gray(170),
    );
    p.text(
        pos2(rect.center().x, rect.bottom() - 18.0),
        Align2::CENTER_TOP,
        v.label,
        FontId::proportional(10.0),
        Color32::from_gray(230),
    );
    let mode = match (v.manual, v.ret) {
        (true, Some(r)) => format!("M·{}", r.name()),
        (true, None) => "MAN".into(),
        (false, Some(r)) => r.name().into(),
        (false, None) => String::new(),
    };
    if !mode.is_empty() {
        p.text(
            pos2(rect.center().x, rect.bottom() - 6.0),
            Align2::CENTER_TOP,
            mode,
            FontId::proportional(8.0),
            Color32::from_gray(150),
        );
    }
    if resp.double_clicked() {
        Some(FaderEvent::Rearm)
    } else if resp.secondary_clicked() {
        Some(FaderEvent::CycleReturn)
    } else if resp.drag_stopped() || resp.clicked() {
        // A plain click set the value while pressed; letting go releases it.
        Some(FaderEvent::Release)
    } else if resp.is_pointer_button_down_on() {
        resp.interact_pointer_pos().map(|pos| {
            FaderEvent::Touch(((track.bottom() - pos.y) / track.height()).clamp(0.0, 1.0))
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_hold_and_glide_back_onto_a_bar_line() {
        let mut f = Fader::default();
        assert_eq!(f.value(0.3, 0.0), 0.3);
        f.touch(0.9);
        assert_eq!(f.value(0.3, 10.0), 0.9, "held while touched");
        // Released at beat 10.5 (bar phase 0.625) with RETURN = 1 bar: lands on beat 16.
        f.release(10.5, 0.625, Return::Bar);
        assert_eq!(f.value(0.3, 11.0), 0.9, "holds first");
        let mid = f.value(0.3, 14.0);
        assert!(mid < 0.9 && mid > 0.3, "{mid}");
        assert!((f.value(0.3, 15.999) - 0.3).abs() < 0.01);
        assert_eq!(f.value(0.3, 16.0), 0.3);
        assert!(!f.is_manual(), "back on AUTO at the bar line");
    }

    #[test]
    fn latch_until_rearmed_and_per_fader_override() {
        let mut f = Fader {
            ret: Some(Return::Never),
            ..Default::default()
        };
        f.touch(0.7);
        f.release(3.0, 0.75, Return::Bar);
        assert_eq!(f.value(0.1, 1000.0), 0.7);
        assert_eq!(f.manual(0.1, 1000.0), Some(0.7));
        f.rearm();
        assert_eq!(f.manual(0.1, 1000.0), None);
        // Global 1 beat, released exactly on a downbeat → lands on the next bar line.
        let mut g = Fader::default();
        g.touch(1.0);
        g.release(8.0, 0.0, Return::Beat);
        assert_eq!(g.value(0.0, 11.0), 1.0);
        assert!(g.value(0.0, 11.5) < 1.0);
        assert_eq!(g.value(0.0, 12.0), 0.0);
        assert_eq!(Return::Phrase.next(), Return::Never);
        assert_eq!(Return::Never.next(), Return::Beat);
    }
}
