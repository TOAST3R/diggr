# eq-window Specification

## Purpose
TBD - created by archiving change classic-ui. Update Purpose after archive.
## Requirements
### Requirement: Equalizer section controls
The EQ window SHALL provide an ON toggle, a preamp slider, 10 band sliders (±12 dB, labeled by band frequency), and a response curve preview, all bound to the Engine's equalizer.

#### Scenario: Slider drives EQ
- **WHEN** the 1K slider is dragged to +6 dB
- **THEN** the Engine's 1 kHz band is set to +6 dB and the curve preview updates

#### Scenario: Double-click resets
- **WHEN** a band slider is double-clicked
- **THEN** it returns to 0 dB

### Requirement: Presets menu
The EQ window SHALL provide a PRESETS menu to load built-in presets and save/load/delete user presets.

#### Scenario: Save user preset
- **WHEN** the user saves the current settings as "My Club"
- **THEN** "My Club" appears in the presets menu after restart

### Requirement: EQ state persistence
EQ on/off, band, and preamp values SHALL persist across restarts.

#### Scenario: Restart
- **WHEN** the app is restarted
- **THEN** the EQ is restored to its previous settings

### Requirement: EQ window
The equalizer SHALL be shown in its own window, separate from the app's window, opened and closed by the mini player's EQ button (see `player-window`). It SHALL be drawn from the skin as the EQ section was, at the EQ section's size times the display scale, with its title bar, which drags the window, and a close button that closes it. It SHALL stay above the app's window. Whether it is open and where it is SHALL be remembered across launches; a settings file from an earlier version holding `show_eq: true` SHALL open it. Opening, closing or moving it SHALL NOT change the EQ settings or affect playback, and changes made in it SHALL be heard as from the EQ section.

#### Scenario: Open from the mini player
- **WHEN** the user clicks the mini player's EQ button
- **THEN** the EQ window opens with ON, presets, the LP knob, the curve, the preamp and the 10 bands, and the EQ button is drawn lit

#### Scenario: Close from its title bar
- **WHEN** the user clicks the EQ window's close button
- **THEN** the window closes, the EQ button is no longer lit, and the EQ settings are unchanged

#### Scenario: Remembered
- **WHEN** the user opens the EQ window, moves it, and restarts the app
- **THEN** the EQ window opens where it was

