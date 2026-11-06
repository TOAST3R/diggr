# Spec Delta

## MODIFIED Requirements

### Requirement: Track info display
The mini player's LCD SHALL show a scrolling "N. (catno) Artist: Title (T BPM) (m:ss)" line on its first row, between the time and its ▤, EQ and spectrum buttons, with no bitrate or sample-rate readout. The verdict flash (see "Verdict flash") SHALL take this line while it lasts. It SHALL NOT show mono or stereo indicators. The name SHALL follow the playlist's display format. Under it the LCD SHALL have a second line. For an entry from Discogs it SHALL read its label, side, year and a for-sale summary, "‹label› · A1 · 1994 · K for sale from ‹lowest price›" or "none for sale", leaving out what isn't known; for a local file it SHALL read the file's album tag, when it has one. The catalog number SHALL NOT be repeated on it, and a line longer than the display SHALL scroll like the first. The line SHALL be empty while nothing plays.

#### Scenario: Long title scrolls
- **WHEN** the title text exceeds the display width
- **THEN** it scrolls horizontally

#### Scenario: Discogs details
- **WHEN** entry 3, "Nightcraft" / "Glasshouse" (6:12) at 124 BPM, plays from side A1 of catalog number LT-012 (1994) on Lowtide Tapes, with 6 copies for sale from €9.00
- **THEN** the first line reads "3. (LT-012) Nightcraft: Glasshouse (124 BPM) (6:12)" and the second "Lowtide Tapes · A1 · 1994 · 6 for sale from €9.00", in the skin's capitals

#### Scenario: A local file
- **WHEN** a local file with the album tag "Silentintroduction" plays
- **THEN** the second line reads "Silentintroduction"; with no album tag it is empty

#### Scenario: No channel indicators
- **WHEN** a mono file plays, and then a stereo file
- **THEN** the mini player shows neither a MONO nor a STEREO indicator, nor a bitrate or sample rate

#### Scenario: Flash in the mini player
- **WHEN** a track plays and the user presses Y on a record not yet wanted
- **THEN** the mini player's first line shows "WANTED" centred for 1.5 s, then the track line again, and the second line is unchanged

### Requirement: Transport and sliders
The mini player SHALL be the player: the band's left panel, a third of its width and at least 227 skin pixels (see "Full-screen window"). Its LCD SHALL hold the play state, the time and the track lines (see "Track info display"), and at its right end, ending 4 skin pixels from the panel's edge: ▤, which groups or ungroups the crate shown in the playlist (see `record-view`) and is drawn lit while it is grouped; EQ, which opens or closes the EQ window (see `eq-window`) and is drawn lit while it is open; and spectrum, which opens or closes the spectrogram window exactly as `S` does and is drawn lit while it is open. Its button row SHALL hold previous, play, pause, stop and next, a volume slider (covering 0–100%, taking the width left between them and shuffle, at least 60 skin pixels), shuffle and repeat, all wired to the Engine or the view, on the transport buttons' centre line, with gaps of 2 to 4 skin pixels and none overlapping, ending where the LCD's icons end. Shuffle and repeat SHALL be drawn as icons the size of the band's icon buttons, lit while their setting is on; repeat one SHALL show the repeat icon with a small 1. Each icon SHALL have a tooltip naming it. There SHALL be no SYNC button (SYNC is on the turntable, see `tempo-preview`), no waveform button (the waveform is always shown, see `waveform-view`), no open (Eject) button, no balance control (playback is always centred), no playlist or crates toggle and no button that changes the layout. Seeking SHALL be done on the waveform.

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

### Requirement: Section focus
The player side (the band: mini player, turntable and waveform) and the playlist SHALL each take keyboard focus when clicked, and Tab SHALL switch focus between them. The app SHALL start with the player focused. The window header SHALL be drawn lit while the player has focus and dimmed while the playlist has it, and the playlist's title bar the other way round, in the same frame as the click. Hovering SHALL NOT change focus. While a text field has keyboard focus, no shortcut SHALL act.

#### Scenario: Click the playlist
- **WHEN** the player has focus and the user clicks an entry in the playlist
- **THEN** the playlist's title bar is drawn lit and the header dimmed, and ↑/↓ now move the playlist cursor

#### Scenario: Tab
- **WHEN** the playlist has focus and the user presses Tab
- **THEN** the player has focus and ↑ raises the volume

#### Scenario: Launch
- **WHEN** the app starts
- **THEN** the player has focus

