## ADDED Requirements

### Requirement: Four bundled scenes
The system SHALL ship `julia_tunnel` (2D fragment), `liquid_feedback` (feedback-based fragment), `kifs_cathedral` (raymarched 3D fragment), and `flame` (compute fractal flame), each with at least two variants and appropriate tags.

#### Scenario: Compute path
- **WHEN** the director selects `flame`
- **THEN** it renders via a compute pass on both native and Chrome WebGPU

#### Scenario: Calm available
- **WHEN** a breakdown rule requests `tag("calm")`
- **THEN** at least one bundled variant matches

### Requirement: Performance
Each starter scene SHALL sustain 60 fps at render scale ≥ 0.75 at 2560×1440 on an Apple M1 or newer.

#### Scenario: Benchmark
- **WHEN** each scene runs for 60 s on an M1 at 1440p
- **THEN** average render scale is ≥ 0.75 with no frames over 2× budget
