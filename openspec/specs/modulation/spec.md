# modulation Specification

## Purpose
TBD - created by archiving change visual-engine. Update Purpose after archive.
## Requirements
### Requirement: Routes
A route SHALL connect a source signal through an ordered chain of shapers to a target parameter with a mode (`set`, `add`, `mul`) and a depth.

#### Scenario: Kick to zoom
- **WHEN** a route `kick → envelope(5ms, 120ms) → range(0, 0.35)` adds to `zoom`
- **THEN** zoom jumps on each kick and decays to its base within ~120 ms

### Requirement: Shaper library
The engine SHALL provide the shapers `envelope`, `smooth`, `curve`, `range`, `quantize`, `sample_hold`, `random`, `step_seq`, and `gate`, with times expressed in beats by default and milliseconds when suffixed.

#### Scenario: Per-bar sample and hold
- **WHEN** `random(per: bar) → sample_hold(on: downbeat)` drives a param
- **THEN** the param changes only on downbeats and holds for the bar

#### Scenario: Step sequencer
- **WHEN** `step_seq([1,0,0,1,0,0,1,0,0,0,1,0,0,1,0,0], rate: 1/16)` drives flash
- **THEN** flash fires on steps 1, 4, 7, 11, 14 of each bar

#### Scenario: Gate by section
- **WHEN** a route is gated on `section_kind in [drop]`
- **THEN** it has no effect outside drops

### Requirement: Deterministic randomness
All random values SHALL be derived from the track seed, scene id, route id, and musical period index.

#### Scenario: Replay
- **WHEN** the same track is played twice from the start
- **THEN** every randomized parameter takes the same values at the same beats

### Requirement: Parameter composition order
Each parameter's final value SHALL be computed as variant base → manual fader override → macro contributions → route contributions → clamp to declared range.

#### Scenario: Manual override with modulation
- **WHEN** the user holds the zoom fader at 2.0 and a kick route adds 0.3
- **THEN** zoom reads 2.3 at the kick and returns toward 2.0

