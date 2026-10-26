## Context

Two layouts live side by side in `app.rs`: `normal_layout` (the player column: main section, waveform section with its title bar, EQ section or CRATES panel; the playlist to its right with a resize handle; the window sized to its sections) and `maximized_layout` (a band: `mini_player`, the turntable while SYNC is on, the waveform; the playlist at full width under it; the window maximized by the OS). `settings.playlist_maximized` picks one; ⇔ and Shift+P switch. Messages are a tooltip at the window's top left in both. The verdict flash is drawn only by the main section. The spectrogram is already a separate deferred viewport (`spectrogram.rs`), which is the pattern for the EQ window.

## Goals / Non-Goals

**Goals:**
- One layout, the band over the full-width playlist, always filling the window; the window opens maximized.
- Nothing the user relies on daily is lost: shuffle, repeat, seeking, the EQ, the spectrogram, the flash, messages, the Options menu, minimize and quit.
- Old settings files load (fail soft).

**Non-Goals:**
- No new visualizer: the 19-bar spectrum / oscilloscope goes; the spectrogram window is the spectrum.
- No redesign of the playlist, sidebar, filter bar, turntable or waveform beyond where they sit.
- No change to the 300 ms launch target or playback.

## Decisions

**The band is the player.** `maximized_layout` becomes the only layout (renamed `layout`), `normal_layout` and every player-column section are deleted. The window is always laid out from its own size (`layout::maximized` without the "maximized" condition): band on top, playlist under it at full width and the remaining height. When the OS restores the window, the same layout fills the smaller window; nothing sizes the window any more, and the `max_seen` tracking goes. *Alternative:* keep the classic layout for a restored window: two layouts again.

**Mini player, 275 pixels, two rows.**
- LCD row (y 4–26): state, time (click: elapsed ↔ remaining), then the title line (scrolling, or the verdict flash centred for 1.5 s), then three 17 × 12 icon buttons at its right end: EQ (lit while the EQ window is open), waveform (lit while shown), spectrum (lit while the spectrogram window is open). The title gets the width left (about 140 pixels at 1×).
- Button row (y 34–52): previous, play, pause, stop, next (23 × 18, as now), volume (48 pixels, as now, at x 123), shuffle at x 174 and repeat at x 193 (17 × 12 icons, the main window's shuffle/repeat glyphs drawn at the toggle-icon size: lit while on, repeat one with a small 1), SYNC at x 226 as now. ⇔ (x 262) is gone.
- The skin generator adds the 17 × 12 shuffle, repeat, EQ, waveform and spectrum icons (off/on) and the mini layout entries; the skin test checks the committed files.

**Window controls in the band's top-right corner.** A 28-pixel corner at the band's right end holds minimize and close (9 × 9, the main title bar's sprites), with the waveform (or the seek bar) ending before it. Close quits, as the main window's did. Dragging the corner outside its buttons moves the window when it isn't maximized (the rest of the band is controls, the waveform's seeking, or the mini player's right-click area).

**EQ in its own window.** The EQ section's drawing (`eq_section`: ON, presets, LP knob, curve, preamp, 10 bands) is drawn in an immediate viewport, as the Insights window is (not the spectrogram's deferred one: `eq_section` needs the app's state, which an immediate viewport's closure can borrow directly): a fixed-size, undecorated window at the EQ section's size times the scale, skinned as the section (its title bar drags it, its × closes it), always on top. Where egui has no windows of its own (`embed_viewports`: the headless tests, and the web target later), it floats as an `Area` over the playlist's top left instead. Open/closed is `show_eq` (an old `show_eq: true` opens it); its position is remembered. *Alternative:* an in-window panel: covers playlist rows (rejected by the user).

**Seek without the waveform.** When the waveform is off, the band's waveform area shows a thin seek bar (the main section's seek sprites, stretched) that seeks on release, as the old seek bar did.

**Messages and the flash.** Messages keep their tooltip at the window's top left (over the mini player), so every requirement that says "the main window says…" holds unchanged; the "Messages" requirement records that mapping once instead of editing each capability. The flash moves from the main section's title line to the mini player's.

**Settings fail soft.** `Settings` is `#[serde(default)]` without `deny_unknown_fields`: removed fields (`playlist_maximized`, `show_playlist`, `show_crates`, `vis`, `playlist_width`, `playlist_rows`) are ignored by RON. `show_eq` keeps its name and now means the EQ window is open. The old-file test covers a file holding all of them.

**Shift+P is unbound;** the help panel drops it and the CRATES/playlist toggles' entries.

## Risks / Trade-offs

- [The classic look goes: the full main window, spectrum and oscilloscope] → a deliberate product call; the spectrogram window covers the spectrum, the CHANGELOG says what moved where.
- [A small screen (1280 × 720) at Double size] → the band is 116 pixels high at 2×; the playlist still gets the rest. The filter bar and sidebar already adapt to width.
- [A second OS window for the EQ] → it uses the deferred-viewport path the spectrogram already proves on macOS; closing Diggr closes it.
- [Many tests draw the classic layout] → they are moved to the band's rects or removed with the feature they test; the test count changes, the README says so.

## Migration Plan

The fail-soft settings load is the migration. Rollback is a revert; an older version reading a newer file defaults the missing fields.
