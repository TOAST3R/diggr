## MODIFIED Requirements

### Requirement: Keyboard shortcuts
The system SHALL support Space (play or pause: from stopped or paused it plays, while playing it pauses), Shift+Space (stop), Z (previous), X (play), C (pause), V (stop), B (next), ←/→ (seek ∓5 s; with the playlist focused in a grouped crate, close or open the record under the cursor instead, see `record-view`), ↑/↓ (volume while the player has focus; move the playlist cursor while the playlist has focus), Tab (switch focus between the player and the playlist), P (show the playing entry in the playlist), Shift+P (maximize the playlist, or restore it), H or F1 (shortcuts help), W (waveform section), S (spectrogram window), [ and ] (previous/next section), Shift+] (next energy rise), L (section loop), Shift+L (4/8/16-bar loop), Y (add to the wantlist, or remove from it), N (pass), I (open the for-sale page), and Cmd+V, or Ctrl+V on Linux and Windows (paste a Discogs page). Space and Shift+Space SHALL work whichever of the player and the playlist has focus, and SHALL NOT activate a focused button.

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

#### Scenario: Maximize key
- **WHEN** the user presses Shift+P, and then Shift+P again
- **THEN** the playlist is maximized, and then restored

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

## ADDED Requirements

### Requirement: Transport shows the play state
The play, pause and stop buttons SHALL be drawn lit, with their `_on` sprites, while playback is respectively playing, paused or stopped; exactly one of them SHALL be lit at any time. The mini player of the maximized playlist SHALL light its play, pause and stop buttons the same way. The lit button SHALL change within one frame of the state changing, whatever changed it (a button, a key, the end of the crate).

#### Scenario: Playing
- **WHEN** a track starts playing
- **THEN** the play button is lit and the pause and stop buttons are not

#### Scenario: Paused by key
- **WHEN** a track is playing and the user presses Space
- **THEN** within one frame the pause button is lit and the play button is not

#### Scenario: Mini player
- **WHEN** the playlist is maximized and a track is playing
- **THEN** the mini player's play button is lit, and its pause and stop are not

#### Scenario: End of the crate
- **WHEN** the last track of the crate ends with repeat off
- **THEN** the stop button is lit

### Requirement: Shortcuts help leads with Space
The help panel's Playback group SHALL list "Space: play / pause" and "Shift+Space: stop" first, and SHALL list Z X C V B on one line as the classic keys. The player-window group SHALL list → / ← for opening and closing records, and SHALL NOT list Space for records.

#### Scenario: Help
- **WHEN** the user opens the help panel
- **THEN** the first Playback line is "Space: play / pause", and no line says Space opens a record
