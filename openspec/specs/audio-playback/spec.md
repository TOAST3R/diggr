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
The system SHALL support play, pause, resume, stop, seek (to an absolute time), next, previous, volume (0–100%), and balance (full left to full right). A seek, next or previous while audio plays SHALL fade the old audio out over 5 ms and the new audio in over 5 ms, so neither stops nor starts with a click; play from stopped and gapless playback SHALL be unchanged.

#### Scenario: Seek
- **WHEN** the user seeks to 2:00 in a 4:00 track
- **THEN** audio resumes from 2:00 within 50 ms; the pre-seek audio fades out over 5 ms and none of it is heard after that

#### Scenario: Pause and resume
- **WHEN** playback is paused and resumed
- **THEN** audio resumes from the exact frame where it paused

#### Scenario: No click at a seek
- **WHEN** a 1 kHz sine at half scale plays and the user seeks to 5 s
- **THEN** no two consecutive output samples differ by more than the sine itself does (0.08), and there is no underrun

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

### Requirement: Varispeed
The engine SHALL play any track at a speed from 0.84 to 1.16 set for it, by resampling (pitch follows speed). A change to the playing track's speed SHALL be heard within 50 ms, without a click, a gap or an underrun: the old audio keeps playing until the new is queued, then a 2 ms crossfade joins them. Each queued track SHALL carry its own speed from its first frame, so gapless playback holds across tracks at different speeds. The audio callback SHALL stay real-time safe. At speed 1 the output SHALL match the previous resampler's within 90 dB SNR. Start (< 30 ms), seek (< 50 ms) and zero underruns SHALL hold at any speed in the range.

#### Scenario: Change heard fast
- **WHEN** the playing track's speed changes from 1.00 to 1.04
- **THEN** the new speed is heard within 50 ms, with no click and no underrun

#### Scenario: Gapless at two speeds
- **WHEN** a track at speed 1.03 ends and the next plays at 0.97
- **THEN** the next starts with no gap and no click

#### Scenario: Benchmark
- **WHEN** `diggr --bench --speed 1.04` runs on the reference Mac
- **THEN** worst start < 30 ms, worst seek < 50 ms, worst speed change < 50 ms, and 0 underruns

### Requirement: Platter hold
The engine SHALL let the playing track be held and released. A hold SHALL slow it to a stop in 0.4 s, its pitch falling with its speed, after which the output SHALL be silent and no more of the queue SHALL be consumed. A release SHALL bring it back to its speed in 0.3 s from wherever it got to. A hold or a release SHALL start to be heard within 20 ms, with no click, and SHALL cause no underrun.

#### Scenario: Brake
- **WHEN** a track plays at speed 1 and the app holds it
- **THEN** the rate heard falls from 1 to 0 over 0.4 s (±10 ms), the output is silent afterwards, and the queue's position stops advancing

#### Scenario: Spin up
- **WHEN** a held track is released
- **THEN** it plays again from where it stopped, the rate rising from 0 to its speed over 0.3 s, with no gap and no underrun

#### Scenario: Release before it stops
- **WHEN** a track is held and released 0.2 s later
- **THEN** the rate rises again from about 0.5, without having reached 0

#### Scenario: Under SYNC
- **WHEN** a track plays fitted at speed 1.03 and is held, then released
- **THEN** it slows from 1.03 to 0 and comes back to 1.03

#### Scenario: Reaction
- **WHEN** the app holds a playing track
- **THEN** the first slowed audio reaches the device within 20 ms

### Requirement: A hold ends by itself
A hold SHALL end, as a release would, when the app hasn't confirmed it for 0.5 s. A seek, stop, pause, play, next, previous or speed change SHALL end it at once, and the audio that follows SHALL play at its own speed from its first frame.

#### Scenario: Watchdog
- **WHEN** the app holds a track and then stops confirming the hold
- **THEN** the track spins back up 0.5 s after the last confirmation

#### Scenario: Seek ends it
- **WHEN** a held track is sought to 1:00
- **THEN** playback continues from 1:00 at its speed, not held

### Requirement: The stage at rest
While no hold is active or ending, the platter stage SHALL pass the audio through unchanged: the output SHALL be identical, sample for sample, to playback without the stage. The stage SHALL allocate nothing, take no lock and make no system call in the audio callback, and the start, seek and underrun targets SHALL still hold.

#### Scenario: Bit-exact at rest
- **WHEN** a track plays through without a hold
- **THEN** its output equals the output without the platter stage, sample for sample

#### Scenario: Real-time safe
- **WHEN** a hold and a release run in the allocation-counting test
- **THEN** the audio callback allocates nothing
