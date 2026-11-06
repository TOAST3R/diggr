# Tasks

## 1. Layout

- [x] 1.1 In `layout.rs`, add `band_panels(win_w, corner)` (thirds rounded down, player and deck clamped to 227–360, the waveform the rest, never negative) and make `BAND_H` 76 and `band_h` / `deck_h` independent of SYNC. Verify: unit tests for 720 (227 / 227 / 227), 1,100 (353 / 353 / 354), 1,500 (360 / 360 / 741) and 600 (227 / 227 / 107); `playlist_fit` gives 23 rows at 720 × 435.

## 2. Skin

- [x] 2.1 In `skin/generate.rs`, redraw `deck_bg` as 227×76 with no title bar: 64 px platter, shorter tonearm, LCD inset, 56 px fader groove, no "PITCH" label; add `deck_fill` and `deck_edge`, the 12 px `deck_state` inset, and the new layout entries (`deck_state`, `deck_sync`, `deck_meter`); drop `deck_titlebar`, `deck_close` and `own_tempo_*`; make `volume_track` / `volume_fill` three pieces; dim the unlit seven-segment strokes to a quarter. Update `REQUIRED_SPRITES` and `REQUIRED_LAYOUT`. Verify: `cargo run -p ui --bin skin-gen`, the skin-on-disk test, and a test that the unlit/lit brightness ratio is at most 0.25.
- [x] 2.2 Set `waveform::HEIGHT` to 76 and share it between the overview and zoom rows. Verify: the waveform tests pass and a draw test at 76 shows both rows.

## 3. The player panel

- [x] 3.1 Make `mini_player` take the panel width: LCD fill, `lcd_icons_x(w)` with three icons, the volume from x 123 to shuffle (at least 60 px, drawn from the three-piece sprites), shuffle and repeat ending 4 px from the edge; remove SYNC from the LCD row and its constants. Verify: `the_buttons_fit_in_the_mini_player` at 227 and 353 (no overlap, 15 title characters at 227, icons 4 px from the edge) and the verdict-flash test.
- [x] 3.2 Add the LCD's second line (`label · side · year · for-sale` for a Discogs entry, the album tag for a file, empty when nothing plays) and take the Discogs continuation off the scrolling line. Verify: headless tests for the Discogs, local-file and stopped cases, and the flash covering row one only.

## 4. The turntable

- [x] 4.1 Draw `deck_section` every frame at the middle panel, content centred; remove the × and the title bar; add the SYNC button (`sync_on` / `sync_off`); with SYNC off dim everything but SYNC, show the record's own BPM, and ignore the fader, range and RESET. Verify: headless tests that the deck is there with SYNC off and on at the same place and size, that SYNC on the deck toggles it, and that the fader and RESET do nothing while off.
- [x] 4.2 Draw the state line (SYNC OFF, ANALYZING, LOCKED +x.x%, OWN · OUT OF ±r%, OWN · NO BPM), the lamp colour and the pitch meter. Verify: headless tests for each state and its text (including the out-of-range case at target 102 / record 124 and the locked case at 126 / 122), and that the longest line fits the readout column at 227.
- [x] 4.3 Add the effects: the lock flash (0.6 s), the digits counting from the record's BPM to the target (0.5 s), the analyzing chase, and the red drift. Verify: tests that a fitted track starting sets the flash and the count (and the same tempo doesn't count), that both end by themselves, and that a paused deck asks for no repaint after they finish.

## 5. Band, focus and drag

- [x] 5.1 Make `band_layout` and `window_buttons` use `band_panels`; add one band-wide drag area under the controls; dim only the playlist's title bar for focus. Verify: headless tests that dragging the turntable's background and the player's background starts a window drag (not maximized), that a control under the pointer keeps its click, the Section focus scenarios, and the Thirds, Wide and Narrow scenarios of `player-window`.
- [x] 5.2 Rewrite the band, SYNC and deck headless tests that used the old numbers (`SYNC`, `DECK_CLOSE`, `PITCH_UP`, `PITCH_MID`, `DECK_RANGE`, the band-seek and waveform tests). Verify: `cargo test -p ui` passes with no test left reading the old 275 / 104 geometry.

## 6. The window header

- [x] 6.1 In `skin/generate.rs`, add `hdr_l`, `hdr_fill`, `hdr_r` (15 px, grooves as in `titlebar()`), four 11×11 header buttons (`hb_help`, `hb_opts`, `hb_min`, `hb_close`, each with `_p`, the gear a 7×7 cog), and the `±` glyph in the font. Verify: `cargo run -p ui --bin skin-gen`, the skin-on-disk test, and a test that the font draws `±`.
- [x] 6.2 In `layout.rs`, add `HEADER_H` and `TOP_H` and make `band_panels(win_w)` take no corner; in `app.rs`, draw the header (caption, the four buttons, the dimming while the playlist has focus), put the band and playlist under it, and remove the corner. Verify: layout tests (240 / 240 / 240 at 720, 360 / 360 / 380 at 1,100, 227 / 227 / 146 at 600) and `playlist_fit` giving 22 rows at 720 × 435.
- [x] 6.3 Wire the header's behaviour: help opens the shortcuts help, drag moves the window, double-click maximizes or restores, close quits, the gear opens Options (empty slot without digging). Verify: headless tests for each scenario of "Window header" and "Header drag and maximize", the gap click doing nothing, and the Section focus scenarios (the header lit with the player, dimmed with the playlist).
- [x] 6.4 Move every band test's y by the header (clicks in the player, turntable and waveform). Verify: `cargo test -p ui` passes with no test left reading the old corner or the old band top.

## 7. Docs and checks

- [x] 7.1 Update README (the layout tour, the SYNC section, the Tests count), the help panel's SYNC entry ("opens the turntable … or its ×") and add CHANGELOG `## Unreleased` entries with the `changelog` skill; note in `openspec/changes/beatmatch-automix/proposal.md` that SYNC now lives on the turntable. Verify: both diffs mention the thirds, the always-on deck, SYNC on it, and the state line.
- [x] 7.2 Run the AGENTS.md checks (`cargo test --workspace`, clippy `-D warnings`, `fmt --check`, the wasm check of `audio` and `platform`) and `openspec validate console-band --strict`. Verify: all clean.
- [x] 7.3 Look at the real app (`run` skill) at 720 and about 1,100 skin px wide, with SYNC off, locked and out of range. Verify: screenshots show three panels with nothing overlapping, the state line readable, and the list with 23 rows.

## Workflow follow-up

- Archive with `/opsx:archive` once applied.
