# Spec Delta

## MODIFIED Requirements

### Requirement: Transport and sliders
The mini player SHALL be the player: 275 skin pixels wide at the left of the band (see "Full-screen window"), in two rows. Its LCD row SHALL hold the play state, the time, the title line, and at its right end, ending 4 skin pixels from the edge: ▤, which groups or ungroups the crate shown in the playlist (see `record-view`) and is drawn lit while it is grouped; EQ, which opens or closes the EQ window (see `eq-window`) and is drawn lit while it is open; spectrum, which opens or closes the spectrogram window exactly as `S` does and is drawn lit while it is open; and SYNC. Its button row SHALL hold previous, play, pause, stop and next, a volume slider (108 skin pixels wide, covering 0–100%), shuffle and repeat, all wired to the Engine or the view, on the transport buttons' centre line, with gaps of 2 to 4 skin pixels and none overlapping, ending where the LCD row ends. Shuffle and repeat SHALL be drawn as icons the size of the band's icon buttons, lit while their setting is on; repeat one SHALL show the repeat icon with a small 1. Each icon SHALL have a tooltip naming it, and SYNC SHALL stay labelled SYNC. There SHALL be no waveform button (the waveform is always shown, see `waveform-view`), no open (Eject) button, no balance control (playback is always centred), no playlist or crates toggle and no button that changes the layout. Seeking SHALL be done on the waveform.

#### Scenario: Volume change is immediate
- **WHEN** the volume slider is moved
- **THEN** the loudness change is audible within 20 ms

#### Scenario: Waveform button
- **WHEN** the mini player is drawn
- **THEN** it has no waveform button, and the waveform shows in the band

#### Scenario: Shuffle and repeat fit
- **WHEN** the mini player is drawn at 275 skin pixels wide
- **THEN** previous, play, pause, stop, next, the volume slider, shuffle and repeat show in its button row without overlapping, repeat ending 4 pixels from the right edge, as SYNC does in the LCD row above it

#### Scenario: LCD row icons fit
- **WHEN** the mini player is drawn at 275 skin pixels wide
- **THEN** ▤, EQ, spectrum and SYNC show at the LCD row's right end without overlapping each other or the title line, which keeps room for 17 characters

#### Scenario: SYNC beside the spectrum button
- **WHEN** the user clicks SYNC, right of the spectrum button in the LCD row
- **THEN** SYNC turns on, as it did from the button row

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

### Requirement: Options menu
Right-clicking (or Control-clicking on macOS) anywhere on the mini player that is not a control SHALL open the Options menu: where digging is available, Discogs…, Browser… and Set up Diggr…. The menu SHALL NOT offer the size (see "Size keys") or the spectrogram, which has the mini player's spectrum button and S. The controls SHALL keep their own clicks. Hovering that area SHALL show a tooltip saying that a right-click opens the options. The gear button in the band's top-right corner (see "Full-screen window") SHALL open the same menu, so it can be found without knowing about the right-click; where digging isn't available the menu is empty and the gear SHALL NOT be shown. Opening the menu SHALL NOT affect playback.

#### Scenario: Open the options
- **WHEN** the user right-clicks the mini player's title line
- **THEN** the Options menu opens with Discogs…, Browser… and Set up Diggr…, and without Double size, Classic size or Spectrogram

#### Scenario: Controls keep their clicks
- **WHEN** the user right-clicks the volume slider
- **THEN** the Options menu doesn't open

#### Scenario: Options from the gear
- **WHEN** the user clicks the gear between minimize and close and chooses Discogs…
- **THEN** the Discogs dialog opens, as from the right-click Options menu

#### Scenario: No digging, no gear
- **WHEN** the app runs where digging isn't available
- **THEN** the band's corner shows minimize and close only, each in its usual place

### Requirement: Full-screen window
The app SHALL have one layout: a band across the top of the window and the playlist under it. The band SHALL hold, from the left, the mini player (see "Transport and sliders"), the turntable while SYNC is on (see `tempo-preview`), and the waveform (see `waveform-view`), always shown, across the rest of the width; at the band's top-right corner SHALL sit, from the left, minimize, the Options gear (see "Options menu") and close, all drawn at the same size, close quitting the app. Each SHALL take clicks only inside its own button, so a click between two of them does nothing. The band SHALL be as tall as the waveform section (58 skin pixels), or the turntable (104) while SYNC is on. The app SHALL open with its window maximized (the screen's usable area, below the menu bar), within the 300 ms launch target. When the operating system restores, resizes or maximizes the window, the same layout SHALL fill it. Dragging the band's top-right corner outside its buttons SHALL move the window when it isn't maximized. There SHALL be no other layout and no button or key that switches layouts. Resizing SHALL NOT affect playback.

