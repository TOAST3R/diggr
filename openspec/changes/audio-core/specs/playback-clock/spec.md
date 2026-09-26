## ADDED Requirements

### Requirement: Audible position clock
The system SHALL publish, from the audio callback and without locks, the data needed to compute the frame currently audible at the speaker: track id, frames handed to the device, callback host timestamp, output latency, sample rate, and playback state.

#### Scenario: Interpolated read
- **WHEN** a reader queries the clock between two callbacks
- **THEN** it receives an audible frame position interpolated from the last callback's timestamp, compensated for output latency

#### Scenario: Accuracy
- **WHEN** a test signal with known click positions is played and captured via loopback
- **THEN** the clock's audible position matches the captured clicks within ±2 ms

### Requirement: Monotonic and track-aware
The clock SHALL be monotonic within a track and SHALL switch track id at the exact audible frame of a gapless boundary.

#### Scenario: Gapless boundary
- **WHEN** playback crosses from track N into N+1 gaplessly
- **THEN** the clock reports N+1 with position 0 at the moment N+1's first frame becomes audible, not when it was decoded

#### Scenario: Seek discontinuity
- **WHEN** a seek occurs
- **THEN** the clock reports a discontinuity flag and the new position once post-seek audio is audible

### Requirement: Paused clock
The clock SHALL report a frozen position while paused or stopped.

#### Scenario: Pause freezes time
- **WHEN** playback is paused
- **THEN** successive reads return the same audible position

### Requirement: User A/V offset
The system SHALL allow a user-configurable offset of ±50 ms applied to reader-computed positions.

#### Scenario: Offset applied
- **WHEN** the offset is set to +20 ms
- **THEN** all clock readers report positions 20 ms later than before
