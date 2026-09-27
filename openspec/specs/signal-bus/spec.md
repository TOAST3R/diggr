# signal-bus Specification

## Purpose
TBD - created by archiving change visual-engine. Update Purpose after archive.
## Requirements
### Requirement: Musical time
The engine SHALL derive musical time (beats elapsed as a float, plus beat, bar, and phrase phases) from the playback clock's audible position and the SongScore, and shaders SHALL NOT receive wall-clock time.

#### Scenario: Pause freezes visuals
- **WHEN** playback is paused
- **THEN** musical time stops and the scene's motion freezes (feedback decay may continue only if the scene opts in)

#### Scenario: Tempo scaling
- **WHEN** a scene rotates one turn per bar
- **THEN** it rotates faster on a 174 BPM track than on a 120 BPM track

### Requirement: Typed signals
The engine SHALL expose phase signals (`beat`, `bar`, `phrase`, `section_progress`), triggers (`kick`, `snare`, `hat`, `downbeat`, `drop`, `section_change`), continuous signals (`energy`, `bass`, `mid`, `treble`, `brightness`, `tension`, `drop_in`), and discrete signals (`section_kind`, `section_label`, `beat_confidence`).

#### Scenario: Signal snapshot
- **WHEN** a frame begins
- **THEN** all signals are sampled once into an immutable snapshot used by the whole frame

### Requirement: Zero-latency triggers
Triggers present in the SongScore SHALL fire when the audible position crosses their timestamp, and all triggers crossed since the previous frame SHALL be delivered.

#### Scenario: Kick on the beat
- **WHEN** a kick in the score becomes audible
- **THEN** the kick trigger fires in the frame being presented at that moment (offset < 1 frame)

#### Scenario: Dense hats
- **WHEN** 16th-note hats occur at 174 BPM on a 60 Hz display
- **THEN** no hat trigger is lost

### Requirement: Live fallback
When the score does not cover the audible position, the engine SHALL derive energy, bands, and onset triggers from the live audio tap and SHALL crossfade to score-driven signals when coverage arrives.

#### Scenario: First second of an uncached track
- **WHEN** an uncached track starts
- **THEN** visuals react to live energy immediately and switch to beat-locked motion within about 1 s without a visible jump

