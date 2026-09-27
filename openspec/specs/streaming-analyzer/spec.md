# streaming-analyzer Specification

## Purpose
TBD - created by archiving change music-analysis. Update Purpose after archive.
## Requirements
### Requirement: Analysis service API
The system SHALL provide an `AnalysisService` that the player drives with the audible track and position every frame (`playhead`), with tracks about to play (`prewarm`), and from which it reads the latest score of a track (`score`). Scores SHALL be addressed by `TrackRef`; the player maps the clock's `TrackId` to a `TrackRef` using the engine's `TrackLoaded` events.

#### Scenario: Score for the audible track
- **WHEN** a track has been playing for 2 s and the player calls `playhead(track, 2.0)` each frame
- **THEN** `score(track)` returns a snapshot covering at least the first 32 bars

#### Scenario: Cheap per-frame calls
- **WHEN** the UI calls `playhead` and `score` on every frame at 60 Hz
- **THEN** each call returns without waiting on analysis work (no blocking locks, no I/O)

### Requirement: Playback isolation
Analysis SHALL run on low-priority threads with its own file handles and decoders and SHALL never block, delay, or share state with the playback path.

#### Scenario: Start latency unaffected
- **WHEN** a track with no cached score is played
- **THEN** the press-play-to-sound latency is the same as with analysis disabled (< 30 ms)

#### Scenario: No underruns under analysis load
- **WHEN** analysis is running for the current and pre-warmed tracks
- **THEN** playback has zero underruns

### Requirement: Rolling horizon
The analyzer SHALL analyze ahead of the audible position, maintaining at least 120 s of coverage ahead of the playhead when possible, then idle until the horizon shrinks. At most two tracks SHALL be analyzed at once (the audible one and the pre-warmed one); a track that is neither for 30 s SHALL stop being analyzed, keeping what was computed.

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
On the engine's `EngineEvent::PreWarm` (about 30 s before a track ends), the player SHALL call `prewarm`, and the analyzer SHALL begin analyzing that next track so its opening is covered before it plays.

#### Scenario: Score ready at transition
- **WHEN** a gapless transition to a pre-warmed uncached track occurs
- **THEN** the new track's first 32 bars are already covered

### Requirement: Cache
Scores (including partial coverage) SHALL be cached on disk by content hash and algorithm version in the platform cache directory (`WINAMP_CACHE_DIR` overrides), and a cached score SHALL be available immediately on play.

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

