## MODIFIED Requirements

### Requirement: Waveform section
The player SHALL offer a waveform section under the main window, in the player column, toggled with `W` or the main window's waveform button and remembered across launches, showing a whole-track overview row and a zoomed row centered on the playhead.

#### Scenario: Toggle
- **WHEN** the user presses W
- **THEN** the waveform section appears under the main window, and pressing W again hides it

#### Scenario: Toggle with the button
- **WHEN** the user clicks the waveform button in the main window
- **THEN** the waveform section appears or hides, exactly as with W
