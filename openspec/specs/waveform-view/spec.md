# waveform-view Specification

## Purpose
TBD - created by archiving change waveform-navigation. Update Purpose after archive.
## Requirements
### Requirement: Waveform section
The band (see `player-window` "Full-screen window") SHALL show the waveform to the right of the mini player (and of the turntable while SYNC is on), across the rest of the band's width up to the window buttons, with a whole-track overview row and a zoomed row centered on the playhead, and no title bar. It SHALL be toggled with `W` or the mini player's waveform button and remembered across launches. While it is hidden, the band SHALL show a thin seek bar in its place, which seeks on release like the waveform's overview row; the band's height and the playlist's rows SHALL NOT change.

#### Scenario: Toggle
- **WHEN** the user presses W
- **THEN** the waveform hides and a seek bar takes its place in the band, and pressing W again brings the waveform back

#### Scenario: Toggle with the button
- **WHEN** the user clicks the mini player's waveform button
- **THEN** the waveform hides or shows, exactly as with W

#### Scenario: Beside the mini player
- **WHEN** the window is 1000 skin pixels wide, SYNC is off and the waveform is on
- **THEN** the overview row starts right of the 275-pixel mini player and ends before the window buttons, and clicking in it seeks

#### Scenario: Seek bar while hidden
- **WHEN** the waveform is hidden and the user drags the band's seek bar and releases at 75%
- **THEN** the Engine seeks to 75% of the track, and the playlist keeps the same rows

### Requirement: Frequency colouring
The waveform SHALL colour each column by its low/mid/high energy, with bass towards red, mids towards green, and highs towards blue.

#### Scenario: Kick versus hats
- **WHEN** the zoomed row shows a kick followed by an isolated hi-hat
- **THEN** the kick columns are predominantly red and the hi-hat columns predominantly blue

### Requirement: Musical markers
The waveform SHALL show section bands and drop (energy-rise) markers on the overview row, and beat ticks with emphasized downbeats on the zoomed row, with provisional sections drawn faint.

#### Scenario: Grid on the zoomed row
- **WHEN** an analyzed 128 BPM track plays with the default zoom
- **THEN** 8 bars are visible, and every 4th beat tick is emphasized as a downbeat

### Requirement: Seek and zoom
Clicking or dragging on the overview row SHALL seek, and the scroll wheel over the zoomed row SHALL zoom between 1 and 64 bars.

#### Scenario: Click to seek
- **WHEN** the user clicks at 75% of the overview row's width
- **THEN** playback continues from 75% of the track

### Requirement: Low cost
The waveform section SHALL NOT repaint while playback is paused or stopped without input, and it SHALL draw without allocations per frame beyond its mesh.

#### Scenario: Paused
- **WHEN** playback is paused with the waveform section visible and there is no input
- **THEN** the window does not repaint

