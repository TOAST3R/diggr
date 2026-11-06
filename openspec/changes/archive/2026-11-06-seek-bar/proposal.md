# Proposal

## Why

Seeking is only possible on the waveform's thin overview row, which is easy to miss and is
never read as a progress bar. The player has no sign of how far into a track it is beyond the
time digits, and the 10 skin pixels under its transport buttons are empty.

## What Changes

- The mini player gets a **seek bar** under its button row, across its width: a groove with a
  thumb that follows playback. Clicking or dragging it seeks, on release, to the point under
  the pointer, and a tooltip names the time it would seek to.
- With nothing playing, or a track whose length isn't known, the bar is inert and shows no thumb.
- After a click the thumb stays where it was sent until playback gets there, instead of jumping
  back for a moment.
- **Seeks no longer click.** A seek, next or previous while audio plays fades the old audio out
  over 5 ms and the new in over 5 ms, instead of cutting one and starting the other abruptly;
  this applies to every seek (the bar, the waveform, ← / →, the record card).
- Not in this change: holding ⏮ / ⏭ to wind (they stay previous and next), scrubbing with
  audio while dragging, and any change to the waveform's own seeking.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `player-window`: the mini player's "Transport and sliders" requirement gains the seek bar,
  and seeking is on the waveform or the bar.
- `audio-playback`: "Transport controls" says how a seek, next or previous sounds (faded, not
  cut).

## Impact

- `crates/ui/src/app.rs`: `mini_player` (the bar, its drag state, the seek on release, the thumb
  held at a sent seek).
- `crates/audio/src/renderer.rs`: the de-click fade on a hard flush of audio that was playing.
- `crates/ui/src/skin/generate.rs`, `mod.rs` and `assets/skin/default/`: a three-piece groove
  (`seek_track_l`, `_m`, `_r`) and a thumb (`seek_thumb`, `seek_thumb_p`); regenerate with
  `cargo run -p ui --bin skin-gen`.
- Tests: headless tests for the bar; the mini-player fit test gains the bar's row.
- Docs: README (the Mini player section, the test count), CHANGELOG.
