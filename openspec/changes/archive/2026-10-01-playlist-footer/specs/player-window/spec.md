## ADDED Requirements

### Requirement: Options menu
Right-clicking (or Control-clicking on macOS) anywhere on the main window, or on the strip of a maximized playlist, that is not a control SHALL open the Options menu: Double size (or Classic size), Spectrogram, and, where digging is available, Discogs… and Browser…. The controls SHALL keep their own clicks. Hovering that area SHALL show a tooltip saying that a right-click opens the options. Opening the menu SHALL NOT affect playback.

#### Scenario: Open the options
- **WHEN** the user right-clicks the main window's track-info area
- **THEN** the Options menu opens with Double size, Spectrogram, Discogs… and Browser…

#### Scenario: Controls keep their clicks
- **WHEN** the user right-clicks the volume slider
- **THEN** the Options menu doesn't open

#### Scenario: Maximized
- **WHEN** the playlist is maximized and the user right-clicks the player strip
- **THEN** the Options menu opens
