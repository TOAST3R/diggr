## MODIFIED Requirements

### Requirement: Gapless playback with pre-warm
The system SHALL pre-open and pre-decode the next queued track before the current one ends, and SHALL transition with no inserted silence. In SYNC mode, the transition SHALL be the planned mix, and the next track SHALL be pre-warmed at least 60 s before the planned mix starts.

#### Scenario: Gapless transition
- **WHEN** track N ends and track N+1 is next in the queue, with SYNC off
- **THEN** the first frame of N+1 follows the last frame of N with no gap

#### Scenario: Pre-warm notification
- **WHEN** the current track has less than 30 s remaining
- **THEN** the next track is opened, its first second decoded, and a pre-warm event is emitted for other subsystems

#### Scenario: Early pre-warm for a mix
- **WHEN** SYNC is on and a mix is planned to start at 3:20 of the current track
- **THEN** the next track is pre-warmed no later than 2:20

## ADDED Requirements

### Requirement: Mix plan execution
The engine SHALL execute a mix plan: starting at an exact output frame of the outgoing track, it SHALL play the incoming track from a given entry point at a given tempo ratio (pitch following tempo), apply the plan's volume and per-band gain envelopes to both tracks, and continue with the incoming track alone at that ratio after the mix, all without underruns and without work in the audio callback beyond a per-segment speed value.

#### Scenario: Hand-written plan
- **WHEN** a plan mixes track B, entering at 60.0 s at ratio 0.984, into track A from A's 180.0 s over 16 bars
- **THEN** the output contains A alone up to 180.0 s, the mixed tracks for 16 bars, then B alone at ratio 0.984, with zero underruns

#### Scenario: Missed plan
- **WHEN** a plan's start has already been sent to the device
- **THEN** the engine reports it as missed and keeps playing the current track
