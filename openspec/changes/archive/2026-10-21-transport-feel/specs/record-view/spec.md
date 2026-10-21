## MODIFIED Requirements

### Requirement: Opening a record
Clicking a record row's ⏵, or pressing → while the cursor is on it and the playlist has focus, SHALL open it: its tracks are shown as track rows under it, in crate order (indented past the cover in the single-line layout). Clicking ⏷, or pressing ← on its record row or on one of its track rows while the playlist has focus, SHALL close it, leaving the cursor on its record row. Records SHALL start closed, except that the record of a track being played or armed from a track row stays open. Which records are open SHALL be kept for the session, per crate. While the player has focus, or the shown crate is flat, ← and → SHALL seek. Space SHALL NOT open or close records (it plays and pauses, see `player-window`).

#### Scenario: Open
- **WHEN** the user clicks ⏵ on a record of 3 tracks
- **THEN** its 3 tracks appear under it, indented, and the mark becomes ⏷

#### Scenario: Arrows open and close
- **WHEN** the playlist has focus, the cursor is on a closed record row, and the user presses →
- **THEN** the record opens, and pressing ← closes it again

#### Scenario: Close from a track row
- **WHEN** the playlist has focus, the cursor is on the 2nd track row of an open record, and the user presses ←
- **THEN** the record closes and the cursor is on its record row

#### Scenario: Arrows seek with the player focused
- **WHEN** the player has focus, a grouped crate is shown, and the user presses →
- **THEN** playback seeks 5 s forward, and no record opens

#### Scenario: Space doesn't open
- **WHEN** the playlist has focus, the cursor is on a closed record row, and the user presses Space
- **THEN** the record stays closed and playback toggles between playing and paused
