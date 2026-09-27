## ADDED Requirements

### Requirement: Rules file
The director SHALL be configured by `visuals/director.ron` containing ordered rules mapping events (`rise(min_db)`, `fall(min_db)`, `change`, `repeat`, `kind(...)`, `track_change`, `idle(bars)`) to actions (`cut`, `crossfade`, `morph`, `set_macro`, `flash`) with pick expressions (`tag`, `same_as_label`, `highest_rated_unused`, `random_weighted`, `sibling`), reloadable at runtime.

#### Scenario: Edit rules live
- **WHEN** the user changes the `fall` rule from crossfade 1 bar to 4 bars and saves
- **THEN** the next quieter section uses a 4-bar crossfade

### Requirement: Musically quantized transitions
All transitions SHALL start and land on downbeats; a drop cut SHALL land exactly on the drop's first beat.

#### Scenario: Cut on a big energy rise
- **WHEN** a final section boundary with an energy rise of at least 4 dB is reached
- **THEN** the scene cut is presented in the frame where that section's first beat is audible

### Requirement: Structural memory
When a section label recurs within a track, the director SHALL return to the variant used for that label earlier (unless a rule overrides it).

#### Scenario: Repeated section
- **WHEN** a section labeled "B" starts for the second time
- **THEN** the same variant as the first "B" section is shown

### Requirement: Default show
The system SHALL ship default rules keyed on section changes and energy (not on section kinds): a rise of ≥ 4 dB cuts to the highest-rated unused variant with a 1-beat flash; a fall of ≥ 4 dB crossfades to a `calm` variant over 1 bar; a repeated label returns to its variant; any other change morphs to a sibling over 2 bars; 16 idle bars morph over 4 bars; a track change crossfades over 2 bars; `stretch` follows `tension` continuously.

#### Scenario: Rising tension
- **WHEN** tension rises within a section
- **THEN** the `stretch` macro follows it and no scene change happens until the next boundary

### Requirement: Safety against provisional data
The director SHALL act on a section boundary only when the section is final or the boundary has stayed at the same beat for at least 8 beats of analysis.

#### Scenario: Unstable prediction
- **WHEN** a provisional boundary moves between analysis updates
- **THEN** no premature transition occurs