#### Scenario: Launch
- **WHEN** the app is launched on a 1440 × 900 display
- **THEN** its first frame fills the area below the menu bar, the mini player is at the top left, the waveform at the top right, minimize, the gear and close at the top-right corner, and the playlist fills the full width under them

#### Scenario: Restored by the operating system
- **WHEN** the user restores the window with the operating system's button
- **THEN** the band and the playlist fill the smaller window the same way, and playback goes on

#### Scenario: Close
- **WHEN** the user clicks the band's close button
- **THEN** the app quits, as the main window's close button did

#### Scenario: Between the gear and close
- **WHEN** the user clicks the gap between the gear and close
- **THEN** nothing opens and the app doesn't quit

#### Scenario: An older settings file
- **WHEN** the app starts with a `settings.ron` holding `playlist_maximized: false`, `show_playlist: false`, `show_crates: true`, a `vis` mode and a playlist width and rows
- **THEN** the settings load, and the app opens full screen with the band and the playlist

### Requirement: Keyboard shortcuts
The system SHALL support Space (play or pause: from stopped or paused it plays, while playing it pauses), Shift+Space (stop), Z (previous), X (play), C (pause), V (stop), B (next), ←/→ (seek ∓5 s; with the playlist focused in a grouped crate, close or open the record under the cursor instead, see `record-view`), ↑/↓ (volume while the player has focus; move the playlist cursor while the playlist has focus), Tab (switch focus between the player and the playlist), P (show the playing entry in the playlist), H or F1 (shortcuts help), S (spectrogram window), [ and ] (previous/next section), Shift+] (next energy rise), L (section loop), Shift+L (4/8/16-bar loop), Y (add to the wantlist, or remove from it), N (pass), I (open the for-sale page), and Cmd+V, or Ctrl+V on Linux and Windows (paste a Discogs page). Space and Shift+Space SHALL work whichever of the player and the playlist has focus, and SHALL NOT activate a focused button.

#### Scenario: Space pauses and resumes
- **WHEN** a track is playing and the user presses Space, then Space again
- **THEN** playback pauses at its position, then resumes from it

#### Scenario: Space from stopped
- **WHEN** playback is stopped with a track loaded and the user presses Space
- **THEN** that track plays from its start

#### Scenario: Space with the playlist focused
- **WHEN** the playlist has focus, the cursor is on a closed record row, and a track is playing, and the user presses Space
- **THEN** playback pauses and the record stays closed

#### Scenario: Shift+Space stops
- **WHEN** a track is playing and the user presses Shift+Space
- **THEN** playback stops, as V does

#### Scenario: Classic keys
- **WHEN** the user presses B during playback
- **THEN** the next track starts

#### Scenario: Arrow keys with the player focused
- **WHEN** the player has focus and the user presses ↑
- **THEN** the volume rises by 5%, and the playlist cursor doesn't move

#### Scenario: Arrow keys with the playlist focused
- **WHEN** the playlist has focus and the user presses ↓
- **THEN** the playlist cursor moves down one entry, and the volume doesn't change

#### Scenario: Structure keys
- **WHEN** the user presses ] during playback of an analyzed track
- **THEN** playback moves to the next section on the next downbeat

#### Scenario: Spectrogram key
- **WHEN** the user presses S in the player window
- **THEN** the spectrogram window opens

#### Scenario: Dig keys
- **WHEN** the user presses N while a track from Discogs plays
- **THEN** the track is marked passed and the next track starts

#### Scenario: Paste a page
- **WHEN** the user presses Cmd+V with a Discogs label address on the clipboard
- **THEN** the label's tracks are added to the shown crate

#### Scenario: No fullscreen key
- **WHEN** the user presses F in the player window
- **THEN** nothing happens, and the help (H) does not list F

#### Scenario: No group key
- **WHEN** the user presses Shift+G with a flat crate shown
- **THEN** nothing happens, and the help (H) does not list Shift+G

#### Scenario: No layout key
- **WHEN** the user presses Shift+P
- **THEN** nothing changes: the layout, the window and playback stay as they are

#### Scenario: No W
- **WHEN** the user presses W
- **THEN** nothing changes: the waveform stays in the band
