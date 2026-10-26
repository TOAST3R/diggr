## ADDED Requirements

### Requirement: Full-screen window
The app SHALL have one layout: a band across the top of the window and the playlist under it. The band SHALL hold, from the left, the mini player (see "Transport and sliders"), the turntable while SYNC is on (see `tempo-preview`), and the waveform (see `waveform-view`) or, while it is hidden, a seek bar, across the rest of the width; minimize and close buttons SHALL sit at the band's top-right corner, close quitting the app. The band SHALL be as tall as the waveform section (58 skin pixels), or the turntable (104) while SYNC is on. The app SHALL open with its window maximized (the screen's usable area, below the menu bar), within the 300 ms launch target. When the operating system restores, resizes or maximizes the window, the same layout SHALL fill it. Dragging the band's top-right corner outside its buttons SHALL move the window when it isn't maximized. There SHALL be no other layout and no button or key that switches layouts. Resizing SHALL NOT affect playback.

#### Scenario: Launch
- **WHEN** the app is launched on a 1440 × 900 display
- **THEN** its first frame fills the area below the menu bar, the mini player is at the top left, the waveform at the top right, minimize and close at the top-right corner, and the playlist fills the full width under them

#### Scenario: Restored by the operating system
- **WHEN** the user restores the window with the operating system's button
- **THEN** the band and the playlist fill the smaller window the same way, and playback goes on

#### Scenario: Close
- **WHEN** the user clicks the band's close button
- **THEN** the app quits, as the main window's close button did

#### Scenario: An older settings file
- **WHEN** the app starts with a `settings.ron` holding `playlist_maximized: false`, `show_playlist: false`, `show_crates: true`, a `vis` mode and a playlist width and rows
- **THEN** the settings load, and the app opens full screen with the band and the playlist

### Requirement: Messages
Wherever a requirement says the main window says, shows or names something (a message, a "Waiting for …" line, a progress line), it SHALL be shown where messages are shown: over the top left of the window, on the mini player, for the time that requirement gives. No message SHALL need the classic main window.

#### Scenario: A send from the browser
- **WHEN** a label is sent from the browser while the window is full screen
- **THEN** its message shows over the top left of the window, on the mini player

## MODIFIED Requirements

### Requirement: Time display
The mini player SHALL show the current time in an LCD-style display derived from the playback clock, toggling between elapsed and remaining on click.

#### Scenario: Toggle remaining
- **WHEN** the user clicks the time display
- **THEN** it shows remaining time prefixed with a minus sign

### Requirement: Track info display
The mini player SHALL show a scrolling "N. (catno) Artist: Title (T BPM) (m:ss)" line on its LCD row, between the time and its EQ, waveform and spectrum buttons, with no bitrate or sample-rate readout. The verdict flash (see "Verdict flash") SHALL take this line while it lasts. It SHALL NOT show mono or stereo indicators. The name SHALL follow the playlist's display format. For an entry from Discogs, the line SHALL continue with its side, year and a for-sale summary: "K for sale from ‹lowest price›", "none for sale", or nothing when the numbers aren't known. The catalog number SHALL NOT be repeated in that continuation.

#### Scenario: Long title scrolls
- **WHEN** the title text exceeds the display width
- **THEN** it scrolls horizontally

#### Scenario: Discogs details
- **WHEN** entry 3, "Nightcraft" / "Glasshouse" (6:12) at 124 BPM, plays from side A1 of catalog number LT-012 (1994), with 6 copies for sale from €9.00
- **THEN** the line reads "3. (LT-012) Nightcraft: Glasshouse (124 BPM) (6:12) · A1 · 1994 · 6 for sale from €9.00", in the skin's capitals

#### Scenario: No channel indicators
- **WHEN** a mono file plays, and then a stereo file
- **THEN** the mini player shows neither a MONO nor a STEREO indicator, nor a bitrate or sample rate

