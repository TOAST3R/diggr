# audio-tap Specification

## Purpose
TBD - created by archiving change audio-core. Update Purpose after archive.
## Requirements
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


### Requirement: Output levels
The audio callback SHALL measure the peak of each channel of the audio it plays, post-EQ and
pre-volume as the tap is, and SHALL publish the loudest peak since the app last read it, without
locks, allocation or waiting. Reading the levels SHALL clear them.

#### Scenario: Loudest peak kept
- **WHEN** three buffers play with peaks of 0.2, 0.9 and 0.4 before the app reads the levels
- **THEN** the app reads 0.9, and reading again before more audio plays gives 0

#### Scenario: Real-time safe
- **WHEN** levels are measured and read in the allocation-counting test
- **THEN** the audio callback allocates nothing, and the start, seek and underrun targets still hold
