//! The shortcuts help panel (`H` or `F1`): every key and mouse action, grouped. This table is
//! the one list of shortcuts shown to the user; keep it in step with the key handlers.

use egui::{Align2, Color32, RichText};

/// (group, [(keys, action)]).
pub const GROUPS: &[(&str, &[(&str, &str)])] = &[
    (
        "Playback",
        &[
            ("Space", "play / pause"),
            ("Shift+Space", "stop"),
            (
                "Z X C V B",
                "previous, play, pause, stop, next (the classic keys)",
            ),
            (
                "Left / Right",
                "seek −5 s / +5 s (with a grouped playlist focused: close / open the record)",
            ),
            (
                "Up / Down",
                "volume (with the playlist focused: move in the list)",
            ),
            (
                "Hold the turntable's platter",
                "a hand on the record: it slows to a stop; let go and it spins back up",
            ),
            (
                "Level meter (under the time)",
                "how loud each channel is, left above right, before the volume: green, amber \
                 from −12 dB, red from −3 dB; the dot is the last second's peak",
            ),
            (
                "Turntable's beat lamp",
                "flashes on each beat of the analysed grid as you hear it, brighter on the \
                 downbeat; dark until the beats are known",
            ),
        ],
    ),
    (
        "SYNC (every track at one tempo)",
        &[
            (
                "SYNC button (on the turntable)",
                "every track at the playing record's tempo (again: off)",
            ),
            (
                "Pitch fader: drag / scroll",
                "your set's tempo: up faster, down slower (centre: the record's own); every \
                 track then plays at it, pitch following",
            ),
            (
                "Shift+Up / Shift+Down",
                "tempo +1 / −1 BPM (player focused)",
            ),
            (
                "Alt+Up / Alt+Down",
                "tempo +0.01 / −0.01 BPM (player focused)",
            ),
            (
                "Up / Down (SYNC on)",
                "tempo +0.1 / −0.1 BPM instead of the volume (player focused)",
            ),
            (
                "Shift+drag / Shift+scroll the fader",
                "fine: 0.01 BPM per pixel / per 4 pixels of scroll",
            ),
            (
                "Click the tempo digits",
                "type a tempo (126.35), Enter sets it, Esc cancels",
            ),
            (
                "RESET / 8%·16%",
                "back to the record's own tempo / the pitch range",
            ),
            (
                "The state line",
                "LOCKED, ANALYZING, or OWN · OUT OF ±8% / NO BPM (red: the track plays at its \
                 own speed, its strobe dots drift and its row says (own tempo)); the meter \
                 beside the push shows how hard the record is pushed",
            ),
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
            (
                "Tab",
                "switch the keyboard between the player and the playlist",
            ),
            (
                "S",
                "spectrogram window (also the mini player's spectrum button)",
            ),
            (
                "Cmd+= / Cmd+− / Cmd+0",
                "double size / classic size / classic size",
            ),
            ("H or F1", "this help"),
            (
                "Up / Down (playlist focused)",
                "move in the list (Shift: extend the selection)",
            ),
            (
                "PgUp / PgDn, Home / End",
                "a page up / down, first / last entry",
            ),
            ("Enter", "play the entry under the cursor"),
            ("P", "show the playing entry"),
            (
                "Right / Left (playlist focused, grouped)",
                "open / close the record under the cursor",
            ),
            (
                "Right / Left, Esc (record card open)",
                "next / previous record, close the card",
            ),
            ("Delete / Backspace", "remove selected entries"),
            ("Cmd+O / Cmd+A", "add files / select all"),
            ("Cmd+F", "search the crate (Esc: clear the search)"),
        ],
    ),
    (
        "Digging (Discogs)",
        &[
            (
                "Cmd+V",
                "paste a Discogs page: its tracks' previews go into the shown crate",
            ),
            (
                "Y",
                "add the playing track's record to the wantlist (again: remove it)",
            ),
            ("N", "pass: dimmed, skipped, left out of later sends"),
            ("I", "open the release's for-sale page"),
        ],
    ),
    (
        "Mouse",
        &[
            (
                "Click in the player or the playlist",
                "give it the keyboard (its title lights up)",
            ),
            (
                "Playlist corner grip: drag",
                "resize the playlist: wider or narrower, and rows",
            ),
            (
                "Column header (wide playlist): click",
                "sort the crate by that column (again: the other way; also Options › Sort)",
            ),
            (
                "Column header: drag a divider / right-click",
                "resize a column / show or hide columns",
            ),
            (
                "Playlist title: click / drag",
                "crate menu when no sidebar shows (switch, new, rename, delete) / move the window",
            ),
            (
                "Crate sidebar (playlist 600 px wide)",
                "click a crate to show it (then Delete deletes it), right-click or Control-click \
                 to rename or delete (Playlist: clear), drop entries on it to send them; your \
                 Discogs collection is pinned at the bottom",
            ),
            (
                "LABELS (sidebar): right-click a label",
                "Delete label…, Export to crate, Refresh label, Download all tracks, Retry failed \
                 tracks; a label crate fills only from its label (N passes a track)",
            ),
            (
                "TOP SELLERS (sidebar): click / double-click a seller",
                "show its crate / dig it (once a day refreshes it); right-click: Add seller…, \
                 Refresh, Narrow down, Remove",
            ),
            (
                "Record row (grouped): click",
                "open its record card (× or Esc closes it): play each track, even one not \
                 downloaded yet, want or cart the record, ‹ › for the next records; Shift- or \
                 Cmd-click only selects",
            ),
            (
                "Copy (open record, seller crate): + CART / IN CART",
                "put the copy in your Discogs cart / take it out (a record with one copy for \
                 sale has the button on its own row; with several, it opens the record)",
            ),
            (
                "Empty crate: a store's name",
                "open the store's site; SET UP THE EXTENSION opens Options › Browser…",
            ),
            (
                "Any other dig entry: + CART",
                "buy it where it is known (the Discogs marketplace, Bandcamp, a store's \
                 basket); known on several stores or none: choose, or search the others",
            ),
            (
                "Juno or Bleep entry: + CART / IN CART",
                "put it in your basket on that store through the browser extension (Bleep asks \
                 which format) / take it out; Beatport and Traxsource open the track's page",
            ),
            (
                "Copy: double-click / right-click",
                "open its listing / Open on discogs.com",
            ),
            (
                "List: right-click below the rows",
                "add files (or an M3U) or a folder, export an M3U, select, remove, clear, sort",
            ),
            (
                "Footer CART n · subtotal (seller crate)",
                "show and play only the records with a copy in your cart",
            ),
            (
                "Footer BPM: drag a handle / × or double-click",
                "show and play only the tracks in that tempo range / show everything",
            ),
            (
                "Footer styles, ARTISTS, LABELS (Discogs wantlist and collection), FORMATS \
                 (any dig crate): click / double-click",
                "show only records with the picked styles, artists, labels or formats \
                 (☰ › Filter by… at any width) / turn that filter off",
            ),
            (
                "Mini player: right-click",
                "Options: Discogs…, Browser…, Set up Diggr…, Reduce motion",
            ),
            (
                "Mini player's EQ button",
                "the equalizer, in its own window",
            ),
            (
                "EQ's LP knob: drag down / double-click",
                "sweep the low-pass filter / turn it off",
            ),
            (
                "Entry: double-click",
                "play (a waiting entry plays as soon as its audio arrives)",
            ),
            (
                "Entry: right-click",
                "play or arm, remove, Send to crate (the selection, when the entry is in it); \
                 Analyze BPM (its audio first in the analysis queue); \
                 remove or select its whole album (tinted while the menu is open); keep, pass, \
                 for-sale page, open or copy its Discogs link; in the collection crate, remove \
                 one copy from your Discogs collection",
            ),
            (
                "Entry: hover",
                "everything known about it: its record's cover, album, label, cat#, side, \
                 year, BPM, status, for sale",
            ),
            (
                "Entry status icons",
                "hollow dot: listed · clock: queued or to search · bar: downloading · warning \
                 sign: needs yt-dlp · barred circle: no clip, clip failed, not found by search or already \
                 in crate",
            ),
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

pub const TITLE: &str = "Keyboard shortcuts";
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
    fn lists_every_navigation_and_digging_key() {
        let keys: Vec<&str> = GROUPS
            .iter()
            .flat_map(|(_, rows)| rows.iter().map(|(k, _)| *k))
            .collect();
        for k in [
            "Space",
            "Shift+Space",
            "Z X C V B",
            "]",
            "[",
            "Shift+]",
            "L",
            "Shift+L",
            "S",
            "Cmd+= / Cmd+− / Cmd+0",
            "H or F1",
            "Cmd+V",
            "Y",
            "N",
            "I",
            "Tab",
            "P",
            "Right / Left (playlist focused, grouped)",
            "Shift+Up / Shift+Down",
            "Alt+Up / Alt+Down",
        ] {
            assert!(keys.contains(&k), "help is missing {k}");
        }
        for k in ["F", "F or Esc", "T", "A", "D", "K", "X", "Shift+P"] {
            assert!(!keys.contains(&k), "help still lists {k}");
        }
        assert_eq!(GROUPS[0].1[0], ("Space", "play / pause"), "Space leads");
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
