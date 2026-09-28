## MODIFIED Requirements

### Requirement: Key routing in fullscreen
In fullscreen, transport keys (Z/X/C/V/B, arrows) and the dig keys (Y, N, I) SHALL keep working, the host SHALL keep `T` (timeline strip) and `A` (annotation mode) — and, while annotating, `Space` and `1`–`6` — for itself, and all other keys SHALL be routed to the visual engine.

#### Scenario: Skip track in fullscreen
- **WHEN** the user presses B in fullscreen
- **THEN** the next track plays and fullscreen remains active

#### Scenario: Host keys are not forwarded
- **WHEN** the user presses T in fullscreen
- **THEN** the timeline strip toggles and the scene does not receive the key

#### Scenario: Dig keys in fullscreen
- **WHEN** the user presses Y in fullscreen while a track from Discogs plays
- **THEN** the track is kept, the scene does not receive the key, and fullscreen remains active
