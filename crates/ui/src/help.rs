//! The shortcuts help panel (`H` or `F1`): every key and mouse action, grouped. This table is
//! the one list of shortcuts shown to the user; keep it in step with the key handlers.

use egui::{Align2, Color32, RichText};

/// (group, [(keys, action)]).
pub const GROUPS: &[(&str, &[(&str, &str)])] = &[
    (
        "Playback",
        &[
            ("X", "play"),
            ("C", "pause / resume"),
            ("V", "stop"),
            ("Z / B", "previous / next track"),
            ("← / →", "seek −5 s / +5 s"),
            ("↑ / ↓", "volume"),
        ],
    ),
    (
        "Structure (on the beat)",
        &[
            ("]", "next section"),
            (
                "[",
                "start of this section (in its first bar: the previous one)",
            ),
            ("Shift+]", "next drop (a section at least 4 dB louder)"),
            ("L", "loop the current section (again: stop)"),
            ("Shift+L", "loop 4 bars (again: 8, then 16)"),
        ],
    ),
    (
        "Player window",
        &[
            ("W", "show / hide the waveform"),
            ("S", "spectrogram window (also in the playlist's OPT menu)"),
            ("F", "fullscreen visuals"),
            ("H or F1", "this help"),
            ("Enter", "play the selected entry"),
            ("Delete / Backspace", "remove selected entries"),
            ("Cmd+O / Cmd+A", "add files / select all"),
        ],
    ),
    (
        "Fullscreen visuals",
        &[
            ("F or Esc", "leave fullscreen"),
            ("D", "fader deck"),
            ("M / Shift+M", "mutate the look (small / big step)"),
            ("K", "keep the look as a new variant"),
            ("Backspace", "undo the last look change"),
            ("1–5", "rate the look"),
            ("T", "analysis strip"),
            (
                "A",
                "annotation mode (Space: tap the beat, 1–6: mark a section)",
            ),
        ],
    ),
    (
        "Mouse",
        &[
            (
                "Playlist title: click / drag",
                "crate menu (switch, new, rename, delete) / move the window",
            ),
            (
                "Entry: double-click",
                "play (a waiting entry plays as soon as its audio arrives)",
            ),
            ("Entry: right-click", "Send to crate (a crate or a new one)"),
            ("Waveform: click / drag", "seek"),
            ("Waveform: scroll wheel", "zoom 1–64 bars"),
            ("Spectrogram: click / drag", "seek / pan"),
            (
                "Spectrogram: scroll / Shift+scroll",
                "zoom time / zoom frequency (double-click: whole track)",
            ),
            (
                "Deck fader: drag",
                "set by hand (returns to automation after RETURN)",
            ),
            (
                "Deck fader: right-click / double-click",
                "its own RETURN / back to automation",
            ),
        ],
    ),
];

const TITLE: &str = "Keyboard shortcuts";
const ACCENT: Color32 = Color32::from_rgb(0, 220, 110);

/// Draws the panel while `open`; the window's close button clears it. It always fits inside
/// the window (the player window is small), scrolling when needed. Returns where it was drawn.
pub fn show(ctx: &egui::Context, open: &mut bool) -> Option<egui::Rect> {
    let screen = ctx.content_rect();
    let width = (screen.width() - 32.0).clamp(200.0, 560.0);
    let height = (screen.height() - 64.0).clamp(120.0, 760.0);
    let frame = egui::Frame::window(&ctx.global_style())
        .fill(Color32::from_rgb(16, 18, 24))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(70, 76, 96)));
    let shown = egui::Window::new(TITLE)
        .open(open)
        .collapsible(false)
        .resizable(false)
        .frame(frame)
        .fixed_size([width, height])
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let keys_w = (width * 0.36).min(170.0);
                    egui::Grid::new("help-grid")
                        .num_columns(2)
                        .striped(true)
                        .spacing([12.0, 5.0])
                        .min_col_width(keys_w)
                        .max_col_width(width - keys_w - 40.0)
                        .show(ui, |ui| {
                            for (group, rows) in GROUPS {
                                ui.label(RichText::new(*group).strong().size(14.0).color(ACCENT));
                                ui.label("");
                                ui.end_row();
                                for (keys, action) in *rows {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(*keys).monospace().color(Color32::WHITE),
                                        )
                                        .wrap(),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(*action).color(Color32::from_gray(215)),
                                        )
                                        .wrap(),
                                    );
                                    ui.end_row();
                                }
                            }
                        });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("H, F1 or Esc closes this")
                            .small()
                            .color(Color32::from_gray(150)),
                    );
                });
        });
    shown.map(|r| r.response.rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_every_navigation_and_visual_key() {
        let keys: Vec<&str> = GROUPS
            .iter()
            .flat_map(|(_, rows)| rows.iter().map(|(k, _)| *k))
            .collect();
        for k in [
            "X", "]", "[", "Shift+]", "L", "Shift+L", "W", "S", "F", "H or F1", "D", "K", "T", "A",
        ] {
            assert!(keys.contains(&k), "help is missing {k}");
        }
    }

    #[test]
    fn describes_the_crate_mouse_actions() {
        let rows: Vec<&str> = GROUPS
            .iter()
            .flat_map(|(_, rows)| rows.iter().map(|(_, a)| *a))
            .collect();
        assert!(rows.iter().any(|a| a.contains("crate menu")));
        assert!(rows.iter().any(|a| a.contains("Send to crate")));
    }

    #[test]
    fn fits_inside_the_small_player_window() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(550.0, 900.0));
        let input = || egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut open = true;
        let mut rect = None;
        // A few frames so the window settles its size.
        for _ in 0..3 {
            let mut out = ctx.run_ui(input(), |ui| rect = show(ui.ctx(), &mut open));
            out.textures_delta.clear();
        }
        assert!(open);
        let rect = rect.expect("the panel was drawn");
        assert!(
            screen.contains_rect(rect),
            "panel {rect:?} must fit in {screen:?}"
        );
        assert!(rect.width() > 400.0, "uses the width available: {rect:?}");
    }
}
