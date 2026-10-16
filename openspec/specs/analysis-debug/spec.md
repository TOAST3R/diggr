# analysis-debug Specification

## Purpose
TBD - created by archiving change music-analysis. Update Purpose after archive.
## Requirements
### Requirement: Evaluation command
The system SHALL provide `analysis-eval <dir>`, which analyzes every annotated track found in an annotations directory and prints per-track and aggregate beat F-measure (±70 ms) and boundary hit rate (±1 bar).

#### Scenario: Run evaluation
- **WHEN** `analysis-eval` runs on a directory with 10 annotated tracks
- **THEN** it prints a line per track and an aggregate line with both metrics

