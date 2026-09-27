# track-overview Specification

## Purpose
TBD - created by archiving change waveform-navigation. Update Purpose after archive.
## Requirements
### Requirement: Background overview pass
The system SHALL compute, for each played or pre-warmed track, a stereo overview at the track's native sample rate in the background, without delaying playback start or the playhead analysis.

#### Scenario: Start is unaffected
- **WHEN** the user presses play on an unanalyzed track
- **THEN** playback starts within the fast-start budget and the analysis's first horizon is ready within its budget, while the overview fills in afterwards

#### Scenario: Progressive availability
- **WHEN** the overview pass has covered the first 60 s of a track
- **THEN** the waveform for those 60 s is available to the UI before the pass finishes

### Requirement: Multi-resolution waveform data
The overview SHALL provide per-block minimum/maximum per channel, RMS, and low/mid/high band energy at a base block of 256 frames, plus successively coarser levels (each 4× the previous).

#### Scenario: Peak accuracy
- **WHEN** a track contains a single full-scale sample at 10.000 s
- **THEN** the base-level block containing that frame reports that peak

#### Scenario: Band colour data
- **WHEN** a block contains only a 60 Hz sine
- **THEN** its low-band energy dominates its mid and high energies

### Requirement: Cached and bounded
Overviews SHALL be cached by content hash and loaded instantly on replay, and the memory used SHALL stay bounded for long mixes.

#### Scenario: Replay
- **WHEN** a track whose overview is cached is played again
- **THEN** the whole waveform is available in the first frame of playback

#### Scenario: Two-hour mix
- **WHEN** a 2-hour mix is played
- **THEN** its complete overview uses no more than 16 MB

