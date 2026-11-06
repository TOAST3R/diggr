# Proposal

## Why

Almost nothing in Diggr moves. Menus, windows, dialogs and panels appear and vanish in a single
frame, so clicking feels abrupt and the app feels static. Only the record card's slide is
animated. A shared, cheap way to animate things appearing and disappearing makes the whole app
feel fluid, without giving up the rule that nothing waits.

## What Changes

- **A motion layer for the UI**: springs, ease curves and a few named durations (fast 120 ms,
  medium 180 ms, slow 240 ms), shared by everything that animates, the record card's slide
  included.
- **Things appear and disappear with motion**: context menus and popups fade in and rise 4 px;
  in-window dialogs (Discogs, Browser, Download all tracks, the welcome setup, help, the record
  card) fade and scale from 96%; the filters panel slides 12 px into place;
  separate windows (EQ, spectrogram, insights) fade their content in. Closing plays the same
  motion, faster.
- **Motion never delays an action**: what you open takes clicks and keys from its first
  frame, what you close stops taking them at once, and nothing waits for an animation to
  finish.
- **Repaint while moving, idle when still**: the app repaints at the display's rate (120 Hz
  on ProMotion) while something animates, and goes back to repainting nothing once it settles.
- **Reduce motion**: a setting in the Options menu (the gear), Reduce motion, that turns all of this off; things then
  appear at once, as they do today.

## Capabilities

### New Capabilities

- `ui-motion`: how the UI animates things appearing and disappearing: the timings, the rule
  that motion never delays input, the repaint while animating, and the reduce-motion setting.

### Modified Capabilities

- `player-window`: the Options menu gains Reduce motion.

## Impact

- `crates/ui`: a new `motion` module (spring, easing, per-id transitions keyed on egui ids and
  `ctx.input().time`); the context menus, dialogs, panels and viewports above; the record card's
  slide moved onto it; `Activity`/`repaint_after` gain "animating"; `Settings` gains
  `reduce_motion` (RON, `serde(default)`, fails soft).
- Tests: unit tests for the spring and easing; headless tests that a menu item can be clicked on
  the menu's first frame, that a closing dialog takes no input, and that the app stops
  repainting once motion settles. The headless `Rig` runs with motion off by default so existing
  tests stay deterministic.
- Docs: README (the Reduce motion setting, test areas and count), help panel, CHANGELOG.
- Follow-ups: `motion-feedback` (presses, verdicts, counters, the fader) and `level-meter` build on
  this module.
