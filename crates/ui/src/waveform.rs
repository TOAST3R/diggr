//! The waveform section (`W`), under the main window: a whole-track overview row (section
//! bands, drop markers, playhead, loop; click or drag to seek) and a zoomed row scrolling around
//! the playhead (colour by frequency, mirrored channels, beat grid; the wheel zooms).

use analysis::SongScore;
use analysis::overview::{Block, Overview};
use egui::{Color32, Mesh, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};

/// Section height in skin pixels (multiplied by the UI scale).
pub const HEIGHT: u32 = 58;
const OVERVIEW_H: f32 = 14.0;
pub const MIN_BARS: f32 = 1.0;
pub const MAX_BARS: f32 = 64.0;
pub const DEFAULT_BARS: f32 = 8.0;

const BG: Color32 = Color32::from_rgb(10, 12, 16);
const FRAME: Color32 = Color32::from_rgb(58, 62, 78);

/// What the user did in the section.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Seek(f64),
    Zoom(f32),
}

pub struct View<'a> {
    pub overview: Option<&'a Overview>,
    pub score: Option<&'a SongScore>,
    /// Audible position, seconds.
    pub now: f64,
    pub duration: Option<f64>,
    /// Bars visible in the zoomed row.
    pub zoom_bars: f32,
    pub loop_region: Option<(f64, f64)>,
    /// A scheduled jump lands here (seconds of the current track).
    pub pending_jump: Option<f64>,
    /// Energy-rise ("drop") boundaries, seconds.
    pub drops: &'a [f64],
}

fn amp(db_byte: u8) -> f32 {
    10f32.powf((db_byte as f32 / 255.0 * 60.0 - 60.0) / 20.0)
}

/// A column's colour from its band energies: bass → red, mids → green, highs → blue.
pub fn colour(b: &Block) -> Color32 {
    let (lo, mi, hi) = (amp(b.low), amp(b.mid), amp(b.high));
    let m = lo.max(mi).max(hi).max(1e-6);
    let c = |x: f32| (40.0 + 215.0 * (x / m).powf(0.8)) as u8;
    Color32::from_rgb(c(lo), c(mi), c(hi))
}

/// Seconds at horizontal position `x` of an overview row spanning the whole track.
pub fn secs_at(x: f32, row: Rect, duration: f64) -> f64 {
    (((x - row.left()) / row.width()).clamp(0.0, 1.0) as f64) * duration
}

/// The time span of the zoomed row: `bars` bars (at the tempo at `now`) centered on `now`.
pub fn window(score: Option<&SongScore>, now: f64, bars: f32) -> (f64, f64) {
    let bpm = score.and_then(|s| s.bpm_at(now)).unwrap_or(120.0);
    let span = bars as f64 * 4.0 * 60.0 / bpm;
    (now - span / 2.0, now + span / 2.0)
}

fn dim(c: Color32, f: f32) -> Color32 {
    Color32::from_rgb(
        (c.r() as f32 * f) as u8,
        (c.g() as f32 * f) as u8,
        (c.b() as f32 * f) as u8,
    )
}

fn peak(a: i8, b: i8) -> f32 {
    (a as f32).abs().max((b as f32).abs()) / 127.0
}

