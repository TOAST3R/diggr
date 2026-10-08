# classic-skin Specification

## Purpose
TBD - created by archiving change classic-ui. Update Purpose after archive.
## Requirements
### Requirement: Atlas-driven rendering
All player, equalizer, and playlist widgets SHALL be drawn from a skin atlas image plus a RON sprite/layout map, with nearest-neighbor filtering and integer scaling.

#### Scenario: Swap atlas
- **WHEN** the atlas PNG is replaced with a recolored version using the same sprite map
- **THEN** the UI renders with the new colors without code changes

#### Scenario: HiDPI
- **WHEN** the app runs on a Retina display
- **THEN** the skin renders at 2× integer scale with crisp pixels

### Requirement: Original default skin
The system SHALL bundle an original default skin that follows the classic layout with Diggr's own look: warm graphite panels, amber LCD text on black, and title bars with fine groove lines either side of the caption instead of stripes. It SHALL NOT include any third-party player's copyrighted bitmaps.

#### Scenario: Bundled assets
- **WHEN** the release bundle is inspected
- **THEN** only project-original skin art is present

#### Scenario: Diggr look
- **WHEN** the player opens with the default skin
- **THEN** the panels are graphite, the time digits and title line are amber on black, and the main and EQ title bars show groove lines around the caption, with no gold stripes

### Requirement: Button states
Skinned buttons SHALL show distinct normal and pressed sprites, and toggle buttons SHALL show on/off sprites.

#### Scenario: Toggle EQ button
- **WHEN** the EQ toggle is clicked
- **THEN** its sprite switches to the "on" state and the EQ section appears

### Requirement: LCD colour from the skin
Every LCD text the app draws (the title line, kbps and kHz, and the playlist footer's BPM range, filters, cart switch and info) SHALL use the skin's LCD colour. A skin file that names no LCD colour SHALL get the earlier green (0, 236, 0).

#### Scenario: Amber default
- **WHEN** a track plays with the default skin
- **THEN** its title line, kbps and kHz are drawn in the default skin's amber

#### Scenario: Older skin file
- **WHEN** a skin file without an LCD colour is loaded
- **THEN** its LCD text is drawn in green (0, 236, 0)

