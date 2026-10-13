## MODIFIED Requirements

### Requirement: Group by record
A crate SHALL be shown either flat (one row per entry, as before) or grouped by record. While it is grouped:
- each album of the crate (see `album-entries`) SHALL be one record row, two rows high;
- an album with one entry SHALL be a record row that acts as that entry, with nothing to open;
- an entry that belongs to no album SHALL be an ordinary row.

The choice SHALL be made with the ▤ button in the playlist title bar (lit while grouped) or with "Group by record" in the list menu (see `playlist` "List menu"). It SHALL be remembered per crate across launches. A crate that has never been toggled SHALL be grouped when it is the user's Discogs wantlist or collection crate, and flat otherwise. Toggling SHALL NOT interrupt playback.

#### Scenario: Toggle
- **WHEN** a flat crate of 12 entries from 4 releases is shown and the user clicks ▤
- **THEN** the crate shows 4 record rows and ▤ is lit; clicking ▤ again shows the 12 entries flat

#### Scenario: Discogs crates default to grouped
- **WHEN** "Collection: digger" is shown for the first time after this change
- **THEN** it is grouped, and the other crates are flat

#### Scenario: Remembered
- **WHEN** the user groups crate "Friday" and restarts the app
- **THEN** "Friday" is still grouped

#### Scenario: No Shift+G
- **WHEN** a flat crate is shown and the user presses Shift+G
- **THEN** the crate stays flat
