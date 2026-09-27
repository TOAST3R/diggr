# rhythm-analysis Specification

## Purpose
TBD - created by archiving change music-analysis. Update Purpose after archive.
## Requirements
### Requirement: Band onsets
The analyzer SHALL detect kick, snare, and hi-hat onsets and store them as timestamped events.

#### Scenario: Four-on-the-floor
- **WHEN** a house track with a kick on every beat is analyzed
- **THEN** a kick event is present within ±20 ms of each audible kick

### Requirement: Tempo estimation with electronica prior
The analyzer SHALL estimate tempo in 60–200 BPM using a prior centered near 125 BPM and SHALL resolve half/double-tempo ambiguity using hi-hat density and kick periodicity.

#### Scenario: Drum and bass
- **WHEN** a 174 BPM drum and bass track is analyzed
- **THEN** the reported tempo is 174 (not 87)

#### Scenario: House
- **WHEN** a 124 BPM house track is analyzed
- **THEN** the reported tempo is 124 ± 0.1 BPM

### Requirement: Grid-fitted beats
Beats SHALL be derived from a fitted linear grid per tempo segment, and the analyzer SHALL open a new tempo segment when the grid stops matching onsets for at least 4 bars.

#### Scenario: Beat accuracy
- **WHEN** the annotated evaluation set is analyzed
- **THEN** beat F-measure at ±70 ms is at least 0.9

#### Scenario: Tempo change in a mix
- **WHEN** a DJ mix transitions from 122 to 128 BPM
- **THEN** a new tempo segment begins near the transition and beats follow the new tempo

### Requirement: Downbeats and phrase grid
The analyzer SHALL identify the bar downbeat and an 8-bar phrase origin for each tempo segment.

#### Scenario: Bar one
- **WHEN** a track whose sections change on bar lines is analyzed
- **THEN** downbeat events coincide with bar starts where section changes occur

### Requirement: Confidence and beatless fallback
Each tempo segment SHALL carry a confidence value; below threshold the region SHALL be marked beatless (no beat grid emitted).

#### Scenario: Ambient intro
- **WHEN** a track begins with 60 s of beatless pads
- **THEN** that region is marked beatless and the grid starts when the beat enters

