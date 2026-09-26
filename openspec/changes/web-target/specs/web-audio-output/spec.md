## ADDED Requirements

### Requirement: AudioWorklet sink
The web `AudioSink` SHALL output audio through an AudioWorklet that reads PCM from a SharedArrayBuffer ring without allocating or blocking.

#### Scenario: Playback
- **WHEN** an MP3 is played in Chrome
- **THEN** it plays at the correct pitch with no underruns under normal load

### Requirement: Resume on first gesture
The AudioContext SHALL be created at load and resumed on the first user gesture, then kept running with silence while idle.

#### Scenario: First click
- **WHEN** the user's first action is clicking play
- **THEN** the context resumes and audio starts within 50 ms

### Requirement: Web playback clock
The web clock SHALL compute the audible frame from worklet frame counters, `getOutputTimestamp()`, and `outputLatency`, exposed through the same `ClockReader` API as native.

#### Scenario: Visual sync in browser
- **WHEN** a click track plays and the visual engine flashes on beats
- **THEN** the flash is within one frame of the audible click (verified with the analysis debug strip and a recording)
