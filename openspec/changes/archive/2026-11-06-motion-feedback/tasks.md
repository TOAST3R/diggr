# Tasks

## 1. Controls

- [x] 1.1 Press-in and spring-back in the skinned button helper. Verify: a motion test for "Pressing play" (1 px down while held, back in 120 ms), and the existing click tests pass.
- [x] 1.2 Bloom and fade for lamps and lit buttons, drawn with epaint glow. Verify: a motion test for "SYNC on" (glow present, settled within 180 ms).
- [x] 1.3 The pitch fader's thumb on a spring: exact under the drag, gliding on RESET, a fit and 8%/16%. Verify: motion tests for "RESET" (target at once, thumb settled within 300 ms, overshoot ≤ 10%) and "Dragging" (thumb under the pointer every frame).
- [x] 1.4 Glow on lit segments and a 120 ms afterglow on segments turning off, for the turntable's digits and the time readout. Verify: a motion test for "A digit changes"; the readable digits test and the paused idle test still pass.

## 2. Digging

- [x] 2.1 Row answers for want (sweep), pass (nudge and fade to dimmed) and cart (flash), and the verdict word sliding in. Verify: motion tests for "Pass" (passed and next track started in the same frame) and "Want".
- [x] 2.2 Rolling and pulsing counts for the CART total and the filter bar's record counts. Verify: a motion test for "Adding to the cart"; actions read the new value at once.
- [x] 2.3 Crate switches cross-fade with a 12 px slide, and rows ease to their place when a grouped crate changes. Verify: motion tests for "Switching crates" (rows clickable at once) and "A record arrives".

## 3. Docs and checks

- [x] 3.1 Update README (the test areas and count) and CHANGELOG. Verify: they describe the feedback.
- [x] 3.2 Run the AGENTS.md checks and `openspec validate motion-feedback --strict`, and confirm an idle, paused window requests no repaint. Verify: all clean.
