# Proposal

## Why

Nothing in Diggr moves with the music except the playhead. A level meter (the two bouncing LED
bars that show how loud each channel is right now) and a lamp that blinks on the beat make the
player feel alive. They are also useful when digging: you can see how loud and how dynamic a
preview is at a glance, and whether the analysed beat grid really sits on the beat.

## What Changes

- **A stereo level meter in the mini player**, beside the time: two LED bars, left and right,
  from −48 to 0 dBFS, green, then amber from −12 dB, then red from −3 dB. They jump up at once on a
  hit and fall back smoothly (20 dB in 1.5 s), with a peak dot that hangs for 1 s before falling.
  They show the music as it plays, before the volume control, so they still move with the volume
  at 0.
- **A beat lamp on the turntable**, beside the state line: it flashes on each beat of the playing
  track's analysed beat grid as that beat is heard, brighter on downbeats, and fades in 150 ms. It
  follows the speed heard (SYNC, a held platter) and stays dark when no beats are known.
- **Levels from the audio engine**: the renderer measures each buffer's peak per channel and
  hands it to the app without locks or allocation, keeping the loudest peak until the app reads
  it, so no transient is missed between two frames.
- Not in this change: loudness (LUFS) readouts, clip warnings that stay lit, a meter in the
  separate windows.

## Capabilities

### New Capabilities

- `level-meter`: the mini player's stereo level meter and its ballistics.

### Modified Capabilities

- `audio-tap`: a new requirement for the output levels the renderer publishes.
- `tempo-preview`: a new requirement for the beat lamp on the turntable.

## Impact

- `crates/audio/src/renderer.rs`: a peak per channel per buffer, post-EQ and pre-volume (as the
  tap), published with `fetch_max` on two `AtomicU32` in `Control`; `crates/audio/src/engine.rs`:
  `Engine::levels()` (reads and clears them).
- `crates/ui`: two new skin rects (`lcd_meter` in the mini player, `deck_beat` on the turntable)
  from `skin-gen`; meter ballistics (a pure function, unit-tested); the beat lamp reading the
  score's beats against the clock. Uses `motion-system`'s repaint while animating so the meter's
  fall finishes and then stops repainting.
- Tests: renderer unit tests for the levels (the loudest peak kept between reads, volume
  independence), `rt_alloc` with levels, ballistics unit tests, headless tests for the meter and
  the lamp.
- Docs: README (the mini player, the turntable, test areas and count), help panel, CHANGELOG.
