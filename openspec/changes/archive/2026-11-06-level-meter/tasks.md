# Tasks

## 1. Levels from the engine

- [x] 1.1 Measure each block's peak per channel post-EQ/LP in the renderer and publish it with `fetch_max` on two `AtomicU32` in `Control`; add `Engine::levels()` (swap to 0). Verify: renderer unit tests for "Loudest peak kept" and the levels not following the volume; `tests/rt_alloc.rs` passes with levels read between callbacks; `cargo check -p audio --target wasm32-unknown-unknown` is clean.
- [x] 1.2 Check the cost in the callback. Verify: `diggr --bench` still meets the start, seek, hold and underrun targets.

## 2. The meter

- [x] 2.1 Add `lcd_meter` (segments in green, amber and red, lit and unlit) to `skin-gen` and regenerate the skin. Verify: the skin test passes with the committed skin.
- [x] 2.2 Add the ballistics as a pure function (instant attack, 20 dB per 1.5 s release, 1 s peak hold). Verify: unit tests for "Attack", "Release" and "A very short transient".
- [x] 2.3 Draw the meter in the mini player from `Engine::levels()`, settling to empty and idle after a pause; Reduce motion draws the level only. Verify: headless tests for "A full-scale hit", "Volume at zero" and "Paused" (no repaint requested once empty).

## 3. The beat lamp

- [x] 3.1 Add `deck_beat` to `skin-gen` and light it on beats and downbeats passed between frames, fading over 150 ms, reset by seeks. Verify: headless tests for "On the beat", "Under SYNC" and "No beats yet"; the lamp stays dark while the platter is held.

## 4. Docs and checks

- [x] 4.1 Update README (the mini player's meter, the turntable's beat lamp, test areas and count), the help panel and CHANGELOG. Verify: they describe the meter and the lamp.
- [x] 4.2 Run the AGENTS.md checks (including the wasm check of `audio`) and `openspec validate level-meter --strict`. Verify: all clean.