#### Scenario: Flash in the mini player
- **WHEN** a track plays and the user presses Y on a record not yet wanted
- **THEN** the mini player's title line shows "WANTED" centred for 1.5 s, then the track line again

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
- **THEN** it has no open (Eject) button, and ⌘O, the footer's ADD and drag-and-drop still add files to the shown crate

### Requirement: Keyboard shortcuts
The system SHALL support Space (play or pause: from stopped or paused it plays, while playing it pauses), Shift+Space (stop), Z (previous), X (play), C (pause), V (stop), B (next), ←/→ (seek ∓5 s; with the playlist focused in a grouped crate, close or open the record under the cursor instead, see `record-view`), ↑/↓ (volume while the player has focus; move the playlist cursor while the playlist has focus), Tab (switch focus between the player and the playlist), P (show the playing entry in the playlist), H or F1 (shortcuts help), W (waveform section), S (spectrogram window), [ and ] (previous/next section), Shift+] (next energy rise), L (section loop), Shift+L (4/8/16-bar loop), Y (add to the wantlist, or remove from it), N (pass), I (open the for-sale page), and Cmd+V, or Ctrl+V on Linux and Windows (paste a Discogs page). Space and Shift+Space SHALL work whichever of the player and the playlist has focus, and SHALL NOT activate a focused button.

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

### Requirement: Section focus
The player side (the band: mini player, turntable and waveform) and the playlist SHALL each take keyboard focus when clicked, and Tab SHALL switch focus between them. The app SHALL start with the player focused. The focused side's title bars SHALL be drawn lit and the other side's dimmed, in the same frame as the click. Hovering SHALL NOT change focus. While a text field has keyboard focus, no shortcut SHALL act.

#### Scenario: Click the playlist
- **WHEN** the player has focus and the user clicks an entry in the playlist
- **THEN** the playlist's title bar is drawn lit, and ↑/↓ now move the playlist cursor

#### Scenario: Tab
- **WHEN** the playlist has focus and the user presses Tab
- **THEN** the player has focus and ↑ raises the volume

#### Scenario: Launch
- **WHEN** the app starts
- **THEN** the player has focus

### Requirement: Options menu
Right-clicking (or Control-clicking on macOS) anywhere on the mini player that is not a control SHALL open the Options menu: Double size (or Classic size), Spectrogram, and, where digging is available, Discogs… and Browser…. The controls SHALL keep their own clicks. Hovering that area SHALL show a tooltip saying that a right-click opens the options. The playlist footer's gear button SHALL open the same menu, so it can be found without knowing about the right-click. Opening the menu SHALL NOT affect playback.

#### Scenario: Open the options
- **WHEN** the user right-clicks the mini player's title line
- **THEN** the Options menu opens with Double size, Spectrogram, Discogs… and Browser…

#### Scenario: Controls keep their clicks
- **WHEN** the user right-clicks the volume slider
- **THEN** the Options menu doesn't open


### Requirement: Transport shows the play state
The mini player's play, pause and stop buttons SHALL be drawn lit, with their `_on` sprites, while playback is respectively playing, paused or stopped; exactly one of them SHALL be lit at any time. The lit button SHALL change within one frame of the state changing, whatever changed it (a button, a key, the end of the crate).

#### Scenario: Playing
- **WHEN** a track starts playing
- **THEN** the play button is lit and the pause and stop buttons are not

#### Scenario: Paused by key
- **WHEN** a track is playing and the user presses Space
- **THEN** within one frame the pause button is lit and the play button is not

#### Scenario: End of the crate
- **WHEN** the last track of the crate ends with repeat off
- **THEN** the stop button is lit

## REMOVED Requirements

### Requirement: Mini visualizer
**Reason**: The 19-bar spectrum and oscilloscope lived in the classic main window, which is removed; the spectrogram window (the mini player's spectrum button, or S) shows the spectrum.
**Migration**: Click the mini player's spectrum button, or press S. An old `vis` setting is ignored.
