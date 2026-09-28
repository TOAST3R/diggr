## Why

The playlist is squeezed under the player at a fixed 275-point width, so long dig names (catalog number, artist, title, BPM) are cut off, and the list can only grow downwards. ↑ and ↓ always change the volume, so there is no way to walk the list from the keyboard. And the balance slider takes a prime spot for a control DJs never touch, while the waveform, which they use constantly, can only be toggled with `W`.

## What Changes

- **Side-by-side layout:** the playlist moves to the right of a fixed left column (main, waveform, EQ), in the same borderless window. Hiding the playlist shrinks the window back to the left column.
- **Two-way resize:** the playlist's corner handle resizes it freely in width (no steps, from the skin's 275-point minimum) and in whole rows in height. The height is at least the left column's. Width and rows are remembered.
- **BREAKING: balance removed.** The balance slider is removed. A saved non-zero balance is reset to centre on first launch, so nobody is left stuck off-centre. Its place holds a **waveform toggle button**, lit when the waveform section shows (the same as `W`).
- **Section focus:** the player (main, waveform, EQ) and the playlist each take keyboard focus when clicked. The focused side's title bar is drawn lit and the other's dimmed, as Winamp 2 did with its windows. `Tab` switches focus. The app starts with the player focused.
- **Arrow keys follow focus:** with the player focused, ↑ and ↓ change the volume (as today). With the playlist focused, they move a keyboard cursor.
- **Keyboard cursor in the playlist:**
  - ↑ / ↓ move by one entry;
  - Shift+↑ / Shift+↓ extend the selection;
  - PgUp / PgDn move by one page, Home / End to the first or last entry;
  - Enter plays the entry under the cursor.

  The cursor is drawn as an outline, separate from the selection highlight, and the list scrolls to keep it visible. The first ↓ in an untouched list lands on the playing entry.
- **Show the playing entry:**
  - `P` scrolls the playlist to the playing entry and puts the cursor on it.
  - When the track changes and the previous playing entry was on screen, the list follows the new one.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-window`: the balance slider is replaced by a waveform button; the arrow keys depend on focus; `P` and `Tab` are added; a section-focus requirement.
- `playlist`: placement to the right, two-way resize, keyboard cursor navigation, following the playing entry.
- `waveform-view`: the section can also be toggled with the main window's waveform button.

## Impact

- `crates/ui/src/app.rs`:
  - `window_size`, section placement (a left column plus the playlist);
  - the resize handle, which now works in 2D;
  - key routing by focus;
  - removal of the `Action::Balance` path.
- `crates/ui/src/settings.rs`: `playlist_width` is added, and `balance` is dropped and reset.
- `crates/ui/src/skin/generate.rs` and `assets/skin/default/`:
  - a waveform button (on and off);
  - dimmed title-bar variants;
  - horizontal fill tiles for the playlist's top and bottom bars.

  Regenerate the skin with `skin-gen`.
- `crates/ui/src/help.rs` (the shortcuts panel) and README (layout, keys, test count).
- Overlaps the `beatmatch-automix` change, which also edits the main window's button row and the player-window requirements "Transport and sliders" and "Keyboard shortcuts".
- No audio-path changes. The engine's `set_balance` stays, and the UI no longer calls it.
