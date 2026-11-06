# Tasks

## 1. Skin

- [x] 1.1 In `skin/generate.rs`, add `seek_track_l`, `seek_track_m`, `seek_track_r` (7 px tall groove) `seek_fill`, and `seek_thumb`, `seek_thumb_p` (9 × 7); add them to `REQUIRED_SPRITES`. Verify: `cargo run -p ui --bin skin-gen` and the skin-on-disk test.

## 2. The bar

- [x] 2.1 In `mini_player`, draw the groove at `SEEK_Y` from x 4 to `w − 4`, the played part tinted, and the thumb at the playing fraction; register the interact rect only with a known duration and a player that isn't stopped. Verify: headless tests that the thumb is at the playing fraction and absent while stopped, and that the bar fits at 227 and 360 px without overlapping the transport (the fit test).
- [x] 2.2 Seek on release (`seek_drag`, `Action::Seek`) and show the tooltip time. Verify: headless tests for click to seek (75% of a 2 s track → 1.5 s), drag then release (no seek before release), the tooltip text, and nothing seeking while stopped.

## 3. Smoother seeks

- [x] 3.1 Hold the thumb at a sent seek until the clock reports it (or 0.5 s). Verify: the headless seek-bar test checks the thumb never jumps back after a click.
- [x] 3.2 De-click hard flushes in the renderer: fade the old audio out and the new in over 5 ms when audio that was playing is replaced. Verify: renderer unit tests (the fade with no step over 0.01, exactly the new audio after it, the clock and epoch; a seek while paused unfaded), engine tests (`a_seek_neither_stops_nor_starts_with_a_click`, next/previous and the seek test checking the audio after the fade), and `rt_alloc`.

## 4. Docs and checks

- [x] 4.1 Update README (the Mini player section, the Tests count) and add a CHANGELOG `## Unreleased` entry. Verify: both mention the seek bar.
- [x] 4.2 Run the AGENTS.md checks (`cargo test --workspace`, clippy `-D warnings`, `fmt --check`, the wasm check) and `openspec validate seek-bar --strict`. Verify: all clean.
