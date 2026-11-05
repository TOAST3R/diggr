## MODIFIED Requirements

### Requirement: Transport and sliders
The mini player SHALL be the player: 275 skin pixels wide at the left of the band (see "Full-screen window"), in two rows. Its LCD row SHALL hold the play state, the time, the title line, and at its right end three icon buttons: EQ, which opens or closes the EQ window (see `eq-window`) and is drawn lit while it is open; waveform, which shows or hides the waveform exactly as `W` does and is drawn lit while it shows; and spectrum, which opens or closes the spectrogram window exactly as `S` does and is drawn lit while it is open. Its button row SHALL hold previous, play, pause, stop and next, a volume slider (48 skin pixels wide, covering 0–100%), shuffle and repeat, and SYNC, all wired to the Engine or the view, and none of them SHALL overlap. Shuffle and repeat SHALL be drawn as icons the size of the band's icon buttons, lit while their setting is on; repeat one SHALL show the repeat icon with a small 1. Each icon SHALL have a tooltip naming it, and SYNC SHALL stay labelled SYNC. There SHALL be no open (Eject) button, no balance control (playback is always centred), no playlist or crates toggle and no button that changes the layout. Seeking SHALL be done on the waveform, or, while it is hidden, on the seek bar the band shows in its place (see `waveform-view`).

#### Scenario: Volume change is immediate
- **WHEN** the volume slider is moved
- **THEN** the loudness change is audible within 20 ms

#### Scenario: Waveform button
- **WHEN** the waveform is hidden and the user clicks the mini player's waveform button
- **THEN** the waveform appears in the band, the button is drawn lit, and the setting is remembered across launches

#### Scenario: Shuffle and repeat fit
- **WHEN** the mini player is drawn at 275 skin pixels wide
- **THEN** previous, play, pause, stop, next, the volume slider, shuffle, repeat and SYNC all show in its button row without overlapping

#### Scenario: Repeat icon cycles
- **WHEN** repeat is off and the user clicks the mini player's repeat icon twice
- **THEN** it is drawn lit after the first click (repeat all), and lit with a small 1 after the second (repeat one)

#### Scenario: Shuffle
- **WHEN** the user clicks the mini player's shuffle icon
- **THEN** shuffle turns on and the icon is drawn lit; clicking again turns it off

#### Scenario: EQ button
- **WHEN** the user clicks the mini player's EQ button
- **THEN** the EQ window opens and the button is drawn lit; clicking it again closes the window

#### Scenario: Spectrum button
- **WHEN** a track plays and the user clicks the mini player's spectrum button
- **THEN** the spectrogram window opens as with S, and the button is drawn lit until it closes

#### Scenario: Saved balance reset
- **WHEN** the app is launched with a settings file from an older version that holds a balance of -0.6
- **THEN** playback is centred

#### Scenario: No Eject
- **WHEN** the mini player is drawn
- **THEN** it has no open (Eject) button, and ⌘O, the list menu's Add files… and drag-and-drop still add files to the shown crate

