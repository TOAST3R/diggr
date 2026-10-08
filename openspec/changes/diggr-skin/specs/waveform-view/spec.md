## MODIFIED Requirements

### Requirement: Waveform section
The player SHALL offer a waveform section under the main window, in the player column, toggled with `W` or the main window's waveform button and remembered across launches, showing a whole-track overview row and a zoomed row centered on the playhead. In the player column, the section SHALL have a title bar like the equalizer's, reading "DIGGR WAVEFORM", that drags the window and has a close button that hides the waveform like `W`. While the playlist is maximized, the section SHALL be drawn as a band across the full width of the playlist area, above the playlist, with the same rows, seeking and zoom, and without the title bar.

#### Scenario: Toggle
- **WHEN** the user presses W
- **THEN** the waveform section appears under the main window, and pressing W again hides it

#### Scenario: Toggle with the button
- **WHEN** the user clicks the waveform button in the main window
- **THEN** the waveform section appears or hides, exactly as with W

#### Scenario: Title bar
- **WHEN** the waveform is on and the playlist is not maximized
- **THEN** a 14-pixel title bar reading "DIGGR WAVEFORM", with groove lines, sits between the main window and the waveform, and the player column is 14 pixels taller

#### Scenario: Close from the title bar
- **WHEN** the user clicks the waveform title bar's close button
- **THEN** the waveform section hides, exactly as with W

#### Scenario: Full-width band
- **WHEN** the playlist is maximized in a 1440-point-wide window and the waveform is on
- **THEN** the overview row spans the playlist area's full width, and clicking in it seeks
