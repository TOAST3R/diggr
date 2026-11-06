# Proposal

## Why

With `motion-system`, things appear and disappear fluidly, but what you *do* still lands without
a response: buttons don't press, a verdict just changes a row's colour, the CART total jumps, and
the pitch fader teleports when RESET or SYNC moves it. Feedback in motion tells you the action
landed and what it changed, the way a hardware control would.

## What Changes

- **Buttons press**: skinned buttons (transport, SYNC, RESET, 8%/16%, the mini player's icons)
  press in by 1 px while held, and a lamp that turns on (SYNC, a toggle) blooms with a soft glow
  that settles in 180 ms.
- **Verdicts answer**: wanting a record sweeps its row with the want colour; passing nudges the
  row and fades it to its dimmed look; adding to the cart flashes the row. The verdict word on the
  title line slides in instead of popping.
- **Counters tick**: the CART total and the record counts in the filter bar roll to their new
  value and pulse once when they change.
- **Lists change smoothly**: switching crates cross-fades the playlist and slides it 12 px in the
  direction of travel; rows that appear or move in a grouped crate ease to their place.
- **The pitch fader glides**: when RESET, a SYNC fit or the 8%/16% button moves the fader's thumb,
  it springs to its new place with a slight overshoot; under the hand it still follows exactly.
- **The LCDs glow**: lit segments on the turntable's digits and the time readout have a faint glow,
  and a segment that turns off fades over 120 ms instead of vanishing.

## Capabilities

### New Capabilities

- `action-feedback`: how the UI answers presses, verdicts, counters and list changes with motion.

### Modified Capabilities

- `tempo-preview`: a new requirement for the fader's thumb gliding when it is moved for you, and
  for the glow and afterglow of the turntable's digits.

## Impact

- `crates/ui`: uses `motion` (from `motion-system`) at the skinned button helpers, the row drawing
  in the playlist, the verdict flash, the filter bar's counts and CART switch, crate switching, the
  pitch fader and the LCD digit drawing.
- Depends on `motion-system` being implemented first (its module, repaint while animating, and
  Reduce motion, which also turns all of this off).
- Tests: headless motion tests for each scenario; the existing tests run with motion off.
- Docs: README (the turntable and digging sections, test areas and count), CHANGELOG.
