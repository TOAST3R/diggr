# structure-analysis Specification

## Purpose
TBD - created by archiving change music-analysis. Update Purpose after archive.
## Requirements
### Requirement: Phrase-snapped section boundaries
The analyzer SHALL detect section boundaries from beat-synchronous feature novelty and SHALL snap them to the nearest phrase boundary (8 bars) when within ±2 bars, otherwise to the nearest bar.

#### Scenario: Boundary accuracy
- **WHEN** the annotated evaluation set is analyzed
- **THEN** at least 70% of annotated boundaries have a detected boundary within ±1 bar

### Requirement: Section labels
Sections SHALL receive labels such that musically similar sections share a label (e.g., both choruses/drops labeled "B"), assigned online with bounded memory.

#### Scenario: Repeated drop
- **WHEN** a track has two similar drops separated by a breakdown
- **THEN** both drops receive the same label

### Requirement: Section kinds
Each section SHALL be classified as one of intro, build, drop, breakdown, groove, or outro.

#### Scenario: Drop after build
- **WHEN** the kick returns at high energy after a riser build
- **THEN** the build section is classified `build` and the following section `drop`

#### Scenario: Breakdown
- **WHEN** the kick disappears and energy falls below the track median for 16 bars
- **THEN** the section is classified `breakdown`

### Requirement: Tension and drop countdown
The score SHALL provide a per-beat `tension` value in [0, 1] that rises through builds preceding a drop and falls to 0 at the drop, and a `drop_in` value giving beats until the next known drop.

#### Scenario: Countdown
- **WHEN** the playhead is 8 beats before a detected drop
- **THEN** `drop_in` reads 8 and `tension` is near its maximum

### Requirement: Finalization
Sections SHALL be marked provisional until the analysis horizon is at least 32 bars past them, then final.

#### Scenario: Provisional near horizon
- **WHEN** a section boundary is within 32 bars of the coverage edge
- **THEN** the section is flagged provisional

