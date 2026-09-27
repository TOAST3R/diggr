//! The analysis debug strip (fullscreen, `T`): what the analyzer thinks is happening around the
//! playhead — beats, bars, sections, tension, coverage — plus annotation marks while annotating.

use analysis::eval::Annotations;
use analysis::{SectionKind, SongScore};
use audio::{PlayState, Position};
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Ui, pos2, vec2};

/// Seconds shown before and after the playhead.
pub const BEFORE: f64 = 6.0;
pub const AFTER: f64 = 10.0;
pub const HEIGHT: f32 = 110.0;

pub fn kind_color(kind: SectionKind) -> Color32 {
    match kind {
        SectionKind::Intro => Color32::from_rgb(90, 110, 160),
        SectionKind::Build => Color32::from_rgb(230, 150, 40),
        SectionKind::Drop => Color32::from_rgb(230, 50, 90),
        SectionKind::Breakdown => Color32::from_rgb(60, 140, 230),
        SectionKind::Groove => Color32::from_rgb(70, 180, 110),
        SectionKind::Outro => Color32::from_rgb(120, 120, 130),
    }
}

/// Horizontal position of time `t` in a strip centered (at 3/8) on `now`.
pub fn x_of(t: f64, now: f64, strip: Rect) -> f32 {
    let f = (t - (now - BEFORE)) / (BEFORE + AFTER);
    strip.left() + (f as f32) * strip.width()
}

/// The strip's rectangle: along the bottom of the screen.
pub fn strip_rect(screen: Rect) -> Rect {
    Rect::from_min_max(
        pos2(screen.left() + 24.0, screen.bottom() - HEIGHT - 24.0),
        pos2(screen.right() - 24.0, screen.bottom() - 24.0),
    )
}

/// One-line status: tempo, current section, countdown to the next drop.
pub fn status_line(score: Option<&SongScore>, now: f64) -> String {
    let Some(s) = score else {
        return "analyzing…".into();
    };
    if !s.coverage.contains(now) {
        return "analyzing…".into();
    }
    let Some(beat) = s.beat_at(now) else {
        return "no beat here".into();
    };
    let b = beat.floor() as usize;
    let mut parts = Vec::new();
    if let Some(bpm) = s.bpm_at(now) {
        parts.push(format!("{bpm:.1} BPM"));
    }
    if let Some(sec) = s.section_at(b) {
        let final_mark = if sec.is_final { "" } else { " (provisional)" };
        parts.push(format!(
            "{} {}{final_mark}",
            sec.label_name(),
            sec.kind.name()
        ));
    }
    if let Some(d) = s.drop_in(b) {
        parts.push(format!("drop in {d} beats"));
    }
    let t = s.tension(b);
    if t > 0.0 {
        parts.push(format!("tension {:.0}%", t * 100.0));
    }
    parts.join("  ·  ")
}

