## ADDED Requirements

### Requirement: Atlas-driven rendering
All player, equalizer, and playlist widgets SHALL be drawn from a skin atlas image plus a RON sprite/layout map, with nearest-neighbor filtering and integer scaling.

#### Scenario: Swap atlas
- **WHEN** the atlas PNG is replaced with a recolored version using the same sprite map
- **THEN** the UI renders with the new colors without code changes

#### Scenario: HiDPI
- **WHEN** the app runs on a Retina display
- **THEN** the skin renders at 2× integer scale with crisp pixels

### Requirement: Original default skin
The system SHALL bundle an original default skin that follows the classic layout (dark panels, green LCD text, gold EQ sliders) and SHALL NOT include Winamp's copyrighted bitmaps.

#### Scenario: Bundled assets
- **WHEN** the release bundle is inspected
- **THEN** only project-original skin art is present

### Requirement: Button states
Skinned buttons SHALL show distinct normal and pressed sprites, and toggle buttons SHALL show on/off sprites.

#### Scenario: Toggle EQ button
- **WHEN** the EQ toggle is clicked
- **THEN** its sprite switches to the "on" state and the EQ section appears