/// Draws the section in `rect` and reports seeks and zoom changes.
pub fn draw(ui: &mut Ui, rect: Rect, scale: f32, v: &View) -> Option<Action> {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, BG);
    p.rect_stroke(
        rect.shrink(0.5),
        0.0,
        Stroke::new(1.0, FRAME),
        egui::StrokeKind::Inside,
    );
    let inner = rect.shrink(2.0 * scale);
    let over = Rect::from_min_size(inner.min, vec2(inner.width(), OVERVIEW_H * scale));
    let zoom = Rect::from_min_max(pos2(inner.left(), over.bottom() + 2.0 * scale), inner.max);
    let mut mesh = Mesh::default();
    let mut action = None;

    // ---- overview row -------------------------------------------------------------------
    if let Some(d) = v.duration.filter(|d| *d > 0.0) {
        let x_of = |t: f64| over.left() + (t / d).clamp(0.0, 1.0) as f32 * over.width();
        if let Some(s) = v.score {
            for sec in &s.sections {
                let (Some(&a), b) = (
                    s.beats.get(sec.start_beat),
                    s.beats.get(sec.end_beat).copied().unwrap_or(d),
                ) else {
                    continue;
                };
                let c = crate::timeline::kind_color(sec.kind);
                let c = if sec.is_final {
                    c
                } else {
                    c.gamma_multiply(0.4)
                };
                mesh.add_colored_rect(
                    Rect::from_x_y_ranges(
                        x_of(a)..=x_of(b) - 1.0,
                        over.bottom() - 2.0 * scale..=over.bottom(),
                    ),
                    c,
                );
            }
        }
        if let Some(o) = v.overview {
            let cols = over.width().max(1.0) as usize;
            let mid = over.center().y - scale;
            let half = over.height() / 2.0 - 2.0 * scale;
            for i in 0..cols {
                let (t0, t1) = (d * i as f64 / cols as f64, d * (i + 1) as f64 / cols as f64);
                let Some(b) = o.column(t0, t1) else { break };
                let h = (peak(b.min_l, b.max_l).max(peak(b.min_r, b.max_r)) * half).max(0.5);
                let c = if t1 <= v.now {
                    dim(colour(&b), 0.55)
                } else {
                    colour(&b)
                };
                let x = over.left() + i as f32;
                mesh.add_colored_rect(Rect::from_x_y_ranges(x..=x + 1.0, mid - h..=mid + h), c);
            }
        }
        if let Some((a, b)) = v.loop_region {
            mesh.add_colored_rect(
                Rect::from_x_y_ranges(x_of(a)..=x_of(b), over.y_range()),
                Color32::from_rgba_unmultiplied(255, 200, 60, 40),
            );
        }
        let resp = ui.interact(
            over,
            ui.id().with("waveform-overview"),
            Sense::click_and_drag(),
        );
        if (resp.clicked() || resp.dragged())
            && let Some(pos) = resp.interact_pointer_pos()
        {
            action = Some(Action::Seek(secs_at(pos.x, over, d)));
        }
        p.add(Shape::mesh(std::mem::take(&mut mesh)));
        for &t in v.drops {
            let x = x_of(t);
            let tri = vec![
                pos2(x - 3.0 * scale, over.top()),
                pos2(x + 3.0 * scale, over.top()),
                pos2(x, over.top() + 4.0 * scale),
            ];
            p.add(Shape::convex_polygon(
                tri,
                Color32::from_rgb(255, 80, 80),
                Stroke::NONE,
            ));
        }
        let x = x_of(v.now);
        p.line_segment(
            [pos2(x, over.top()), pos2(x, over.bottom())],
            Stroke::new(scale.max(1.0), Color32::WHITE),
        );
    }

    // ---- zoomed row ---------------------------------------------------------------------
    let (t0, t1) = window(v.score, v.now, v.zoom_bars);
    let x_of = |t: f64| zoom.left() + ((t - t0) / (t1 - t0)) as f32 * zoom.width();
    let mid = zoom.center().y;
    let half = zoom.height() / 2.0 - scale;
    if let Some((a, b)) = v.loop_region {
        mesh.add_colored_rect(
            Rect::from_x_y_ranges(
                x_of(a).max(zoom.left())..=x_of(b).min(zoom.right()),
                zoom.y_range(),
            ),
            Color32::from_rgba_unmultiplied(255, 200, 60, 28),
        );
    }
    if let Some(o) = v.overview {
        let cols = zoom.width().max(1.0) as usize;
        let step = (t1 - t0) / cols as f64;
        for i in 0..cols {
            let a = t0 + step * i as f64;
            if a < 0.0 {
                continue;
            }
            let Some(b) = o.column(a, a + step) else {
                break;
            };
            let c = colour(&b);
            let x = zoom.left() + i as f32;
            let (l, r) = (peak(b.min_l, b.max_l) * half, peak(b.min_r, b.max_r) * half);
            let core = (b.rms as f32 / 255.0 * 1.8).min(1.0);
            mesh.add_colored_rect(
                Rect::from_x_y_ranges(x..=x + 1.0, mid - l.max(0.5)..=mid),
                dim(c, 0.6),
            );
            mesh.add_colored_rect(
                Rect::from_x_y_ranges(x..=x + 1.0, mid..=mid + r.max(0.5)),
                dim(c, 0.6),
            );
            mesh.add_colored_rect(
                Rect::from_x_y_ranges(x..=x + 1.0, mid - l * core..=mid + r * core),
                c,
            );
        }
    }
    p.add(Shape::mesh(mesh));
    if let Some(s) = v.score {
        let from = s.beats.partition_point(|&b| b < t0);
        let to = s.beats.partition_point(|&b| b <= t1);
        let mut downbeats = s
            .downbeats
            .iter()
            .copied()
            .filter(|&d| d >= from && d < to)
            .peekable();
        for (i, &bt) in s.beats[from..to].iter().enumerate() {
            let is_down = downbeats.peek() == Some(&(from + i));
            if is_down {
                downbeats.next();
            }
            let x = x_of(bt);
            let (len, c) = if is_down {
                (5.0, Color32::from_gray(230))
            } else {
                (2.5, Color32::from_gray(120))
            };
            p.line_segment(
                [pos2(x, zoom.top()), pos2(x, zoom.top() + len * scale)],
                Stroke::new(1.0, c),
            );
            p.line_segment(
                [pos2(x, zoom.bottom() - len * scale), pos2(x, zoom.bottom())],
                Stroke::new(1.0, c),
            );
        }
    }
    if let Some(at) = v.pending_jump.filter(|&t| t >= t0 && t <= t1) {
        let x = x_of(at);
        p.line_segment(
            [pos2(x, zoom.top()), pos2(x, zoom.bottom())],
            Stroke::new(scale, Color32::from_rgb(255, 200, 60)),
        );
    }
    let cx = x_of(v.now);
    p.line_segment(
        [Pos2::new(cx, zoom.top()), Pos2::new(cx, zoom.bottom())],
        Stroke::new(scale.max(1.0), Color32::WHITE),
    );
    let zresp = ui.interact(zoom, ui.id().with("waveform-zoom"), Sense::hover());
    if zresp.hovered() {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy != 0.0 {
            let bars = (v.zoom_bars * 2f32.powf(-dy / 60.0)).clamp(MIN_BARS, MAX_BARS);
            action = action.or(Some(Action::Zoom(bars)));
        }
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysis::overview::OverviewBuilder;

    const SR: u32 = 44_100;

    fn block_of(signal: impl Fn(usize) -> f32) -> Block {
        let mut b = OverviewBuilder::new(SR);
        let x: Vec<f32> = (0..SR as usize)
            .flat_map(|i| [signal(i), signal(i)])
            .collect();
        b.push(&x);
        b.finish().column(0.4, 0.6).unwrap()
    }

    #[test]
    fn kicks_are_red_and_hats_are_blue() {
        let kick = colour(&block_of(|i| {
            0.8 * (std::f32::consts::TAU * 55.0 * i as f32 / SR as f32).sin()
        }));
        assert!(
            kick.r() > 200 && kick.b() < 120 && kick.g() < 120,
            "{kick:?}"
        );
        let hat = colour(&block_of(|i| {
            // White-ish noise high-passed by differencing: energy mostly above 6 kHz.
            let n = |k: usize| ((k as f32 * 12.9898).sin() * 43758.547).fract() - 0.5;
            0.5 * (n(i) - n(i + 1))
        }));
        assert!(hat.b() > 200 && hat.r() < 120, "{hat:?}");
    }

    #[test]
    fn overview_click_maps_to_track_time() {
        let row = Rect::from_min_size(pos2(10.0, 0.0), vec2(200.0, 14.0));
        assert_eq!(secs_at(160.0, row, 240.0), 180.0);
        assert_eq!(secs_at(0.0, row, 240.0), 0.0);
        assert_eq!(secs_at(500.0, row, 240.0), 240.0);
    }

    #[test]
    fn zoom_window_follows_tempo() {
        let (a, b) = window(None, 60.0, 8.0);
        assert_eq!((a, b), (52.0, 68.0), "8 bars at 120 BPM = 16 s");
    }

    #[test]
    fn draws_headless() {
        let ctx = egui::Context::default();
        let mut b = OverviewBuilder::new(SR);
        b.push(&vec![0.3; 2 * SR as usize * 10]);
        let o = b.finish();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let v = View {
                overview: Some(&o),
                score: None,
                now: 4.0,
                duration: Some(10.0),
                zoom_bars: 8.0,
                loop_region: Some((2.0, 3.0)),
                pending_jump: Some(5.0),
                drops: &[6.0],
            };
            let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(550.0, 116.0));
            assert_eq!(draw(ui, rect, 2.0, &v), None);
        });
        out.textures_delta.clear();
    }
}
