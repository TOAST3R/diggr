## MODIFIED Requirements

### Requirement: Keyboard shortcuts
The system SHALL support Z (previous), X (play), C (pause), V (stop), B (next), ←/→ (seek ∓5 s), ↑/↓ (volume), F (fullscreen visuals), H or F1 (shortcuts help), W (waveform section), [ and ] (previous/next section), Shift+] (next energy rise), L (section loop), and Shift+L (4/8/16-bar loop).

#### Scenario: Classic keys
- **WHEN** the user presses B during playback
- **THEN** the next track starts

#### Scenario: Structure keys
- **WHEN** the user presses ] during playback of an analyzed track
- **THEN** playback moves to the next section on the next downbeat

## ADDED Requirements

### Requirement: Shortcuts help
`H` or `F1` SHALL open a panel listing every keyboard and mouse shortcut, grouped by area, in the player window and in fullscreen; `H`, `F1`, `Esc` or the panel's close button SHALL close it, and `Esc` SHALL close the panel before it leaves fullscreen.

#### Scenario: Open help
- **WHEN** the user presses H in the player window
- **THEN** a panel lists the playback, structure, player window, fullscreen visuals and mouse shortcuts

#### Scenario: Esc in fullscreen
- **WHEN** the help panel is open in fullscreen and the user presses Esc
- **THEN** the panel closes and fullscreen stays on