### Requirement: Full-screen window
The app SHALL have one layout: a header across the top of the window (see "Window header"), a band under it, and the playlist under that. The band SHALL hold three panels, from the left: the mini player (see "Transport and sliders"), the turntable (see `tempo-preview`), always shown, and the waveform (see `waveform-view`), always shown. The mini player and the turntable SHALL each be a third of the window's width, but at least 227 and at most 360 skin pixels, and the waveform SHALL have the rest, so it is a third up to 1,080 skin pixels and wider above; below 681 it SHALL keep what is left, down to nothing. Thirds round down. The panels SHALL have no title bars of their own. The band SHALL always be 76 skin pixels tall, whether or not SYNC is on. The app SHALL open with its window maximized (the screen's usable area, below the menu bar), within the 300 ms launch target. When the operating system restores, resizes or maximizes the window, the same layout SHALL fill it. Dragging the band anywhere that is not a control (the turntable's background, the player's background) SHALL move the window when it isn't maximized. There SHALL be no other layout and no button or key that switches layouts. Resizing SHALL NOT affect playback.

#### Scenario: Launch
- **WHEN** the app is launched on a 1440 × 900 display
- **THEN** its first frame fills the area below the menu bar, the header is across the top, the mini player is at the left of the band under it, the turntable beside it and the waveform at the right, and the playlist fills the full width under them

#### Scenario: Thirds
- **WHEN** the window is 720 skin pixels wide
- **THEN** the mini player, the turntable and the waveform are 240 pixels wide each, and the band is 76 pixels tall under the 15-pixel header

#### Scenario: A wide window
- **WHEN** the window is 1,500 skin pixels wide
- **THEN** the mini player and the turntable are 360 pixels wide and the waveform is 780

#### Scenario: A narrow window
- **WHEN** the window is 600 skin pixels wide
- **THEN** the mini player and the turntable are 227 pixels wide and the waveform has the 146 pixels left

#### Scenario: Drag the band
- **WHEN** the user drags the turntable's background, with the window not maximized
- **THEN** the window moves

#### Scenario: Restored by the operating system
- **WHEN** the user restores the window with the operating system's button
- **THEN** the header, the band and the playlist fill the smaller window the same way, and playback goes on

#### Scenario: Close
- **WHEN** the user clicks the header's close button
- **THEN** the app quits, as the main window's close button did

#### Scenario: Between the gear and close
- **WHEN** the user clicks the gap between the header's minimize and close buttons
- **THEN** nothing opens and the app doesn't quit

#### Scenario: An older settings file
- **WHEN** the app starts with a `settings.ron` holding `playlist_maximized: false`, `show_playlist: false`, `show_crates: true`, a `vis` mode and a playlist width and rows
- **THEN** the settings load, and the app opens full screen with the band and the playlist

### Requirement: Options menu
Right-clicking (or Control-clicking on macOS) anywhere on the mini player that is not a control SHALL open the Options menu: where digging is available, Discogs…, Browser… and Set up Diggr…. The menu SHALL NOT offer the size (see "Size keys") or the spectrogram, which has the mini player's spectrum button and S. The controls SHALL keep their own clicks. Hovering that area SHALL show a tooltip saying that a right-click opens the options. The gear button in the window header (see "Window header") SHALL open the same menu, so it can be found without knowing about the right-click; where digging isn't available the menu is empty and the gear SHALL NOT be shown. Opening the menu SHALL NOT affect playback.

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
- **THEN** the header shows help, minimize and close only, each in its usual place

## ADDED Requirements

### Requirement: Window header
The window SHALL have a header across its top, 15 skin pixels tall, in the skin's title-bar style: the caption DIGGR centred between groove lines, and at its right end, from the left, help (?), the Options gear (see "Options menu"), minimize and close, all drawn at the same size, close quitting the app. Each SHALL take clicks only inside its own button, so a click between two of them does nothing. Help SHALL open the shortcuts help, as H and F1 do.

#### Scenario: Buttons
- **WHEN** the window is 1,100 skin pixels wide
- **THEN** the header spans the full width with DIGGR in its middle, and help, the gear, minimize and close at its right end

#### Scenario: Between two buttons
- **WHEN** the user clicks the gap between the gear and minimize
- **THEN** nothing opens and the app doesn't quit

#### Scenario: Help
- **WHEN** the user clicks the header's help button
- **THEN** the shortcuts help opens, as with H

### Requirement: Header drag and maximize
Dragging the window header SHALL move a window that isn't maximized, and double-clicking it SHALL maximize the window or, when it is maximized, restore it.

#### Scenario: Drag
- **WHEN** the user drags the header, with the window not maximized
- **THEN** the window moves

#### Scenario: Double-click
- **WHEN** the user double-clicks the header's caption
- **THEN** the window maximizes, and double-clicking again restores it
