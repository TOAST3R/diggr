# Proposal

## Why

A DJ stops a record by putting a hand on it: it slows to a halt with the pitch falling, and
spins back up when let go. Diggr can only pause, which cuts the sound at once. Holding the
turntable's platter should do what a hand does, and it lays the ground for scratching later.

## What Changes

- **Holding the platter stops the record.** Pressing and holding the turntable's platter while
  a track plays slows it to a stop in 0.4 s, the pitch falling with the speed; releasing it
  brings it back to its speed in 0.3 s. It is momentary: the record runs again as soon as the
  platter is let go. It works with SYNC on or off.
- **A platter stage in the audio engine**, between the queue's audio and the effects: it plays
  the audio at a rate from 0 to 1 with interpolation, consuming the queue only as fast as it
  plays, and is bypassed at rate 1, so playback without a hold is unchanged bit for bit.
- **The clock follows the hold**: the position slows, stops and resumes with the record, and
  the speed it reports is the rate heard.
- **Safety**: a hold ends by itself if the app stops confirming it for 0.5 s, and any seek, stop,
  pause, track change or speed change ends it, so audio is never left silent by a stuck UI.
- Not in this change: scratching (moving the record back and forth, playing in reverse), a
  latching stop, a key for it, and brake or spin-up times the user can set.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `audio-playback`: new requirements for the platter hold, its ending, and the stage being
  transparent at rest.
- `playback-clock`: a new requirement for the clock under a hold.
- `tempo-preview`: a new requirement for holding the turntable's platter.

## Impact

- `crates/audio/src/renderer.rs`: the platter stage (rate slew, interpolation, the clock
  snapshot's speed and position, the tap under a hold), and `Control`'s hold flag and keepalive.
- `crates/audio/src/engine.rs`: `Engine::hold(bool)` and `Engine::hold_keepalive()`, and the
  hold cleared by seek, stop, pause, play, next, previous and `set_speed`.
- `crates/ui/src/app.rs`: the platter's press-and-hold in `deck_section`, the keepalive while held,
  and the platter left undimmed while SYNC is off.
- Tests: engine tests with `ManualSink` (brake and spin-up times, silence while held, the queue not
  consumed, the watchdog, the bypass bit-exact), the `rt_alloc` test with a hold cycle, clock tests,
  a headless test for the platter; `diggr --bench` gains the hold's reaction time.
- Docs: README, CHANGELOG, help.
- Related: `beatmatch-automix` plans two decks in the engine; the stage would then sit per deck.
