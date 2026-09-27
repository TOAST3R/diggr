## MODIFIED Requirements

### Requirement: Visual engine hosting
In fullscreen mode, the system SHALL give the visual engine the wgpu device, queue, and full surface each frame at display refresh rate, and SHALL let it draw an egui layer (overlay, deck) on top. Each frame SHALL also carry the audible track's latest `SongScore`, when one exists, so scenes can follow beats, sections and upcoming drops.

#### Scenario: Frame pacing
- **WHEN** fullscreen runs on a 60 Hz display
- **THEN** the visual engine is invoked once per vsync

#### Scenario: Score available to the scene
- **WHEN** the audible track has been analyzed
- **THEN** the scene's frame includes its score, and the placeholder visual flashes on the analyzed beats instead of a fixed grid

### Requirement: Key routing in fullscreen
In fullscreen, transport keys (Z/X/C/V/B, arrows) SHALL keep working, the host SHALL keep `T` (timeline strip) and `A` (annotation mode) — and, while annotating, `Space` and `1`–`6` — for itself, and all other keys SHALL be routed to the visual engine.

#### Scenario: Skip track in fullscreen
- **WHEN** the user presses B in fullscreen
- **THEN** the next track plays and fullscreen remains active

#### Scenario: Host keys are not forwarded
- **WHEN** the user presses T in fullscreen
- **THEN** the timeline strip toggles and the scene does not receive the key
