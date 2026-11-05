# Spec Delta

## MODIFIED Requirements

### Requirement: Options menu
Right-clicking (or Control-clicking on macOS) anywhere on the mini player that is not a control SHALL open the Options menu: where digging is available, Discogs…, Browser… and Set up Diggr…. The menu SHALL NOT offer the size (see "Size keys") or the spectrogram, which has the mini player's spectrum button and S. The controls SHALL keep their own clicks. Hovering that area SHALL show a tooltip saying that a right-click opens the options. The playlist footer's gear button SHALL open the same menu, so it can be found without knowing about the right-click. Opening the menu SHALL NOT affect playback.

#### Scenario: Open the options
- **WHEN** the user right-clicks the mini player's title line
- **THEN** the Options menu opens with Discogs…, Browser… and Set up Diggr…, and without Double size, Classic size or Spectrogram

#### Scenario: Controls keep their clicks
- **WHEN** the user right-clicks the volume slider
- **THEN** the Options menu doesn't open

## ADDED Requirements

### Requirement: Size keys
Cmd+= or Cmd++ (Ctrl on Linux and Windows) SHALL draw the app at double size (2×), and Cmd+− and Cmd+0 at classic size (1×). The size SHALL be remembered across launches. A key asking for the size already shown SHALL change nothing. The skin SHALL only ever be drawn at whole-number scales: no other key or gesture zooms the interface. The shortcuts help SHALL list the size keys. Changing the size SHALL NOT affect playback.

#### Scenario: Double size
- **WHEN** the app is at classic size and the user presses Cmd+=
- **THEN** the band, the playlist and the EQ window are drawn at 2×, playback goes on, and the next launch opens at 2×

#### Scenario: Classic size
- **WHEN** the app is at double size and the user presses Cmd+−
- **THEN** the app is drawn at 1×

#### Scenario: Reset
- **WHEN** the app is at double size and the user presses Cmd+0
- **THEN** the app is drawn at 1×

#### Scenario: Already that size
- **WHEN** the app is at double size and the user presses Cmd+= again
- **THEN** nothing changes, and the interface isn't zoomed past 2×

#### Scenario: Listed in the help
- **WHEN** the user presses H
- **THEN** the help lists Cmd+= / Cmd+− / Cmd+0 for the size
