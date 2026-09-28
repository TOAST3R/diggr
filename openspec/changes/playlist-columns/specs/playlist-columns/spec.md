## ADDED Requirements

### Requirement: Column layout
When the playlist is at least 480 points wide (at 1×), each entry SHALL be drawn as aligned columns:
- number, catalog number, artist, title and BPM;
- side, year and for sale ("6 · €9.00", "none");
- time: the duration, or the entry's status where the duration would be.

A header row SHALL name the columns. Narrower than 480 points, entries SHALL be drawn in the single-line format, without a header. Empty values SHALL leave their cell blank.

#### Scenario: Wide playlist
- **WHEN** the playlist is 700 points wide and shows a Discogs entry and a local file
- **THEN** both are drawn in columns, and the local file's catalog number, side, year and for-sale cells are blank

#### Scenario: Narrow playlist
- **WHEN** the playlist is resized to 400 points wide
- **THEN** the header disappears and entries use the single-line format

### Requirement: Column widths and visibility
Dragging a header divider SHALL resize the column to its left. Right-clicking the header SHALL offer to show or hide each column except number, title and time. Widths and visibility SHALL be remembered across launches. When the playlist is resized, the columns SHALL scale with it, and the title SHALL take the remaining width.

#### Scenario: Hide a column
- **WHEN** the user right-clicks the header and hides Year, then restarts the app
- **THEN** the Year column is still hidden and the title is wider

#### Scenario: Resize a column
- **WHEN** the user drags the divider right of Artist 40 points to the right
- **THEN** the Artist column is 40 points wider and the title narrower

### Requirement: Sort the crate by a column
Clicking a column header (other than the number column), or choosing that field in OPT ▸ Sort, SHALL reorder the shown crate by that field, ascending, and a second click SHALL reorder it descending. The rules:
- The sort SHALL be stable.
- Entries without a value SHALL go last in both directions.
- Catalog numbers and sides SHALL sort in natural order (LT-2 before LT-10, A2 before A10).
- For sale SHALL sort by lowest price, with "none for sale" after priced entries and before entries with no snapshot.

The new order SHALL be the crate's order for playback, saving and export. The playing entry SHALL keep playing, and the next track SHALL follow the new order. The header SHALL mark the sorted column and direction until the crate is reordered by hand. Sorting a 1,000-entry crate SHALL take less than 16 ms and SHALL NOT affect playback.

#### Scenario: Sort by BPM
- **WHEN** a crate holds entries at 128, 122, unknown and 140 BPM and the user clicks the BPM header
- **THEN** the order is 122, 128, 140, unknown, and the header marks BPM ascending

#### Scenario: Descending keeps unknowns last
- **WHEN** the user clicks the BPM header again
- **THEN** the order is 140, 128, 122, unknown

#### Scenario: Natural catalog order
- **WHEN** entries have catalog numbers LT-10, LT-2 and LT-1 and the user sorts by Cat#
- **THEN** the order is LT-1, LT-2, LT-10

#### Scenario: Next follows the sort
- **WHEN** entry "A" plays and a sort moves entry "B" directly after it
- **THEN** "A" keeps playing without interruption, and B plays next

#### Scenario: Manual reorder clears the mark
- **WHEN** the crate is sorted by Year and the user drags an entry to another position
- **THEN** the header no longer marks Year as sorted

#### Scenario: Big crate
- **WHEN** a 1,000-entry crate is sorted by Artist during playback
- **THEN** the sort takes less than 16 ms and playback has zero underruns