pub fn draw(
    ui: &Ui,
    screen: Rect,
    pos: &Position,
    score: Option<&SongScore>,
    ann: Option<&Annotations>,
) {
    let strip = strip_rect(screen);
    let p = ui.painter().with_clip_rect(strip.expand(2.0));
    let now = if pos.state == PlayState::Stopped {
        0.0
    } else {
        pos.seconds()
    };
    let (a, b) = (now - BEFORE, now + AFTER);
    p.rect_filled(strip, 6.0, Color32::from_black_alpha(170));
    let band = Rect::from_min_max(
        pos2(strip.left(), strip.top() + 22.0),
        pos2(strip.right(), strip.top() + 58.0),
    );

    if let Some(s) = score {
        // Unanalyzed time is hatched darker.
        let mut x0 = strip.left();
        for &(cs, ce) in s.coverage.spans() {
            let (l, r) = (x_of(cs, now, strip), x_of(ce, now, strip));
            if l > x0 {
                p.rect_filled(
                    Rect::from_min_max(pos2(x0, band.top()), pos2(l, band.bottom())),
                    0.0,
                    Color32::from_gray(25),
                );
            }
            x0 = x0.max(r);
        }
        if x0 < strip.right() {
            p.rect_filled(
                Rect::from_min_max(pos2(x0, band.top()), pos2(strip.right(), band.bottom())),
                0.0,
                Color32::from_gray(25),
            );
        }
        // Sections.
        for sec in &s.sections {
            let (Some(&t0), Some(&t1)) = (
                s.beats.get(sec.start_beat),
                s.beats.get(sec.end_beat.min(s.beats.len()) - 1),
            ) else {
                continue;
            };
            if t1 < a || t0 > b {
                continue;
            }
            let alpha = if sec.is_final { 200 } else { 90 };
            let c = kind_color(sec.kind);
            let r = Rect::from_min_max(
                pos2(x_of(t0, now, strip), band.top()),
                pos2(x_of(t1, now, strip), band.bottom()),
            );
            p.rect_filled(
                r,
                2.0,
                Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha),
            );
            p.text(
                pos2(r.left().max(strip.left()) + 4.0, band.top() + 3.0),
                Align2::LEFT_TOP,
                format!("{} {}", sec.label_name(), sec.kind.name()),
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        }
        // Beats (downbeats taller) and the tension curve.
        let first = s.beats.partition_point(|&t| t < a);
        let mut tension: Vec<Pos2> = Vec::new();
        for i in first..s.beats.len() {
            let t = s.beats[i];
            if t > b {
                break;
            }
            let x = x_of(t, now, strip);
            let down = s.downbeats.binary_search(&i).is_ok();
            let h = if down { 16.0 } else { 7.0 };
            p.line_segment(
                [pos2(x, band.bottom()), pos2(x, band.bottom() + h)],
                Stroke::new(if down { 2.0 } else { 1.0 }, Color32::from_gray(210)),
            );
            let y = strip.bottom() - 6.0 - s.tension(i) * 28.0;
            tension.push(pos2(x, y));
        }
        if tension.len() > 1 {
            p.add(egui::Shape::line(
                tension,
                Stroke::new(2.0, Color32::from_rgb(255, 200, 60)),
            ));
        }
    }
    if let Some(ann) = ann {
        for &t in ann.beats.iter().filter(|&&t| t >= a && t <= b) {
            let x = x_of(t, now, strip);
            p.circle_filled(
                pos2(x, band.top() - 4.0),
                3.0,
                Color32::from_rgb(255, 80, 80),
            );
        }
        for m in ann.boundaries.iter().filter(|m| m.t >= a && m.t <= b) {
            let x = x_of(m.t, now, strip);
            p.line_segment(
                [pos2(x, band.top() - 10.0), pos2(x, band.bottom())],
                Stroke::new(3.0, kind_color(m.kind)),
            );
        }
    }
    // Playhead and status.
    let x = x_of(now, now, strip);
    p.line_segment(
        [pos2(x, strip.top() + 18.0), pos2(x, strip.bottom() - 2.0)],
        Stroke::new(2.0, Color32::WHITE),
    );
    let mut status = status_line(score, now);
    if ann.is_some() {
        status = format!(
            "REC · ANNOTATING  Space beat · 1 intro 2 build 3 drop 4 breakdown 5 groove 6 outro · A stop    {status}"
        );
    }
    p.text(
        strip.left_top() + vec2(8.0, 4.0),
        Align2::LEFT_TOP,
        status,
        FontId::proportional(13.0),
        Color32::from_gray(230),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysis::score::{IntervalSet, Section};

    fn score() -> SongScore {
        let mut coverage = IntervalSet::default();
        coverage.insert(0.0, 60.0);
        SongScore {
            beats: (0..120).map(|i| i as f64 * 0.5).collect(),
            downbeats: (0..30).map(|i| i * 4).collect(),
            sections: vec![
                Section {
                    start_beat: 0,
                    end_beat: 32,
                    label: 0,
                    kind: SectionKind::Build,
                    is_final: true,
                },
                Section {
                    start_beat: 32,
                    end_beat: 120,
                    label: 1,
                    kind: SectionKind::Drop,
                    is_final: false,
                },
            ],
            coverage,
            tempo_segments: vec![analysis::score::TempoSegment {
                start: 0.0,
                end: 60.0,
                t0: 0.0,
                period: 0.5,
                confidence: 1.0,
            }],
            curves: analysis::score::BeatCurves {
                tension: (0..120).map(|i| if i < 32 { 0.5 } else { 0.0 }).collect(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn mapping_and_status() {
        let r = Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 100.0));
        assert_eq!(x_of(10.0, 10.0, r), 600.0, "playhead at 3/8");
        assert_eq!(x_of(4.0, 10.0, r), 0.0);
        let s = score();
        assert_eq!(
            status_line(Some(&s), 12.0),
            "120.0 BPM  ·  A build  ·  drop in 8 beats  ·  tension 50%"
        );
        assert_eq!(status_line(Some(&s), 70.0), "analyzing…");
        assert_eq!(status_line(None, 1.0), "analyzing…");
        let kinds: std::collections::HashSet<_> =
            SectionKind::ALL.iter().map(|&k| kind_color(k)).collect();
        assert_eq!(kinds.len(), 6, "each kind has its own color");
    }

    #[test]
    fn draws_headless_without_panicking() {
        let ctx = egui::Context::default();
        let s = score();
        let mut ann = Annotations::default();
        ann.tap(11.0);
        ann.boundary(16.0, SectionKind::Drop);
        let pos = Position {
            track: 1,
            frame: 480_000,
            sample_rate: 48_000,
            state: PlayState::Playing,
            discontinuity: false,
        };
        let mut out = ctx.run_ui(Default::default(), |ui| {
            draw(
                ui,
                Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 720.0)),
                &pos,
                Some(&s),
                Some(&ann),
            );
            draw(
                ui,
                Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 720.0)),
                &pos,
                None,
                None,
            );
        });
        out.textures_delta.clear();
    }
}
