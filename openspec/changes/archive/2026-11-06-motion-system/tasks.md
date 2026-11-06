# Tasks

## 1. The motion module

- [x] 1.1 Add `crates/ui/src/motion.rs`: `Transition` progress keyed by id on `ctx.input().time`, the easings, a critically damped `Spring`, the durations and `animating`. Verify: unit tests that a transition reaches 1 at its duration (±1 frame) and 0 on its close duration, that the spring settles within 1% of its target without overshooting past 5%, and that reduce motion jumps at once.
- [x] 1.2 Add `Settings::reduce_motion` (default off, `serde(default)`) and the Reduce motion item in the Options menu; have the headless `Rig` start with it on. Verify: a settings test that a file without the field loads as off; a headless test that the item toggles and persists.
- [x] 1.3 Add `animating` to `Activity` and repaint at the next vsync while animating. Verify: `repaint_after` unit tests; a headless test that after the last animation settles with nothing playing no repaint is requested.

## 2. Appearing and disappearing

- [x] 2.1 Context menus and popups: fade and 4 px rise on open (120 ms), fade on close (80 ms), drawn by layer transform with input at the final place. Verify: motion tests for "A context menu opens", "A menu closes" and "Clicking a menu item at once".
- [x] 2.2 In-window dialogs (Discogs, Browser, Download all tracks, welcome setup, help): fade and 96%→100% scale (180 ms in, 120 ms out), non-interactive while closing. Verify: motion tests for "The help panel opens" and "A closing dialog takes no clicks".
- [x] 2.3 Panels and the card: the filters panel slides 12 px into place (240 ms in, 160 ms out); the record card opens as a dialog; move the card's record-to-record slide onto the module's easing with its timing. Verify: a motion test for "The filters panel opens"; the record card's existing tests pass unchanged.
- [x] 2.4 Separate windows (EQ, spectrogram, insights) fade their content in over 180 ms. Verify: a motion test for "The EQ opens".
- [x] 2.5 Check that motion leaves playback alone. Verify: a motion test for "Playback is untouched"; `diggr --bench` latency targets still pass.

## 3. Docs and checks

- [x] 3.1 Update README (Reduce motion in the Options menu, the test areas, the test count), the help panel and CHANGELOG. Verify: they describe the motion and the setting.
- [x] 3.2 Run the AGENTS.md checks and `openspec validate motion-system --strict`. Verify: all clean.
