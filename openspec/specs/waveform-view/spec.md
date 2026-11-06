# waveform-view Specification

## Purpose
TBD - created by archiving change waveform-navigation. Update Purpose after archive.

## Requirements

### Requirement: Waveform section
The band (see `player-window` "Full-screen window") SHALL show the waveform as its right panel, to the right of the turntable, across the rest of the band's width (a third of it up to 1,080 skin pixels), 76 skin pixels tall, with a whole-track overview row and a zoomed row centered on the playhead, and no title bar. It SHALL always be shown: there SHALL be no key or button that hides it, and a settings file that hid it SHALL load with the waveform shown.

#### Scenario: Toggle
- **WHEN** the user presses W
- **THEN** nothing changes, and the waveform stays in the band

#### Scenario: Toggle with the button
- **WHEN** the mini player is drawn
- **THEN** it has no waveform button

#### Scenario: Beside the mini player
- **WHEN** the window is 1000 skin pixels wide
- **THEN** the overview row starts right of the turntable (at 666 pixels, after the two 333-pixel panels) and ends at the window's right edge, and clicking in it seeks

#### Scenario: Not squeezed by SYNC
- **WHEN** the window is 720 skin pixels wide and SYNC is turned on
- **THEN** the waveform stays 240 pixels wide and 76 tall

#### Scenario: Seek bar while hidden
- **WHEN** the app starts with a `settings.ron` holding `show_waveform: false`
- **THEN** the settings load, the waveform shows in the band, and there is no seek bar

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
