## ADDED Requirements

### Requirement: Toggle fullscreen visuals
Pressing `F` SHALL switch to borderless fullscreen visual mode on the current monitor; pressing `F` or `Esc` SHALL return to the classic window at its previous size and position.

#### Scenario: Enter and leave
- **WHEN** the user presses F, then Esc
- **THEN** the app enters fullscreen visuals and then returns to the classic window unchanged

#### Scenario: Playback unaffected
- **WHEN** fullscreen is toggled during playback
- **THEN** audio has no glitches or pauses

### Requirement: Visual engine hosting
In fullscreen mode, the system SHALL give the visual engine the wgpu device, queue, and full surface each frame at display refresh rate, and SHALL let it draw an egui layer (overlay, deck) on top.

#### Scenario: Frame pacing
- **WHEN** fullscreen runs on a 60 Hz display
- **THEN** the visual engine is invoked once per vsync

### Requirement: Key routing in fullscreen
In fullscreen, transport keys (Z/X/C/V/B, arrows) SHALL keep working, and all other keys SHALL be routed to the visual engine.

#### Scenario: Skip track in fullscreen
- **WHEN** the user presses B in fullscreen
- **THEN** the next track plays and fullscreen remains active

### Requirement: Cursor hiding
In fullscreen the mouse cursor SHALL hide after 2 s without movement and reappear on movement.

#### Scenario: Idle cursor
- **WHEN** the mouse does not move for 2 s in fullscreen
- **THEN** the cursor is hidden
