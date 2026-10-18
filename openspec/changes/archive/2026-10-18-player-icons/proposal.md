## Why

The main section's lower half is crowded with text buttons (WAVE, EQ, PL, SHUFFLE, REP, SYNC) and
carries controls that no longer pay their way: Eject duplicates ⌘O and the footer's ADD, and the
MONO/STEREO lamps say nothing a digger needs. Icons read faster and leave the player calmer.

## What Changes

- **BREAKING** Remove the Eject button and the way it opened files: replacing the Playlist crate
  from a file dialog. Opening files always adds to the shown crate (⌘O, the footer's ADD,
  drag-and-drop). Files on the command line still replace the Playlist crate.
- Remove the MONO and STEREO indicators.
- Draw the waveform, EQ and playlist toggles as pixel-art icons instead of the WAVE, EQ and PL
  labels, with the same lit and unlit states.
- Move SYNC from the transport row into the toggle row, beside the three icons. It stays a text
  button labelled SYNC.
- Draw shuffle and repeat as icons instead of the SHUFFLE and REP/REP1 labels. Repeat one shows
  the repeat icon with a small 1. Both sit in the transport row where Eject, SHUFFLE, REP and SYNC
  were.
- The Options menu (right-click and the footer's gear) is unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `player-window`: no mono/stereo indicators; no open (Eject) button; the waveform, EQ and
  playlist toggles, shuffle and repeat are icons; SYNC sits with the toggles.
- `crates`: the Playlist crate is replaced only by files on the command line, not by Eject.

## Impact

- `crates/ui/src/skin/generate.rs`: new icon sprites and layout rects; the `eject`, `mono`,
  `stereo` and the text toggle/shuffle/repeat sprites go. `assets/skin/default/` is regenerated
  with `cargo run -p ui --bin skin-gen`.
- `crates/ui/src/app.rs`: the main section's drawing, `Action::Eject`, `Open::Replace` and the
  test that uses it.
- `README.md`: the Eject mentions (Playlist crate, double-click retry) and the test count.
- No change to audio, analysis, dig or the playback path.
