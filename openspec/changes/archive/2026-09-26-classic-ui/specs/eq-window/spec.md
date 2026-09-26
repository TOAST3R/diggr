## ADDED Requirements

### Requirement: Equalizer section controls
The EQ section SHALL provide an ON toggle, a preamp slider, 10 band sliders (±12 dB, labeled by band frequency), and a response curve preview, all bound to the Engine's equalizer.

#### Scenario: Slider drives EQ
- **WHEN** the 1K slider is dragged to +6 dB
- **THEN** the Engine's 1 kHz band is set to +6 dB and the curve preview updates

#### Scenario: Double-click resets
- **WHEN** a band slider is double-clicked
- **THEN** it returns to 0 dB

### Requirement: Presets menu
The EQ section SHALL provide a PRESETS menu to load built-in presets and save/load/delete user presets.

#### Scenario: Save user preset
- **WHEN** the user saves the current settings as "My Club"
- **THEN** "My Club" appears in the presets menu after restart

### Requirement: EQ state persistence
EQ on/off, band, and preamp values SHALL persist across restarts.

#### Scenario: Restart
- **WHEN** the app is restarted
- **THEN** the EQ is restored to its previous settings
