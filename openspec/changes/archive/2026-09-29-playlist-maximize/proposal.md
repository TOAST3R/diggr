## Why

Side by side, the player column (550 points at 2×) and a playlist wide enough for columns don't fit a 1440-point screen: the window runs off both edges. While digging, the playlist and the waveform are what matter; the full player is mostly in the way. Also, scrolling the playlist with a trackpad barely works: small scroll steps are rounded away.

## What Changes

- **Maximized playlist:** a toggle (a ⇔ button in the playlist's title bar, or Shift+P) maximizes the window to the screen's usable area (below the menu bar), and:
  - folds the player into a **thin strip** on the left with only the play state, the elapsed time (mm over ss), prev, play/pause, next, and ⇔ to restore;
  - gives the rest of the window to the playlist (width and rows follow the window, columns included);
  - shows the waveform, when it's on, as a **slim band across the full width** above the playlist, with far more detail than in the player column;
  - hides the EQ and hands the keyboard to the playlist.

  Toggling back restores the previous window frame and layout exactly. The mode is remembered across launches. All shortcuts keep working.
- **Trackpad scrolling fix:** playlist scrolling accumulates partial steps, so slow two-finger scrolling (and its momentum) moves the list row by row instead of being lost.

Not in this change: a horizontal swipe to switch crates, and pinch to change the playlist's text size.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `playlist`: adds the maximized playlist and smooth trackpad scrolling.
- `player-window`: Shift+P in the keyboard shortcuts.
- `waveform-view`: the waveform section becomes a full-width band while the playlist is maximized.

## Impact

- `crates/ui/src/app.rs`: the maximized mode (viewport maximize and restore, layout from the window size, the strip, the band), window-size handling that skips forced sizes while maximized, focus, the fullscreen round trip, and the scroll accumulator.
- `crates/ui/src/layout.rs`: the maximized geometry (strip width, playlist width and rows from the window).
- `crates/ui/src/settings.rs`: `playlist_maximized`.
- `crates/ui/src/skin/generate.rs` and `assets/skin/default/`: the ⇔ button (normal and pressed).
- `apps/native/src/main.rs`: start maximized when the setting is on.
- Help panel and README.
- No audio-path changes.
