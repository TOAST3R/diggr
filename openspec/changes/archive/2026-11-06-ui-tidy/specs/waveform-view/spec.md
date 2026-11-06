# Spec Delta

## MODIFIED Requirements

### Requirement: Waveform section
The band (see `player-window` "Full-screen window") SHALL show the waveform to the right of the mini player (and of the turntable while SYNC is on), across the rest of the band's width up to the window buttons, with a whole-track overview row and a zoomed row centered on the playhead, and no title bar. It SHALL always be shown: there SHALL be no key or button that hides it, and a settings file that hid it SHALL load with the waveform shown.

#### Scenario: Toggle
- **WHEN** the user presses W
- **THEN** nothing changes, and the waveform stays in the band

#### Scenario: Toggle with the button
- **WHEN** the mini player is drawn
- **THEN** it has no waveform button

#### Scenario: Beside the mini player
- **WHEN** the window is 1000 skin pixels wide, SYNC is off and the waveform is on
- **THEN** the overview row starts right of the 275-pixel mini player and ends before the window buttons, and clicking in it seeks

#### Scenario: Seek bar while hidden
- **WHEN** the app starts with a `settings.ron` holding `show_waveform: false`
- **THEN** the settings load, the waveform shows in the band, and there is no seek bar
