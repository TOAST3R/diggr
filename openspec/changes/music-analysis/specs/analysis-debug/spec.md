## ADDED Requirements

### Requirement: Debug timeline strip
The system SHALL provide a toggleable timeline strip showing beats, downbeats, sections colored by kind with labels, the tension curve, coverage, and the playhead.

#### Scenario: Toggle in fullscreen
- **WHEN** the debug key is pressed in fullscreen
- **THEN** the strip appears along the bottom and scrolls with playback

### Requirement: Annotation capture
The system SHALL let the user mark beats and section boundaries (with a kind) while listening, saving annotations as JSON per track.

#### Scenario: Mark a drop
- **WHEN** the user presses the boundary key and selects "drop" at 1:32
- **THEN** the annotation file for the track contains a drop boundary at the audible time 1:32 (clock-corrected)

### Requirement: Evaluation CLI
The system SHALL provide an evaluation command that analyzes all annotated tracks and reports beat F-measure (±70 ms) and boundary hit rate (±1 bar).

#### Scenario: Run evaluation
- **WHEN** the evaluation command runs on a folder with 10 annotated tracks
- **THEN** it prints per-track and aggregate metrics
