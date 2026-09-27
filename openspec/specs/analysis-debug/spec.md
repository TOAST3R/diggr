# analysis-debug Specification

## Purpose
TBD - created by archiving change music-analysis. Update Purpose after archive.
## Requirements
### Requirement: Debug timeline strip
In fullscreen, pressing `T` SHALL toggle a timeline strip along the bottom of the screen, drawn by the host over any scene, showing beats, downbeats, sections colored by kind with their labels (provisional sections faint), the tension curve, analysis coverage, and the playhead.

#### Scenario: Toggle in fullscreen
- **WHEN** the user presses T in fullscreen during playback
- **THEN** the strip appears along the bottom and scrolls with playback; pressing T again hides it

#### Scenario: Nothing analyzed yet
- **WHEN** the strip is shown for a track with no score yet
- **THEN** it shows the playhead and an "analyzing…" state instead of stale data

### Requirement: Annotation capture
In fullscreen, pressing `A` SHALL toggle annotation mode. While annotating, `Space` SHALL record a beat tap and `1`–`6` SHALL record a section boundary of kind intro, build, drop, breakdown, groove, or outro. Times SHALL be the audible position from the playback clock, and annotations SHALL be saved as JSON per track (with its path) in `<cache>/annotations/`.

#### Scenario: Mark a drop
- **WHEN** the user is annotating and presses 3 when the drop is heard at 1:32
- **THEN** the track's annotation file contains a `drop` boundary at 1:32 (clock-corrected)

#### Scenario: Keys go to annotation, not the scene
- **WHEN** annotation mode is on and the user presses 1
- **THEN** a boundary is recorded and the key is not passed to the visual scene

### Requirement: Evaluation command
The system SHALL provide `analysis-eval <dir>`, which analyzes every annotated track found in an annotations directory and prints per-track and aggregate beat F-measure (±70 ms) and boundary hit rate (±1 bar).

#### Scenario: Run evaluation
- **WHEN** `analysis-eval` runs on a directory with 10 annotated tracks
- **THEN** it prints a line per track and an aggregate line with both metrics

