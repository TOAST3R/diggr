//! Fullscreen track info: artist, title, progress with section ticks, time. Shown for 5 s on
//! entry, on track change and on input, then fades out over 1 s.

use analysis::{SectionKind, SongScore};
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, pos2, vec2};

pub const SHOW_SECS: f64 = 5.0;
pub const FADE_SECS: f64 = 1.0;

#[derive(Debug, Default)]
pub struct Fade {
    shown_at: Option<f64>,
}

impl Fade {
    /// Show again (entry, track change, mouse move, key).
    pub fn poke(&mut self, now: f64) {
        self.shown_at = Some(now);
    }

    pub fn alpha(&self, now: f64) -> f32 {
        let Some(t0) = self.shown_at else { return 0.0 };
        let t = now - t0;
        if t < SHOW_SECS {
            1.0
        } else {
            (1.0 - (t - SHOW_SECS) / FADE_SECS).clamp(0.0, 1.0) as f32
        }
    }
}

pub fn kind_color(k: SectionKind) -> Color32 {
    match k {
        SectionKind::Intro => Color32::from_rgb(140, 160, 190),
        SectionKind::Build => Color32::from_rgb(255, 170, 60),
        SectionKind::Drop => Color32::from_rgb(255, 70, 70),
        SectionKind::Breakdown => Color32::from_rgb(80, 210, 230),
        SectionKind::Groove => Color32::from_rgb(110, 220, 120),
        SectionKind::Outro => Color32::from_rgb(180, 120, 230),
    }
}

/// Where the section ticks go: (fraction of the track, kind, final).
pub fn ticks(score: &SongScore, duration: f64) -> Vec<(f32, SectionKind, bool)> {
    if duration <= 0.0 {
        return Vec::new();
    }
    score
        .sections
        .iter()
        .filter_map(|s| {
            let t = *score.beats.get(s.start_beat)?;
            Some(((t / duration).clamp(0.0, 1.0) as f32, s.kind, s.is_final))
        })
        .collect()
}

pub struct Info<'a> {
    pub artist: &'a str,
    pub title: &'a str,
    pub elapsed: f64,
    pub duration: Option<f64>,
    pub score: Option<&'a SongScore>,
    /// Space to keep free at the bottom (the host's analysis strip).
    pub bottom_inset: f32,
}

fn with_alpha(c: Color32, a: f32) -> Color32 {
    let [r, g, b, x] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, (x as f32 * a) as u8)
}

fn shadowed(
    p: &egui::Painter,
    at: Pos2,
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
    alpha: f32,
) -> Rect {
    p.text(
        at + vec2(1.5, 1.5),
        anchor,
        text,
        font.clone(),
        with_alpha(Color32::BLACK, 0.7 * alpha),
    );
    p.text(at, anchor, text, font, with_alpha(color, alpha))
}

/// Draws the overlay; does nothing when fully faded (no layout work).
pub fn draw(p: &egui::Painter, screen: Rect, alpha: f32, info: &Info, accent: Color32) {
    if alpha <= 0.0 {
        return;
    }
    // Soft dark gradient behind the text so it reads on any picture.
    let bottom = screen.bottom() - info.bottom_inset;
    let top = bottom - 260.0;
    let mut mesh = egui::Mesh::default();
    let clear = Color32::TRANSPARENT;
    let dark = with_alpha(Color32::BLACK, 0.6 * alpha);
    mesh.colored_vertex(pos2(screen.left(), top), clear);
    mesh.colored_vertex(pos2(screen.right(), top), clear);
    mesh.colored_vertex(pos2(screen.right(), bottom), dark);
    mesh.colored_vertex(pos2(screen.left(), bottom), dark);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(mesh);

    let x = screen.left() + 48.0;
    let width = (screen.width() - 96.0).min(560.0);
    let mut y = bottom - 48.0;
    // Time, then the progress line, then title and artist above.
    let time = match info.duration {
        Some(d) => format!(
            "{} / {}",
            ui::format::clock(info.elapsed),
            ui::format::clock(d)
        ),
        None => ui::format::clock(info.elapsed),
    };
    shadowed(
        p,
        pos2(x, y),
        Align2::LEFT_BOTTOM,
        &time,
        FontId::monospace(15.0),
        Color32::from_gray(210),
        alpha,
    );
    y -= 26.0;
    if let Some(d) = info.duration.filter(|d| *d > 0.0) {
        let line = Rect::from_min_size(pos2(x, y - 3.0), vec2(width, 3.0));
        p.rect_filled(line, 1.5, with_alpha(Color32::from_gray(255), 0.25 * alpha));
        let f = (info.elapsed / d).clamp(0.0, 1.0) as f32;
        p.rect_filled(
            Rect::from_min_size(line.min, vec2(width * f, 3.0)),
            1.5,
            with_alpha(accent, alpha),
        );
        if let Some(score) = info.score {
            for (at, kind, fin) in ticks(score, d) {
                let tx = x + width * at;
                let a = if fin { alpha } else { 0.35 * alpha };
                p.line_segment(
                    [pos2(tx, y - 9.0), pos2(tx, y + 4.0)],
                    Stroke::new(2.0, with_alpha(kind_color(kind), a)),
                );
            }
        }
        y -= 18.0;
    }
    let title = shadowed(
        p,
        pos2(x, y),
        Align2::LEFT_BOTTOM,
        info.title,
        FontId::proportional(34.0),
        Color32::WHITE,
        alpha,
    );
    if !info.artist.is_empty() {
        shadowed(
            p,
            pos2(x, title.top() - 4.0),
            Align2::LEFT_BOTTOM,
            &info.artist.to_uppercase(),
            FontId::proportional(15.0),
            Color32::from_gray(200),
            alpha,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysis::score::Section;

    #[test]
    fn shows_for_five_seconds_then_fades_in_one() {
        let mut f = Fade::default();
        assert_eq!(f.alpha(0.0), 0.0, "hidden until poked");
        f.poke(10.0);
        assert_eq!(f.alpha(14.9), 1.0);
        assert!((f.alpha(15.5) - 0.5).abs() < 1e-6);
        assert_eq!(f.alpha(16.0), 0.0);
        f.poke(20.0);
        assert_eq!(f.alpha(20.1), 1.0, "input brings it back");
    }

    #[test]
    fn ticks_at_section_starts_with_finality() {
        let score = SongScore {
            beats: (0..100).map(|i| i as f64).collect(),
            sections: vec![
                Section {
                    start_beat: 0,
                    end_beat: 50,
                    label: 0,
                    kind: SectionKind::Intro,
                    is_final: true,
                },
                Section {
                    start_beat: 50,
                    end_beat: 100,
                    label: 1,
                    kind: SectionKind::Drop,
                    is_final: false,
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            ticks(&score, 200.0),
            [
                (0.0, SectionKind::Intro, true),
                (0.25, SectionKind::Drop, false)
            ]
        );
        assert!(ticks(&score, 0.0).is_empty());
    }

    #[test]
    fn draws_the_spec_example_without_panicking() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let info = Info {
                artist: "M83",
                title: "Midnight City",
                elapsed: 132.0,
                duration: Some(243.0),
                score: None,
                bottom_inset: 0.0,
            };
            draw(ui.painter(), ui.max_rect(), 1.0, &info, Color32::GREEN);
        });
        out.textures_delta.clear();
    }
}
