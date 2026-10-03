## MODIFIED Requirements

### Requirement: Playlist footer
The playlist footer SHALL hold, from left to right: a `+` button, a `≡` button, a gear button, the BPM filter control (when the crate has two different known tempos), and the "selected/total" time readout, right-aligned in its LCD box beside the resize grip (only the total when both don't fit in the box). Everything SHALL fit at the classic 275 skin pixel width. The buttons SHALL open menus:
- `+`: Add files…, Add folder…, Import M3U…;
- `≡`: Select all, Select none, Invert selection, Remove selected, Clear crate, Sort ▸ (every column's field), Group by record (a checkbox, see `record-view`), Show all tempos (only while a BPM range is set), Export M3U…;
- the gear: the Options menu (see the player window's Options menu), with the same items as the right-click.

Each item SHALL act as the same item did in the footer's earlier menus. No app setting SHALL be in the `+` and `≡` menus.

#### Scenario: Three buttons
- **WHEN** the playlist is shown at 275 skin pixels wide
- **THEN** the footer shows `+`, `≡`, the gear and the time readout, and no ADD, REM, SEL, MISC or OPT button

#### Scenario: Options from the footer
- **WHEN** the user clicks the gear and chooses Discogs…
- **THEN** the Discogs dialog opens, as from the right-click Options menu

#### Scenario: Add
- **WHEN** the user clicks `+` and chooses Add folder…
- **THEN** the folder dialog opens, as ADD ▸ Add folder… did

#### Scenario: Sort from the footer
- **WHEN** the user clicks `≡` and chooses Sort ▸ BPM
- **THEN** the crate is sorted by BPM, as by clicking the BPM column header

#### Scenario: Group from the menu
- **WHEN** the user clicks `≡` and ticks Group by record
- **THEN** the shown crate is grouped, as with Shift+G

### Requirement: Maximized playlist
A toggle SHALL maximize the playlist: the ⇔ button in the playlist's title bar, or Shift+P. The title bar SHALL hold, from the right: the close button, ⇔, and the ▤ button that groups the shown crate by record (see `record-view`), all at the classic 275 skin pixel width. While maximized:
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

#### Scenario: Title bar buttons
- **WHEN** the playlist is shown at 275 skin pixels wide
- **THEN** its title bar shows ▤, ⇔ and the close button at its right end, and the crate name is shortened to fit before them
