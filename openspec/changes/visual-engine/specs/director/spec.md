## ADDED Requirements

### Requirement: Rules file
The director SHALL be configured by `visuals/director.ron` containing ordered rules mapping events (`build_start`, `drop`, `breakdown_start`, `section_change`, `track_change`, `idle(bars)`) to actions (`cut`, `crossfade`, `morph`, `set_macro`, `flash`) with pick expressions (`tag`, `same_as_label`, `highest_rated_unused`, `random_weighted`), reloadable at runtime.

#### Scenario: Edit rules live
- **WHEN** the user changes the breakdown rule from crossfade 1 bar to 4 bars and saves
- **THEN** the next breakdown uses a 4-bar crossfade

### Requirement: Musically quantized transitions
All transitions SHALL start and land on downbeats; a drop cut SHALL land exactly on the drop's first beat.

#### Scenario: Drop cut
- **WHEN** a final drop section begins
- **THEN** the scene cut is presented in the frame where the drop's first beat is audible

### Requirement: Structural memory
When a section label recurs within a track, the director SHALL return to the variant used for that label earlier (unless a rule overrides it).

#### Scenario: Second drop
- **WHEN** the second drop labeled "B" starts
- **THEN** the same variant as the first "B" drop is shown

### Requirement: Default show
The system SHALL ship default rules: build drives `stretch` from `tension`; drop cuts to the highest-rated unused variant with a 1-beat flash; breakdown crossfades to a `calm` variant over 1 bar; 16 idle bars morph to a sibling over 4 bars; track change crossfades over 2 bars.

#### Scenario: Build-up
- **WHEN** a build section plays
- **THEN** the `stretch` macro follows tension and no scene change happens until the drop

### Requirement: Safety against provisional data
The director SHALL act on a drop only when the section is final or `drop_in` has been stable for at least 8 beats.

#### Scenario: Unstable prediction
- **WHEN** a provisional drop prediction changes position
- **THEN** no premature cut occurs
