## ADDED Requirements

### Requirement: Playback isolation
Analysis SHALL run on low-priority threads with its own file handles and decoders and SHALL never block, delay, or share state with the playback path.

#### Scenario: Start latency unaffected
- **WHEN** a track with no cached score is played
- **THEN** the press-play-to-sound latency is the same as with analysis disabled (< 30 ms)

#### Scenario: No underruns under analysis load
- **WHEN** analysis is running for the current and pre-warmed tracks
- **THEN** playback has zero underruns

### Requirement: Rolling horizon
The analyzer SHALL analyze ahead of the audible position, maintaining at least 120 s of coverage ahead of the playhead when possible, then idle until the horizon shrinks.

#### Scenario: Early coverage
- **WHEN** a 4-minute uncached track starts playing
- **THEN** the score covers the first 32 bars within 1 s

#### Scenario: Long mix memory
- **WHEN** a 2-hour mix plays to the end
- **THEN** analyzer memory stays bounded (feature windows are released; only per-beat score data is retained)

### Requirement: Seek handling
When the playhead moves outside covered regions, the analyzer SHALL restart at the new position with a short context lead-in and SHALL report coverage so consumers know where the score is valid.

#### Scenario: Seek into uncovered region
- **WHEN** the user seeks to 47:00 in a mix covered only to 12:00
- **THEN** analysis restarts near 47:00 and coverage for that region appears within 1 s

### Requirement: Next-track pre-warm
On the engine's pre-warm event, the analyzer SHALL begin analyzing the next track so its opening is covered before it plays.

#### Scenario: Score ready at transition
- **WHEN** a gapless transition to a pre-warmed uncached track occurs
- **THEN** the new track's first 32 bars are already covered

### Requirement: Cache
Scores (including partial coverage) SHALL be cached by content hash and algorithm version, and a cached score SHALL be available immediately on play.

#### Scenario: Second play
- **WHEN** a previously fully analyzed track is played again
- **THEN** its full score is available at play start without re-analysis

#### Scenario: Algorithm upgrade
- **WHEN** the analysis algorithm version changes
- **THEN** old cache entries are ignored and regenerated

### Requirement: Lock-free consumption
Consumers SHALL read the score through immutable snapshots that are swapped atomically, with no locks on the render path.

#### Scenario: Render-thread read
- **WHEN** the visual engine reads the score every frame while the analyzer appends
- **THEN** reads never block
