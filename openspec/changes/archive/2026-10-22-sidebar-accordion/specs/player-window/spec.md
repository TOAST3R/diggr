## MODIFIED Requirements

### Requirement: Transport and sliders
The main section SHALL provide previous, play, pause, stop, next, shuffle, and repeat buttons, a seek bar, a volume slider, waveform, EQ, crates and playlist toggles, and a SYNC button, all wired to the Engine or the view. It SHALL NOT provide an open (Eject) button. The waveform, EQ, crates and playlist toggles SHALL be drawn as icons, not labels, and SHALL sit in one row with the SYNC button, in that order, and SYNC SHALL stay labelled SYNC. The crates toggle SHALL show or hide the CRATES panel (see `crates` "Crates panel"), and SHALL be drawn lit while it shows; the EQ toggle SHALL be drawn lit only while the EQ section shows. Shuffle and repeat SHALL be drawn as icons in the transport row; repeat one SHALL show the repeat icon with a small 1. Each icon SHALL be drawn lit while its setting is on, and each SHALL have a tooltip naming it. The waveform toggle SHALL sit right after the volume slider, SHALL show or hide the waveform section exactly as `W` does, and SHALL be drawn lit while the waveform section shows. The volume slider SHALL be 48 skin pixels wide and still cover 0–100%. The main section SHALL NOT offer a balance control, and playback SHALL always be centred.

#### Scenario: Drag seek bar
- **WHEN** the user drags the seek bar and releases at 75%
- **THEN** the Engine seeks to 75% of the track duration on release

#### Scenario: Volume change is immediate
- **WHEN** the volume slider is moved
- **THEN** the loudness change is audible within 20 ms

#### Scenario: Waveform button
- **WHEN** the waveform section is hidden and the user clicks the waveform button
- **THEN** the waveform section appears, the button is drawn lit, and the setting is remembered across launches

#### Scenario: Saved balance reset
- **WHEN** the app is launched with a settings file from an older version that holds a balance of -0.6
- **THEN** playback is centred

#### Scenario: Repeat icon cycles
- **WHEN** repeat is off and the user clicks the repeat icon twice
- **THEN** it is drawn lit after the first click (repeat all), and lit with a small 1 after the second (repeat one)

#### Scenario: No Eject
- **WHEN** the main section is drawn
- **THEN** it has no open (Eject) button, and ⌘O, the footer's ADD and drag-and-drop still add files to the shown crate

#### Scenario: SYNC beside the toggles
- **WHEN** the main section is drawn
- **THEN** the SYNC button, labelled SYNC, sits in the row of the waveform, EQ, crates and playlist icons, and clicking it turns SYNC on or off as before

#### Scenario: Crates toggle
- **WHEN** the main section is drawn
- **THEN** the crates icon sits between the EQ and playlist icons, its tooltip reads "Crates", and the volume slider ends before the waveform icon

#### Scenario: Volume range kept
- **WHEN** the user drags the 48 px volume slider from its left end to its right end
- **THEN** the volume goes from 0% to 100%
