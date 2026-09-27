# player-window Specification

## Purpose
TBD - created by archiving change classic-ui. Update Purpose after archive.
## Requirements
### Requirement: Time display
The main section SHALL show the current time in an LCD-style display derived from the playback clock, toggling between elapsed and remaining on click.

#### Scenario: Toggle remaining
- **WHEN** the user clicks the time display
- **THEN** it shows remaining time prefixed with a minus sign

### Requirement: Track info display
The main section SHALL show a scrolling "N. Artist - Title (m:ss)" line, bitrate (kbps), sample rate (kHz), and mono/stereo indicators.

#### Scenario: Long title scrolls
- **WHEN** the title text exceeds the display width
- **THEN** it scrolls horizontally

### Requirement: Transport and sliders
The main section SHALL provide previous, play, pause, stop, next, open, shuffle, and repeat buttons, a seek bar, a volume slider, and a balance slider, all wired to the Engine.

#### Scenario: Drag seek bar
- **WHEN** the user drags the seek bar and releases at 75%
- **THEN** the Engine seeks to 75% of the track duration on release

#### Scenario: Volume change is immediate
- **WHEN** the volume slider is moved
- **THEN** the loudness change is audible within 20 ms

### Requirement: Mini visualizer
The main section SHALL show a 19-bar spectrum analyzer with peak caps (or an oscilloscope, toggled by click), aligned to the audible position from the playback clock.

#### Scenario: Spectrum tracks audio
- **WHEN** a kick drum is heard
- **THEN** the low bars rise in the same displayed frame (within one UI refresh)

### Requirement: Keyboard shortcuts
The system SHALL support Z (previous), X (play), C (pause), V (stop), B (next), ←/→ (seek ∓5 s), ↑/↓ (volume), F (fullscreen visuals), H or F1 (shortcuts help), W (waveform section), S (spectrogram window), [ and ] (previous/next section), Shift+] (next energy rise), L (section loop), and Shift+L (4/8/16-bar loop).

#### Scenario: Classic keys
- **WHEN** the user presses B during playback
- **THEN** the next track starts

#### Scenario: Structure keys
- **WHEN** the user presses ] during playback of an analyzed track
- **THEN** playback moves to the next section on the next downbeat

#### Scenario: Spectrogram key
- **WHEN** the user presses S in the player window
- **THEN** the spectrogram window opens

### Requirement: Low idle cost
The UI SHALL consume near-zero CPU when idle and SHALL NOT repaint while minimized or occluded.

#### Scenario: Idle stopped
- **WHEN** nothing is playing and there is no input for 10 s
- **THEN** the UI performs no repaints

### Requirement: Fast launch
The app SHALL show an interactive window within 300 ms of launch on macOS.

#### Scenario: Cold launch
- **WHEN** the app is launched with a persisted 500-entry playlist
- **THEN** the window is interactive within 300 ms and durations fill in afterwards

### Requirement: Shortcuts help
`H` or `F1` SHALL open a panel listing every keyboard and mouse shortcut, grouped by area, in the player window and in fullscreen; `H`, `F1`, `Esc` or the panel's close button SHALL close it, and `Esc` SHALL close the panel before it leaves fullscreen.

#### Scenario: Open help
- **WHEN** the user presses H in the player window
- **THEN** a panel lists the playback, structure, player window, fullscreen visuals and mouse shortcuts

#### Scenario: Esc in fullscreen
- **WHEN** the help panel is open in fullscreen and the user presses Esc
- **THEN** the panel closes and fullscreen stays on

