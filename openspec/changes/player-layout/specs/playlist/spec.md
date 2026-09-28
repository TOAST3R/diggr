## ADDED Requirements

### Requirement: Playlist placement and size
The playlist SHALL be drawn to the right of the player column (main, waveform and EQ sections) in the same window. Its corner handle SHALL resize it freely in width, from the skin's playlist width (275 points at 1×) upwards, and in whole rows in height, never shorter than the player column. Its width and rows SHALL be remembered across launches. Hiding the playlist SHALL shrink the window to the player column. A restored width SHALL be reduced if the window wouldn't fit on its monitor. Resizing SHALL NOT affect playback.

#### Scenario: Widen
- **WHEN** the user drags the playlist's corner handle 300 points to the right
- **THEN** the playlist is 300 points wider, long names show in full where they fit, and the player column keeps its size

#### Scenario: Remembered size
- **WHEN** the playlist is resized to 700 × 30 rows and the app is restarted
- **THEN** it opens at 700 points wide with 30 rows

#### Scenario: Hidden playlist
- **WHEN** the user hides the playlist
- **THEN** the window is exactly as wide as the player column

### Requirement: Keyboard navigation
While the playlist has focus it SHALL show a keyboard cursor, drawn as an outline distinct from the selection highlight. The keys SHALL work as follows:
- ↑ and ↓ SHALL move the cursor by one entry and select only that entry;
- Shift+↑ and Shift+↓ SHALL extend the selection;
- PgUp and PgDn SHALL move by one visible page;
- Home and End SHALL move to the first and last entries;
- Enter SHALL play the cursor's entry (or arm it when it is waiting for audio).

The list SHALL scroll so the cursor stays visible. When there is no cursor yet, the first ↑ or ↓ SHALL put it on the playing entry, or on the first entry when nothing in the shown crate plays. The cursor SHALL follow its entry when entries are added, replaced or reordered around it.

#### Scenario: Walk down
- **WHEN** the playlist has focus, the cursor is on entry 9 of 40 with 10 visible rows ending at entry 10, and the user presses ↓ twice
- **THEN** the cursor and the selection are on entry 11, and the list has scrolled by one row so entry 11 is visible

#### Scenario: First press lands on the playing entry
- **WHEN** entry 23 plays, the playlist has just been focused with no cursor, and the user presses ↓
- **THEN** the cursor is on entry 23 and the list shows it

#### Scenario: Extend
- **WHEN** the cursor is on entry 5 and the user presses Shift+↓ three times
- **THEN** entries 5 to 8 are selected and the cursor is on entry 8

#### Scenario: Stable under dig expansion
- **WHEN** the cursor is on entry 30 and a "listed" entry above it is replaced by three clip entries
- **THEN** the cursor stays on the same entry, now numbered 32

### Requirement: Show the playing entry
Pressing P SHALL show the playing crate, scroll it so the playing entry is visible, and put the cursor on it. When the playing entry changes and the previous playing entry was visible, the list SHALL scroll by the least amount that shows the new one; when it wasn't visible, the list SHALL NOT scroll.

#### Scenario: P
- **WHEN** entry 180 plays while the list shows entries 1 to 20
- **THEN** pressing P scrolls to show entry 180 and puts the cursor on it

#### Scenario: Follow
- **WHEN** entry 20 is visible in the last row and playing, and the next track starts
- **THEN** the list scrolls by one row so entry 21 is visible

#### Scenario: Don't steal the scroll
- **WHEN** the user has scrolled to entries 200 to 220 while entry 5 plays, and the next track starts
- **THEN** the list stays at entries 200 to 220
