# Spec Delta

## MODIFIED Requirements

### Requirement: Transport and sliders
The mini player SHALL be the player: the band's left panel, a third of its width and at least 227 skin pixels (see "Full-screen window"). Its LCD SHALL hold the play state, the time and the track lines (see "Track info display"), and at its right end, ending 4 skin pixels from the panel's edge: ▤, which groups or ungroups the crate shown in the playlist (see `record-view`) and is drawn lit while it is grouped; EQ, which opens or closes the EQ window (see `eq-window`) and is drawn lit while it is open; and spectrum, which opens or closes the spectrogram window exactly as `S` does and is drawn lit while it is open. Its button row SHALL hold previous, play, pause, stop and next, a volume slider (covering 0–100%, taking the width left between them and shuffle, at least 60 skin pixels), shuffle and repeat, all wired to the Engine or the view, on the transport buttons' centre line, with gaps of 2 to 4 skin pixels and none overlapping, ending where the LCD's icons end. Shuffle and repeat SHALL be drawn as icons the size of the band's icon buttons, lit while their setting is on; repeat one SHALL show the repeat icon with a small 1. Each icon SHALL have a tooltip naming it. Under the button row, in the last 8 skin pixels of the band, the mini player SHALL have a seek bar across its width, 4 skin pixels in from either edge: a groove with a thumb at the playing position, which SHALL follow playback. Clicking or dragging it SHALL seek, on release, to the point under the pointer, and while the pointer is on it a tooltip SHALL name the time at the pointer. It SHALL be inert, with no thumb, while nothing plays or the track's length isn't known. There SHALL be no SYNC button (SYNC is on the turntable, see `tempo-preview`), no waveform button (the waveform is always shown, see `waveform-view`), no open (Eject) button, no balance control (playback is always centred), no playlist or crates toggle and no button that changes the layout. Seeking SHALL be done on the waveform or the seek bar.

#### Scenario: Click to seek
- **WHEN** a 2 s track plays and the user clicks the seek bar at 75% of its width
- **THEN** playback continues from 1.5 s, and the thumb is at 75%

#### Scenario: Drag and release
- **WHEN** the user presses on the seek bar, drags to 25% and releases
- **THEN** the thumb follows the pointer while dragging, and the engine seeks to 25% of the track on release, not before

#### Scenario: Follows playback
- **WHEN** a track plays for 10 s and the user does not touch the bar
- **THEN** the thumb moves with the playing position

#### Scenario: Tooltip
- **WHEN** a 4:00 track plays and the pointer rests on the seek bar at 50%
- **THEN** a tooltip reads 2:00

#### Scenario: Nothing to seek
- **WHEN** nothing plays, and the user clicks the seek bar
- **THEN** no thumb is drawn and nothing seeks

#### Scenario: The bar fits
- **WHEN** the mini player is drawn at 227 and at 360 skin pixels wide
- **THEN** the seek bar spans its width less 4 pixels either side, below the transport buttons and inside the band, overlapping nothing

#### Scenario: Volume change is immediate
- **WHEN** the volume slider is moved
- **THEN** the loudness change is audible within 20 ms

#### Scenario: Waveform button
- **WHEN** the mini player is drawn
- **THEN** it has no waveform button, and the waveform shows in the band

#### Scenario: Shuffle and repeat fit
- **WHEN** the mini player is drawn at its narrowest, 227 skin pixels wide
- **THEN** previous, play, pause, stop, next, the volume slider (at least 60 pixels), shuffle and repeat show in its button row without overlapping, repeat ending 4 pixels from the right edge, as the spectrum button does in the LCD above it

#### Scenario: LCD row icons fit
- **WHEN** the mini player is drawn at 227 skin pixels wide
- **THEN** ▤, EQ and spectrum show at the LCD's right end without overlapping each other or the track line, which keeps room for 15 characters

#### Scenario: SYNC beside the spectrum button
- **WHEN** the mini player is drawn
- **THEN** nothing sits right of the spectrum button, and SYNC is on the turntable beside the player, where clicking it turns SYNC on

#### Scenario: Wider
- **WHEN** the window is 1,100 skin pixels wide
- **THEN** the mini player is 360 pixels wide: the track lines are longer and the volume slider wider, and the icons still end 4 pixels from its right edge

#### Scenario: Group button
- **WHEN** a flat crate is shown and the user clicks the mini player's ▤
- **THEN** the crate is grouped by record and ▤ is drawn lit; its tooltip reads "Group by record"

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
