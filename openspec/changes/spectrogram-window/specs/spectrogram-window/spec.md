## ADDED Requirements

### Requirement: Open the spectrogram
Pressing `S` in the player window, or choosing Spectrogram in the playlist's OPT menu, SHALL open a separate, resizable spectrogram window for the current track; pressing it again or closing the window SHALL close it. Its size and mode SHALL persist, and the shortcuts help SHALL list `S`.

#### Scenario: Open and close
- **WHEN** the user presses S while a track plays
- **THEN** a spectrogram window for that track opens, and pressing S again closes it

### Requirement: Track, detail and live modes
The window SHALL offer a whole-track mode with the playhead, a detail mode for zoomed ranges computed at full resolution, and a live mode showing a scrolling waterfall of the audible sound.

#### Scenario: Instant track view
- **WHEN** the window opens on a track whose overview is cached
- **THEN** the whole-track spectrogram is shown in the first frame

#### Scenario: Live in sync
- **WHEN** live mode shows a track with a kick on every beat
- **THEN** each kick's column appears when the kick is audible, within one frame

### Requirement: Axes, readouts and controls
The window SHALL show time and log-frequency axes, a cursor readout of time, frequency (with note name) and level in dB, a channel selector (mid and side in track mode; mid, left, right and side in detail and live modes) and an adjustable dB range.

#### Scenario: Readout
- **WHEN** the cursor rests on a 440 Hz tone
- **THEN** the readout shows about 440 Hz and "A4" and the tone's level in dB

### Requirement: Seek and zoom
Clicking SHALL seek playback to the clicked time, the same way as clicking the waveform's overview row (an active loop the target lies outside of is cleared); scrolling SHALL zoom time around the cursor, and Shift+scroll SHALL zoom frequency; dragging SHALL pan.

#### Scenario: Seek from the spectrogram
- **WHEN** the user clicks at 1:30 on the time axis
- **THEN** playback continues from 1:30

### Requirement: Quality verdict
The window SHALL show the detected content cutoff and, when flagged, a "likely from a lossy source" verdict with an approximate original bitrate.

#### Scenario: Flagged file
- **WHEN** the window shows a FLAC file flagged with a 16 kHz cutoff
- **THEN** it reads "Content ends at 16.0 kHz: likely from a lossy source (≈128 kbps MP3)"

### Requirement: Low cost
The spectrogram window SHALL repaint only while live mode is playing, while detail is arriving, or on input, and SHALL NOT repaint while fullscreen visuals are active.

#### Scenario: Track mode, paused
- **WHEN** the window is in track mode and playback is paused with no input
- **THEN** the window does not repaint
