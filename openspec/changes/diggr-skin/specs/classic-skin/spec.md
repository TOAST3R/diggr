## MODIFIED Requirements

### Requirement: Original default skin
The system SHALL bundle an original default skin that follows the classic layout with Diggr's own look: warm graphite panels, amber LCD text on black, and title bars with fine groove lines either side of the caption instead of stripes. It SHALL NOT include any third-party player's copyrighted bitmaps.

#### Scenario: Bundled assets
- **WHEN** the release bundle is inspected
- **THEN** only project-original skin art is present

#### Scenario: Diggr look
- **WHEN** the player opens with the default skin
- **THEN** the panels are graphite, the time digits and title line are amber on black, and the main and EQ title bars show groove lines around the caption, with no gold stripes

## ADDED Requirements

### Requirement: LCD colour from the skin
Every LCD text the app draws (the title line, kbps and kHz, and the playlist footer's BPM range, filters, cart switch and info) SHALL use the skin's LCD colour. A skin file that names no LCD colour SHALL get the earlier green (0, 236, 0).

#### Scenario: Amber default
- **WHEN** a track plays with the default skin
- **THEN** its title line, kbps and kHz are drawn in the default skin's amber

#### Scenario: Older skin file
- **WHEN** a skin file without an LCD colour is loaded
- **THEN** its LCD text is drawn in green (0, 236, 0)
