# Design

## Context

The renderer (`crates/audio/src/renderer.rs`) runs EQ and the low-pass on each block in
`scratch`, writes the tap from it (post-EQ, pre-volume), then applies volume into the device
buffer. State crosses to the app through atomics on `Control`; the audio callback must not
allocate, lock or block (`tests/rt_alloc.rs`). The app repaints every 16 ms while playing
(`repaint_after`), and `motion-system` adds repainting while anything animates. The analyser's
`SongScore` holds beats and downbeats in track time, and the clock gives the audible position.

## Goals / Non-Goals

**Goals:**
- No transient lost between frames, at a cost of a few operations per sample in the callback.
- Ballistics as a pure function, so they are unit-tested without a display.

**Non-Goals:**
- RMS or LUFS readouts; true-peak (inter-sample) measurement.
- Reading the levels from the tap: it has one reader (the app's spectrogram and analysis feed),
  and it is lossy by design, so peaks could be dropped.

## Decisions

**Peaks via `fetch_max` on float bits.** For non-negative `f32`, the IEEE bit pattern orders like
the value, so `AtomicU32::fetch_max(peak.to_bits())` keeps the loudest peak since the last read,
lock-free and allocation-free. The renderer computes `max(|x|)` per channel over each block after
EQ/LP (where the tap reads), once per block. `Engine::levels()` does `swap(0)` on both and returns
`[f32; 2]` as linear peaks. *Alternative:* storing the latest buffer's peak: at ~94 buffers a
second and 60 frames a second, a third of the peaks would never be seen.

**Ballistics in the UI.** `meter::step(state, peak_db, dt) -> state`: a bar rises to `peak_db` at
once and otherwise falls at 20/1.5 dB/s; the peak dot holds 1 s, then falls at the same rate. Below
−48 dB a bar is empty. When every bar and dot is empty and nothing plays, the meter reports
settled, so `animating` is false and the app can go idle.

**Placement and skin.** `skin-gen` gains `lcd_meter` in the mini player beside the time (two rows
of 24 segments, 2 px tall, 1 px gaps) with lit and unlit segment sprites in the three colours, and
`deck_beat` beside the turntable's state line (an LED sprite, lit and unlit). The committed skin is
regenerated, and the skin test checks it.

**The beat lamp.** Each frame while playing, the app takes the score's beats between the last
frame's audible position and this frame's (from the clock, so speed and holds are accounted for):
if any, the lamp lights at full (downbeat) or 70% (beat) and fades over 150 ms. A held platter
reports a still position, so no beat passes and the lamp stays dark. A seek resets the last
position so jumping doesn't fire the beats in between.

## Risks / Trade-offs

- [Per-sample max in the callback] → One `abs` and `max` per sample per channel on audio already
  in cache; `diggr --bench` shows the targets still hold.
- [A 16 ms frame can be up to one frame late for a beat] → The spec says "first frame at or after",
  which is what a display can do; the clock is within 2 ms of the device.
- [The meter keeps the app repainting after a pause] → Only for the fall (the bars ≤ 3.6 s, the
  peak dots ≤ 4.6 s after their hold); then idle.
- [Skin space in the mini player is tight] → The meter is two thin rows; the exact x is fixed in
  `skin-gen` against the time digits and the LCD icons.

## Migration Plan

New skin rects are generated; old user skins are not a concern (the skin is generated and
shipped). No data changes.
