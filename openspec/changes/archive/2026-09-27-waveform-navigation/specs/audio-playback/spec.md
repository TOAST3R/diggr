## ADDED Requirements

### Requirement: Frame-scheduled seek
The engine SHALL accept a seek to take effect at the first of several candidate future positions of the current track that it has not yet sent to the device; it SHALL switch at exactly that frame with at most a 2 ms crossfade, without flushing queued audio, and the clock SHALL report a discontinuity at the switch.

#### Scenario: Scheduled switch
- **WHEN** a seek to 5.0 s is scheduled at the candidates 0.005 s (already sent) and 1.0 s
- **THEN** the output contains the first 1.0 s of the track followed immediately by audio from 5.0 s, with no inserted silence, and the engine reports the jump at 1.0 s

#### Scenario: Too late
- **WHEN** every candidate position has already been sent to the device
- **THEN** the engine reports the jump as missed, and the caller may seek immediately

### Requirement: Gapless loop region
The engine SHALL support a loop region within the current track that wraps from its end to its start sample-accurately and without gaps, until it is cleared.

#### Scenario: Loop wraps
- **WHEN** a loop from 32.0 s to 39.5 s is set while playing at 33 s
- **THEN** the audio from 39.5 s is followed directly by audio from 32.0 s, repeatedly, until the loop is cleared
