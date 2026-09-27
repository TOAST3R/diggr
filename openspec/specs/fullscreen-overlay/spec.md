# fullscreen-overlay Specification

## Purpose
TBD - created by archiving change visual-engine. Update Purpose after archive.
## Requirements
### Requirement: Track info overlay
In fullscreen, the overlay SHALL show artist, title, a progress line with section ticks colored by kind (provisional ticks faint), and elapsed/total time, in the bundled font, legible on any background, placed above the host's analysis strip when that is visible.

#### Scenario: Content
- **WHEN** "M83 - Midnight City" is at 2:12 of 4:03
- **THEN** the overlay shows "M83", "Midnight City", a progress line at ~54% with section ticks, and "2:12 / 4:03"

### Requirement: Auto-fade
The overlay SHALL be visible for 5 s on entering fullscreen and on track change, fade out over 1 s, and reappear on mouse movement or key press.

#### Scenario: Track change
- **WHEN** the next track starts in fullscreen
- **THEN** the overlay fades in with the new track's info and fades out after 5 s

#### Scenario: Hidden cost
- **WHEN** the overlay is fully faded out
- **THEN** no overlay layout work is performed

