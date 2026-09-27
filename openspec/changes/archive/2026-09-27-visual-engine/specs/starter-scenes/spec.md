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
Each starter scene SHALL sustain the display refresh rate at render scale ≥ 0.75 in fullscreen on an Apple M-series Mac (native display resolution).

#### Scenario: Benchmark
- **WHEN** each scene runs for 15 s in fullscreen via the visual benchmark
- **THEN** average render scale is ≥ 0.75 and fewer than 1% of frames exceed twice the frame budget
