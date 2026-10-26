## Why

Diggr has two layouts: the classic window (a player column beside the playlist) and a full-screen one (a band with a mini player and the waveform over a full-width playlist), switched with ⇔ or Shift+P. Digging happens in the playlist, and the full-screen layout gives it the room it needs, so it becomes the only one. With one layout every control has one place, nothing has to be switched, and the specs and tests stop describing everything twice.

## What Changes

- **BREAKING (user-visible): the classic layout is removed.** No player column, no window sized to its sections, no ⇔ buttons, no Shift+P, no `playlist_maximized` setting. The app opens full screen (maximized) in the one layout: the band across the top (mini player, turntable while SYNC is on, waveform), the playlist under it at full width. Restoring the window through the operating system keeps the same layout at the smaller size.
- **The mini player gains shuffle and repeat** as icons in its button row (lit while on, repeat one with a small 1), where ⇔ was, and keeps its 275-pixel width.
- **EQ, waveform and spectrum buttons** sit at the right end of the mini player's LCD row: EQ opens the equalizer in **its own window** (a separate OS window, skinned as today's EQ section, position remembered); waveform shows or hides the waveform like `W`; spectrum opens the spectrogram window like `S`.
- **Minimize and close** move to the band's top-right corner.
- **With the waveform off**, the band shows a thin seek bar in its place, so seeking stays one click away.
- **The mini player's title line shows the verdict flash** (WANTED, PASS…), as the main window's did.
- **Removed with the main window:** the 19-bar spectrum / oscilloscope, the bitrate and sample-rate readouts, the separate seek bar row, the CRATES panel and its toggle (the crate sidebar is always shown at full-screen width), the playlist toggle and the playlist's close button (the playlist is always shown).
- **Kept:** the Options menu (right-click on the mini player, or the playlist footer's gear), Double size, every key except Shift+P, messages at the window's top left, the sidebar, the filter bar, the turntable.
- An old `settings.ron` (with `playlist_maximized`, `show_playlist`, `show_crates`, `show_eq`, `vis`, playlist width and rows) loads without error; fields that no longer mean anything are ignored, and `show_eq` means "EQ window open".

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `player-window`: the mini player is the player (time, title line with the flash, transport, shuffle, repeat, volume, SYNC, EQ / waveform / spectrum buttons); new requirements "Full-screen window" and "Messages"; "Mini visualizer" removed; Shift+P removed; section focus, Options menu and lit transport follow the mini player.
- `playlist`: "Maximized playlist" removed; "Playlist placement and size" becomes the full-width playlist under the band; new "Title bar buttons" (▤ only); "Empty crate hint" without layouts.
- `eq-window`: new "EQ window" (its own OS window); its controls and presets are in that window.
- `crates`: "Crates panel" removed; "Crate menu" and "Crate sidebar" depend on width only.
- `waveform-view`: "Waveform section" is in the band, with a seek bar in its place when off.
- `tempo-preview`: "SYNC control" opens the turntable in the band.
- `filter-bar`: no layouts to name.
- `classic-skin`: "Button states" scenario uses the EQ button and window.
- `spectrogram-window`: "Open the spectrogram" adds the spectrum button.

## Impact

- **`crates/ui/src/app.rs`:** `normal_layout`, the main, EQ (as a section), CRATES panel and player-column drawing, the playlist's column placement, resize handle and toggle, `Action::ToggleMaximized`, `set_maximized`, Shift+P and the ⇔ buttons go; `maximized_layout` and `mini_player` become the layout (shuffle, repeat, EQ, waveform, spectrum buttons, minimize, close, flash, seek bar); the EQ section moves into a deferred viewport like `spectrogram.rs`.
- **`crates/ui/src/layout.rs`:** the player column and window sizing go; the band and the playlist under it stay.
- **`crates/ui/src/spectrum.rs`:** removed with the mini visualizer (its tap reader too, if nothing else uses it).
- **`crates/ui/src/settings.rs`:** `playlist_maximized`, `show_playlist`, `show_crates`, `vis`, playlist width/rows dropped; `show_eq` (EQ window open) and the EQ window's position kept/added.
- **`apps/native/src/main.rs`:** always a maximized first frame.
- **Skin generator:** mini-player sprites and layout (shuffle, repeat, EQ, waveform, spectrum, minimize, close in the band), unused main-window sprites dropped; regenerated with `skin-gen`.
- **`crates/ui/src/help.rs`, README, CHANGELOG (Changed).**
- Many tests in `app.rs` and `headless_tests/` that use the classic layout (grep `normal_layout`, `show_playlist`, `show_crates`, `ToggleMaximized`, main-window rects).
- No change to playback, analysis, digging or the bridge.
