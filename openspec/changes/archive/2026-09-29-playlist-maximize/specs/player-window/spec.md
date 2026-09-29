## MODIFIED Requirements

### Requirement: Keyboard shortcuts
The system SHALL support Z (previous), X (play), C (pause), V (stop), B (next), ←/→ (seek ∓5 s), ↑/↓ (volume while the player has focus or in fullscreen; move the playlist cursor while the playlist has focus), Tab (switch focus between the player and the playlist), P (show the playing entry in the playlist), Shift+P (maximize the playlist, or restore it), F (fullscreen visuals), H or F1 (shortcuts help), W (waveform section), S (spectrogram window), [ and ] (previous/next section), Shift+] (next energy rise), L (section loop), Shift+L (4/8/16-bar loop), Y (keep), N (pass), I (open the for-sale page), and Cmd+V, or Ctrl+V on Linux and Windows (paste a Discogs page).

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
