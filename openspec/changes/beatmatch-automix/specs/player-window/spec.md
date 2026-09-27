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

## ADDED Requirements

### Requirement: SYNC button
The main window SHALL show a SYNC button, drawn in the skin's style, that switches SYNC mode on and off, shows whether it is on, and is remembered across launches. While a mix is planned or running, the main window SHALL show a MIX indicator with the time until the mix starts or until its handover.

#### Scenario: Toggle SYNC
- **WHEN** the user clicks SYNC
- **THEN** the button lights, SYNC mode is on, and it is still on after the app is restarted

#### Scenario: Mix countdown
- **WHEN** a mix is planned to start in 12 s
- **THEN** the main window shows "MIX 0:12", counting down
