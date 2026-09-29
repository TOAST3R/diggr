## ADDED Requirements

### Requirement: Maximized playlist
A toggle SHALL maximize the playlist: the ⇔ button in the playlist's title bar, or Shift+P. While maximized:
- the window SHALL fill the screen's usable area (below the menu bar), as the operating system maximizes it;
- the player SHALL be a thin strip on the left, at most 27 skin pixels wide, showing only the play state, the elapsed time, previous, play or pause, next, and the ⇔ button;
- the playlist SHALL take the rest of the window, its width and rows following the window, with columns when it is wide enough;
- the EQ SHALL be hidden, and the keyboard SHALL go to the playlist.

Toggling again SHALL restore the previous window frame, layout, playlist width and rows exactly. When the playlist is hidden, the toggle SHALL show it first. The mode SHALL be remembered across launches, and a launch in this mode SHALL stay within the 300 ms launch target. Entering or leaving the mode SHALL NOT affect playback.

#### Scenario: Maximize
- **WHEN** the window is 1100 × 580 points at 2× on a 1440 × 900 display, and the user presses Shift+P
- **THEN** the window fills the area below the menu bar, the player is a thin strip on the left, and the playlist fills the rest, with columns

#### Scenario: Restore
- **WHEN** the user clicks ⇔ while maximized
- **THEN** the window returns to 1100 × 580 points at its previous position, with the player column and the playlist at their previous width and rows

#### Scenario: Strip controls
- **WHEN** the playlist is maximized and the user clicks next in the strip
- **THEN** the next track starts, exactly as with B

#### Scenario: Remembered
- **WHEN** the app is quit while maximized and launched again
- **THEN** its first frame is already maximized, within 300 ms of launch

### Requirement: Trackpad scrolling
Scrolling the playlist SHALL add up partial scroll steps, so that slow two-finger trackpad scrolling, and its momentum, move the list one row per row-height of scrolling. Scrolling SHALL work anywhere over the list, entries included. Leftover partial steps SHALL be dropped when the pointer leaves the list or scrolling pushes past its top or bottom.

#### Scenario: Slow scroll
- **WHEN** the user scrolls the playlist down by 3 points per frame for 10 frames with 13-point rows
- **THEN** the list moves down by 2 rows

#### Scenario: No jump later
- **WHEN** 10 points of scrolling are left over and the pointer leaves the list and comes back
- **THEN** the next 3 points of scrolling don't move the list
