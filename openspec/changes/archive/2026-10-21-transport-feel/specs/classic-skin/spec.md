## MODIFIED Requirements

### Requirement: Button states
Skinned buttons SHALL show distinct normal and pressed sprites, and toggle buttons SHALL show on/off sprites. The play, pause and stop buttons SHALL each have an `_on` sprite (`play_on`, `pause_on`, `stop_on`), drawn by the skin generator in the same frame as the normal sprite with the glyph in the lit colour the shuffle and repeat icons use.

#### Scenario: Toggle EQ button
- **WHEN** the EQ toggle is clicked
- **THEN** its sprite switches to the "on" state and the EQ section appears

#### Scenario: Transport sprites generated
- **WHEN** the skin generator runs
- **THEN** the atlas holds `play_on`, `pause_on` and `stop_on` of the same size as `play`, `pause` and `stop`, and the skin test passes
