# Spec Delta

## MODIFIED Requirements

### Requirement: Playlist placement and size
The playlist SHALL be drawn under the header and the band (see `player-window` "Window header" and "Full-screen window") at the window's full width, its rows filling the height left, with columns when it is wide enough. Its frame SHALL be its title bar and filter bar above the rows and, below them, an edge drawn like its left edge and as wide (12 skin pixels), holding nothing: there SHALL be no footer. Its width and rows SHALL follow the window; it SHALL have no resize handle and SHALL always be shown (there is no playlist toggle). Resizing the window SHALL NOT affect playback.

#### Scenario: Fills the window
- **WHEN** the window is 1100 × 600 skin pixels and SYNC is off
- **THEN** the playlist is 1100 skin pixels wide and its rows fill the 509 pixels under the 15-pixel header and the 76-pixel band

#### Scenario: No footer
- **WHEN** the window is 720 × 435 skin pixels and SYNC is off
- **THEN** the list shows 22 rows: 435 less the 15-pixel header, the 76-pixel band, the 20-pixel title bar, the 16-pixel filter bar and the 12-pixel bottom edge, in 13-pixel rows; under the last row is only the edge

#### Scenario: Follows the window
- **WHEN** the operating system restores the window to 800 × 500 skin pixels
- **THEN** the playlist is 800 skin pixels wide, with fewer rows, and the shown crate and playback are unchanged

#### Scenario: Always shown
- **WHEN** the app starts with a `settings.ron` holding `show_playlist: false`
- **THEN** the playlist is shown
