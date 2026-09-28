## MODIFIED Requirements

### Requirement: Transport and sliders
The main section SHALL provide previous, play, pause, stop, next, open, shuffle, and repeat buttons, a seek bar, a volume slider, and a waveform button, all wired to the Engine or the view. The waveform button SHALL sit where the balance slider was, SHALL show or hide the waveform section exactly as `W` does, and SHALL be drawn lit while the waveform section shows. The main section SHALL NOT offer a balance control, and playback SHALL always be centred.

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

### Requirement: Keyboard shortcuts
The system SHALL support Z (previous), X (play), C (pause), V (stop), B (next), ←/→ (seek ∓5 s), ↑/↓ (volume while the player has focus or in fullscreen; move the playlist cursor while the playlist has focus), Tab (switch focus between the player and the playlist), P (show the playing entry in the playlist), F (fullscreen visuals), H or F1 (shortcuts help), W (waveform section), S (spectrogram window), [ and ] (previous/next section), Shift+] (next energy rise), L (section loop), Shift+L (4/8/16-bar loop), Y (keep), N (pass), I (open the for-sale page), and Cmd+V, or Ctrl+V on Linux and Windows (paste a Discogs page).

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

## ADDED Requirements

### Requirement: Section focus
The player side (main, waveform and EQ sections) and the playlist SHALL each take keyboard focus when clicked, and Tab SHALL switch focus between them. The app SHALL start with the player focused. The focused side's title bars SHALL be drawn lit and the other side's dimmed, in the same frame as the click. Hovering SHALL NOT change focus. While a text field has keyboard focus, no shortcut SHALL act.

#### Scenario: Click the playlist
- **WHEN** the player has focus and the user clicks an entry in the playlist
- **THEN** the playlist's title bar is drawn lit, the main and EQ title bars dimmed, and ↑/↓ now move the playlist cursor

#### Scenario: Tab
- **WHEN** the playlist has focus and the user presses Tab
- **THEN** the player has focus and ↑ raises the volume

#### Scenario: Launch
- **WHEN** the app starts
- **THEN** the player has focus
