# Spec Delta

## ADDED Requirements

### Requirement: The fader glides
When the pitch fader's thumb is moved for the user (RESET, a new record fitted under SYNC, the
8%/16% button rescaling it), it SHALL spring to its new place, overshooting by no more than 10% of
the move and settling within 300 ms. The target and the speed heard SHALL change in the frame
of the action. Under the user's drag the thumb SHALL follow the pointer exactly.

#### Scenario: RESET
- **WHEN** SYNC is on at +4% and the user clicks RESET
- **THEN** the target becomes the record's own tempo in that frame, and the thumb springs to the centre detent, settling within 300 ms

#### Scenario: Dragging
- **WHEN** the user drags the fader
- **THEN** the thumb is drawn under the pointer on every frame, with no lag

### Requirement: Glowing digits
The turntable's digits and the time readout SHALL draw a faint glow around lit segments, and a
segment that turns off SHALL fade out over 120 ms. Unlit segments SHALL stay no brighter than a
quarter of the lit ones (see "Readable digits and idle cost"), and the digits SHALL ask for no
repaint once every fade has finished.

#### Scenario: A digit changes
- **WHEN** the target changes from 126 to 127
- **THEN** the segments of 6 that 7 doesn't use fade out within 120 ms (±1 frame), and the new segments are lit at once
