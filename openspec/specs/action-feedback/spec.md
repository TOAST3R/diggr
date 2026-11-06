# action-feedback Specification

## Purpose
How the player answers what the user does (presses, verdicts, changing counts and lists) with
short motion, so every action visibly lands without being delayed by it.

## Requirements

### Requirement: Buttons press
A skinned button SHALL be drawn pressed in by 1 px while the pointer button is down on it, and
SHALL spring back over 120 ms when released. Its action SHALL run when it does today.

#### Scenario: Pressing play
- **WHEN** the user holds the mouse button down on play
- **THEN** the play button is drawn 1 px lower until it is released, then returns in 120 ms (±1 frame)

### Requirement: Lamps bloom
A lamp or lit button that turns on (SYNC, a toggle in the filter bar or the mini player) SHALL
glow beyond its edge by up to 3 px when it lights, the glow settling to its resting look in
180 ms. Turning off SHALL fade the lamp over 120 ms.

#### Scenario: SYNC on
- **WHEN** the user turns SYNC on
- **THEN** the SYNC button lights in that frame with a glow that settles within 180 ms (±1 frame)

### Requirement: Verdicts answer
A verdict SHALL take effect in the frame it is given, and its row SHALL answer it: wanting
sweeps the row with the want colour from left to right over 240 ms; passing moves the row 6 px
aside and back while fading it to its dimmed look over 180 ms; adding to the cart flashes the row
once over 240 ms. The verdict word on the title line SHALL slide in over 120 ms.

#### Scenario: Pass
- **WHEN** the user presses N on the playing track
- **THEN** it is passed and the next track starts in that frame, and its row nudges and fades to dimmed within 180 ms (±1 frame)

#### Scenario: Want
- **WHEN** the user wants a record
- **THEN** its row is swept with the want colour within 240 ms (±1 frame) and then shows as wanted

### Requirement: Counters tick
When the CART total or a record count in the filter bar changes, the number SHALL roll from the
old value to the new one over 180 ms and pulse once. The value used by any action SHALL be the
new one from the frame it changes.

#### Scenario: Adding to the cart
- **WHEN** the CART switch reads "CART 3 · €41.20" and a €12.00 copy is added
- **THEN** it rolls to "CART 4 · €53.20" within 180 ms (±1 frame) and pulses once

### Requirement: Lists change smoothly
Switching to another crate SHALL cross-fade the playlist and slide its rows 12 px in the
direction of travel over 180 ms. A row that appears or moves in the shown crate SHALL ease to its
place over 180 ms. Rows SHALL take clicks at their final place from the first frame.

#### Scenario: Switching crates
- **WHEN** the user picks the next crate in the side panel
- **THEN** its rows are clickable at once, and the switch is drawn as a cross-fade with a 12 px slide over 180 ms (±1 frame)

#### Scenario: A record arrives
- **WHEN** a sent record's tracks arrive in a grouped crate
- **THEN** the rows below move down to make room over 180 ms instead of jumping
