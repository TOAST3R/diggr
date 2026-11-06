# Spec Delta

## MODIFIED Requirements

### Requirement: Options menu
Right-clicking (or Control-clicking on macOS) anywhere on the mini player that is not a control SHALL open the Options menu: where digging is available, Discogs…, Browser… and Set up Diggr…, then Reduce motion, checked while on (see `ui-motion` "Reduce motion"). The menu SHALL NOT offer the size (see "Size keys") or the spectrogram, which has the mini player's spectrum button and S. The controls SHALL keep their own clicks. Hovering that area SHALL show a tooltip saying that a right-click opens the options. The gear button in the window header (see "Window header") SHALL open the same menu, so it can be found without knowing about the right-click; where digging isn't available the menu is empty and the gear SHALL NOT be shown. Opening the menu SHALL NOT affect playback.

#### Scenario: Open the options
- **WHEN** the user right-clicks the mini player's title line
- **THEN** the Options menu opens with Discogs…, Browser…, Set up Diggr… and Reduce motion, and without Double size, Classic size or Spectrogram

#### Scenario: Controls keep their clicks
- **WHEN** the user right-clicks the volume slider
- **THEN** the Options menu doesn't open

#### Scenario: Options from the gear
- **WHEN** the user clicks the gear between minimize and close and chooses Discogs…
- **THEN** the Discogs dialog opens, as from the right-click Options menu

#### Scenario: No digging, no gear
- **WHEN** the app runs where digging isn't available
- **THEN** the header shows help, minimize and close only, each in its usual place
