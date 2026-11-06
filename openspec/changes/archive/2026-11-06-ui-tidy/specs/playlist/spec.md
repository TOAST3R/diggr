# Spec Delta

## MODIFIED Requirements

### Requirement: Playlist placement and size
The playlist SHALL be drawn under the band (see `player-window` "Full-screen window") at the window's full width, its rows filling the height left, with columns when it is wide enough. Its frame SHALL be its title bar and filter bar above the rows and, below them, an edge drawn like its left edge and as wide (12 skin pixels), holding nothing: there SHALL be no footer. Its width and rows SHALL follow the window; it SHALL have no resize handle and SHALL always be shown (there is no playlist toggle). Resizing the window SHALL NOT affect playback.

#### Scenario: Fills the window
- **WHEN** the window is 1100 × 600 skin pixels and SYNC is off
- **THEN** the playlist is 1100 skin pixels wide and its rows fill the 542 pixels under the 58-pixel band

#### Scenario: No footer
- **WHEN** the window is 720 × 435 skin pixels and SYNC is off
- **THEN** the list shows 25 rows (23 with the old footer): 435 less the 58-pixel band, the 20-pixel title bar, the 16-pixel filter bar and the 12-pixel bottom edge, in 13-pixel rows; under the last row is only the edge

#### Scenario: Follows the window
- **WHEN** the operating system restores the window to 800 × 500 skin pixels
- **THEN** the playlist is 800 skin pixels wide, with fewer rows, and the shown crate and playback are unchanged

#### Scenario: Always shown
- **WHEN** the app starts with a `settings.ron` holding `show_playlist: false`
- **THEN** the playlist is shown

### Requirement: Title bar buttons
The playlist's title bar SHALL hold the shown crate's name and no button: no ▤ (which is in the mini player, see `record-view`), no close button and no button that maximizes or restores the playlist.

#### Scenario: Title bar buttons
- **WHEN** the playlist is shown
- **THEN** its title bar shows the crate's name, and no ▤, close or maximize button

## REMOVED Requirements

### Requirement: Playlist footer
**Reason**: The footer held only the Options gear and the "selected/total" time readout; the gear moves to the band's window corner (see `player-window` "Full-screen window") and the time readout to the filter bar (see `filter-bar` "What the bar holds"), and the list takes the footer's height.
**Migration**: Click the gear between minimize and close for the Options menu; read the crate's time after the search field. Adding music and M3U import and export stay in the list menu (see "List menu").
