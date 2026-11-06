# Tasks

## 1. The target in hundredths

- [x] 1.1 In `sync.rs`, add `target_cents` beside `target`, the `target_bpm` / `set_target_bpm` accessors, `nudge` in hundredths, and the range clamp in hundredths; make `speed` and `fit` use them. Verify: unit tests for rounding to 0.01, the 40–250 limits, the clamp (123 at ±8% → 132.84 at the top), an old settings string with `target: 124` only, and that a save writes both fields.
- [x] 1.2 Move every use of `sync.target` in `app.rs` to the accessors; `SyncTarget` and `SyncNudge` carry hundredths; the SYNC messages use `format::bpm_2`. Verify: the existing SYNC tests pass with the new numbers (the fader's top reads 132.84) and a test for the message "plays at 126.35 BPM".

## 2. Fine steps

- [x] 2.1 Shift+drag on the fader (relative, 0.01 per skin pixel), Shift+scroll (0.01 per 4 px) and Alt+↑ / Alt+↓ (player focused). Verify: headless tests for each scenario of "Fine tempo steps", including kept within the range and Shift+↑ unchanged.

- [x] 2.2 With SYNC on and the player focused, plain ↑ / ↓ step the tempo by 0.1 BPM instead of the volume. Verify: the headless fine-steps test (126.00 → 126.10 after ↑ ↑ ↓, the volume unchanged; with SYNC off ↓ lowers the volume) and the "Arrows set the tempo under SYNC" scenario.

## 3. Typing and display

- [x] 3.1 Draw the hundredths at `deck_fine` and move the "BPM" label; add the layout entry in `skin/generate.rs`. Verify: `skin-gen`, the skin-on-disk test, and a headless test that the deck draws "126" and ".35" for 126.35.
- [x] 3.2 The typing editor: open on a click on the digits (SYNC on, BPM known), accept digits, a point or comma, Backspace, Enter (with clamp and message), Esc and a click elsewhere; block every other shortcut while open. Verify: headless tests for each scenario of "Typing the tempo".

## 4. Spike

- [x] 4.1 Measure the analyzer's tempo error on 8 synthesized tracks with fractional tempos and write the results and a recommendation in this change's `design.md`. Verify: the numbers are in `design.md`.

## 5. Docs and checks

- [x] 5.1 Update the help panel's SYNC entries, README (the SYNC section, the Tests count) and add CHANGELOG entries. Verify: they mention typing, Shift+drag, Shift+scroll and Alt+arrows.
- [x] 5.2 Run the AGENTS.md checks and `openspec validate fine-tempo --strict`. Verify: all clean.
