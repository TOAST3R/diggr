## ADDED Requirements

### Requirement: Waveform section
The player SHALL offer a waveform section under the main window, toggled with `W` and remembered across launches, showing a whole-track overview row and a zoomed row centered on the playhead.

#### Scenario: Toggle
- **WHEN** the user presses W
- **THEN** the waveform section appears under the main window, and pressing W again hides it

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
