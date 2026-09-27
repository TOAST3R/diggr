## MODIFIED Requirements

### Requirement: Gapless playback with pre-warm
The system SHALL pre-open and pre-decode the next queued track before the current one ends, and SHALL transition with no inserted silence. With automix on, the transition SHALL be the planned mix, and pre-warm SHALL start early enough for the next track's beat grid to be analyzed before the mix begins.

#### Scenario: Gapless transition
- **WHEN** track N ends, track N+1 is next in the queue, and automix is off
- **THEN** the first frame of N+1 follows the last frame of N with no gap

#### Scenario: Pre-warm notification
- **WHEN** the current track has less than 30 s remaining and automix is off
- **THEN** the next track is opened, its first second decoded, and a pre-warm event is emitted for other subsystems

#### Scenario: Early pre-warm for a mix
- **WHEN** automix is on with 32-bar mixes at 124 BPM
- **THEN** the next track is pre-warmed at least 82 s before the current track's end
