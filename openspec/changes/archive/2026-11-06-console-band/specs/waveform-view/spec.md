# Spec Delta

## MODIFIED Requirements

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
