## ADDED Requirements

### Requirement: Position-stamped sample tap
The system SHALL deliver post-EQ, pre-volume stereo samples from the audio callback to consumers, each chunk stamped with its track id and starting frame index.

#### Scenario: Correlate with clock
- **WHEN** a consumer reads a tap chunk and the playback clock
- **THEN** it can determine which tapped samples are audible now by comparing frame indices

#### Scenario: Volume independence
- **WHEN** the volume is set to 0%
- **THEN** the tap still carries full-level samples

### Requirement: Never blocks audio
The tap SHALL be lossy: if consumers fall behind, the callback SHALL drop tap data rather than wait.

#### Scenario: Slow consumer
- **WHEN** no consumer reads the tap for 5 s
- **THEN** audio continues without glitches and the tap reports dropped chunks when read again
