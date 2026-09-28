## MODIFIED Requirements

### Requirement: Keyboard shortcuts
The system SHALL support Z (previous), X (play), C (pause), V (stop), B (next), Shift+B (mix into the next track), ←/→ (seek ∓5 s), ↑/↓ (volume), F (fullscreen visuals), H or F1 (shortcuts help), W (waveform section), S (spectrogram window), [ and ] (previous/next section), Shift+] (next energy rise), L (section loop), Shift+L (4/8/16-bar loop), and E (swap kick and bass during a mix).

#### Scenario: Classic keys
- **WHEN** the user presses B during playback
- **THEN** the next track starts

#### Scenario: Structure keys
- **WHEN** the user presses ] during playback of an analyzed track
- **THEN** playback moves to the next section on the next downbeat

#### Scenario: Spectrogram key
- **WHEN** the user presses S in the player window
- **THEN** the spectrogram window opens

#### Scenario: Mix key
- **WHEN** the user presses Shift+B during playback of an analyzed track
- **THEN** a mix into the next track begins at the next phrase boundary

### Requirement: Transport and sliders
The main section SHALL provide previous, play, pause, stop, next, open, shuffle, repeat and SYNC buttons, a seek bar, a volume slider, and a balance slider, all wired to the Engine. The shuffle and repeat buttons SHALL be icon buttons: crossed arrows for shuffle, a loop arrow for repeat, and a loop with a "1" for repeat one, drawn lit when on.

#### Scenario: Drag seek bar
- **WHEN** the user drags the seek bar and releases at 75%
- **THEN** the Engine seeks to 75% of the track duration on release

#### Scenario: Volume change is immediate
- **WHEN** the volume slider is moved
- **THEN** the loudness change is audible within 20 ms

#### Scenario: Repeat icon states
- **WHEN** the user clicks repeat twice starting from off
- **THEN** the button shows the lit loop, then the lit loop with a "1", and playback repeats all and then one track

## ADDED Requirements

### Requirement: SYNC button
The main window SHALL show a SYNC button, drawn in the skin's style at the right end of the button row, right of the shuffle and repeat buttons, that switches SYNC mode (vinyl: pitch follows tempo) on and off, shows whether it is on, and is remembered across launches. While a mix is planned or running, the main window SHALL show a MIX indicator with the time until the mix starts or until its handover.

#### Scenario: Toggle SYNC
- **WHEN** the user clicks SYNC
- **THEN** the button lights, SYNC mode is on, and it is still on after the app is restarted

#### Scenario: Mix countdown
- **WHEN** a mix is planned to start in 12 s
- **THEN** the main window shows "MIX 0:12", counting down
