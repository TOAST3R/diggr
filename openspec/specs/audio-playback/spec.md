# audio-playback Specification

## Purpose
TBD - created by archiving change audio-core. Update Purpose after archive.
## Requirements
### Requirement: Supported formats
The system SHALL decode MP3, FLAC, WAV, OGG Vorbis, and AAC/M4A files using pure-Rust decoders, resampling to the output device's sample rate and channel layout when they differ.

#### Scenario: Play each format
- **WHEN** a valid file of each supported format is played
- **THEN** audio is output at the correct pitch and speed

#### Scenario: Sample-rate mismatch
- **WHEN** a 44.1 kHz file is played on a 48 kHz device
- **THEN** audio is resampled and plays at the correct pitch

### Requirement: Always-open output device
The system SHALL open the output device at application startup and keep the stream running, outputting silence while idle.

#### Scenario: Idle app
- **WHEN** the app has launched and nothing is playing
- **THEN** the output stream is running and emitting silence

### Requirement: Fast start
The system SHALL deliver the first non-silent sample of a track to the device within 30 ms of the play command on a warm app, for local files.

#### Scenario: Play latency measured
- **WHEN** play is issued for a local MP3 or FLAC file while the app is idle
- **THEN** the first non-silent sample is written to the device buffer within 30 ms (measured by the engine's internal instrumentation)

#### Scenario: Metadata does not block start
- **WHEN** a track has large embedded artwork or extensive tags
- **THEN** audio start is not delayed by reading them

### Requirement: Transport controls
The system SHALL support play, pause, resume, stop, seek (to an absolute time), next, previous, volume (0–100%), and balance (full left to full right).

#### Scenario: Seek
- **WHEN** the user seeks to 2:00 in a 4:00 track
- **THEN** audio resumes from 2:00 within 50 ms and no pre-seek audio is heard after the seek

#### Scenario: Pause and resume
- **WHEN** playback is paused and resumed
- **THEN** audio resumes from the exact frame where it paused

### Requirement: Gapless playback with pre-warm
The system SHALL pre-open and pre-decode the next queued track before the current one ends, and SHALL transition with no inserted silence.

#### Scenario: Gapless transition
- **WHEN** track N ends and track N+1 is next in the queue
- **THEN** the first frame of N+1 follows the last frame of N with no gap

#### Scenario: Pre-warm notification
- **WHEN** the current track has less than 30 s remaining
- **THEN** the next track is opened, its first second decoded, and a pre-warm event is emitted for other subsystems

### Requirement: Real-time safety
The audio callback SHALL NOT allocate memory, take locks, perform I/O, or wait on other threads.

#### Scenario: Allocation guard
- **WHEN** the engine runs its test suite in debug mode with the allocation guard enabled
- **THEN** no allocation occurs inside the audio callback

#### Scenario: Underrun
- **WHEN** the ring buffer is empty at callback time
- **THEN** the callback outputs silence, increments an underrun counter, and returns immediately

### Requirement: Error resilience
The system SHALL skip undecodable packets, skip tracks that cannot be decoded, and recover from output-device loss or change without user action.

#### Scenario: Corrupt frames
- **WHEN** a file contains corrupted frames in the middle
- **THEN** playback continues past them

#### Scenario: Unplayable file
- **WHEN** a file cannot be decoded at all
- **THEN** it is marked failed and playback advances to the next track

#### Scenario: Headphones unplugged
- **WHEN** the active output device disappears during playback
- **THEN** the stream is rebuilt on the new default device within 1 s and playback continues from the same position

### Requirement: Track metadata
The system SHALL expose artist, title, album, duration, bitrate, sample rate, and channel count for the current track, falling back to the file name when tags are absent.

#### Scenario: Missing tags
- **WHEN** a file has no artist or title tags
- **THEN** the title is derived from the file name and artist is empty

### Requirement: Gapless repeat modes
The engine SHALL support repeat modes Off, All and One. With All, playback SHALL continue from the last queue entry to the first; with One, the current track SHALL loop. Both SHALL be gapless and SHALL pre-warm the track that actually plays next.

#### Scenario: Repeat one
- **WHEN** repeat is One and the track ends
- **THEN** the same track starts again with no inserted silence

#### Scenario: Repeat all wraps
- **WHEN** repeat is All and the last queue entry ends
- **THEN** the first entry follows gaplessly, and `next` on the last entry also wraps to the first

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

