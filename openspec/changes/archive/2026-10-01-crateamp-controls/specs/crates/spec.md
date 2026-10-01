## ADDED Requirements

### Requirement: Crate sidebar
When the playlist is at least 600 skin pixels wide, or maximized, and the sidebar is on, a list of every crate SHALL be shown on the left of the playlist, with each crate's number of entries, and the crate menu's marks for the playing crate (⏵) and the shown crate (•). Clicking a crate SHALL show it. Right-clicking SHALL offer Rename… and Delete…, with the same rules as the crate menu. "+ New crate" SHALL create and show a new crate, after asking its name as the crate menu does. A crate that can't be read SHALL be dimmed and can't be chosen. A ☰ button in the playlist title bar SHALL show or hide the sidebar, and the choice SHALL be remembered across restarts. The crate menu on the title bar SHALL keep working. Showing a crate from the sidebar SHALL NOT interrupt playback.

#### Scenario: Switch from the sidebar
- **WHEN** the playlist is maximized and the user clicks "Keepers" in the sidebar
- **THEN** the Keepers crate is shown, the sidebar marks it •, and the playing track plays on

#### Scenario: Too narrow
- **WHEN** the playlist is 400 px wide and not maximized
- **THEN** no sidebar is shown, and the crate menu still works

#### Scenario: Hidden and remembered
- **WHEN** the user clicks ☰ to hide the sidebar and restarts the app with the playlist maximized
- **THEN** no sidebar is shown until ☰ is clicked again

### Requirement: Drop entries on a crate
Dragging entries onto a crate in the sidebar SHALL send them to that crate exactly as Send to crate does: the whole selection when the dragged entry is part of it, skipping entries the crate already holds, with the same message. The crate under the pointer SHALL be highlighted while dragging. Dropping on the shown crate SHALL do nothing. Dropping inside the list SHALL still reorder.

#### Scenario: Drop a selection
- **WHEN** entries 3 to 5 are selected and dragged onto "Friday", which already holds entry 4's clip
- **THEN** entries 3 and 5 are added to Friday, and the message says 2 were sent (1 already there)

#### Scenario: Drop in the list
- **WHEN** an entry is dragged and dropped between two rows of the list
- **THEN** it is reordered there, and no crate receives it
