# show-render Specification

## Purpose
TBD - created by archiving change show-render. Update Purpose after archive.
## Requirements
### Requirement: Render a track's show to video
`winamp-native --render-show <track> -o <file.mp4>` SHALL render the track's visual show and the track's audio into an H.264/AAC MP4, with options for size (default 1920×1080; the width divisible by 8 and the height even), frame rate (default 60), a time range, the overlay card, and a pinned look.

#### Scenario: Default render
- **WHEN** the user runs `winamp-native --render-show track.flac -o show.mp4`
- **THEN** show.mp4 is 1920×1080 at 60 fps, as long as the track, with the track's audio

#### Scenario: Range and size
- **WHEN** the user adds `--from 1:00 --to 1:30 --size 1280x720 --fps 30`
- **THEN** the file is a 30-second 1280×720 30 fps video of that part of the show and audio

### Requirement: Frame-exact sync
Video frame n SHALL show the musical state at exactly (start + n/fps) seconds of the track, with no display-delay compensation, so that beat-driven events are on the frame of their audio.

#### Scenario: Kick on the frame
- **WHEN** a rendered track has a kick at 12.500 s and is rendered at 60 fps
- **THEN** the kick-driven visual response starts on frame 750

### Requirement: Same show as live
A render SHALL produce the same looks, transitions and parameter values at the same musical times as the hands-off live show of the same track replayed with a cached analysis and the same looks and director rules.

#### Scenario: Deterministic
- **WHEN** the same track is rendered twice with the same looks and rules
- **THEN** the two videos' frames are identical before encoding

### Requirement: Fast and reported
Rendering SHALL run faster than real time at 1080p60 on an M2 Mac, SHALL show progress with an estimate of the time left, and SHALL remove the partial file if cancelled or failed.

#### Scenario: Speed
- **WHEN** a 5-minute track is rendered at 1080p60 on an M2 Mac
- **THEN** the render finishes in under 5 minutes

#### Scenario: Cancel
- **WHEN** the user cancels a render
- **THEN** rendering stops and no partial output file remains

### Requirement: Clear dependency on ffmpeg
Before rendering, the system SHALL check for ffmpeg and, if missing, fail with a message explaining how to install it.

#### Scenario: No ffmpeg
- **WHEN** ffmpeg is not on the PATH
- **THEN** the command fails immediately with "ffmpeg not found — install it with `brew install ffmpeg`"

