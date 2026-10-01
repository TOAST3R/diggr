## MODIFIED Requirements

### Requirement: Column layout
When the playlist is at least 480 points wide (at 1×), each entry SHALL be drawn as aligned columns:
- number, catalog number, artist, title, album and BPM;
- side, year and for sale ("6 · €9.00", "none");
- time: the duration, or the entry's status where the duration would be.

A header row SHALL name the columns. The album SHALL appear only in its column, not in the title. Narrower than 480 points, entries SHALL be drawn in the single-line format, without a header. Empty values SHALL leave their cell blank.

#### Scenario: Wide playlist
- **WHEN** the playlist is 700 points wide and shows a Discogs entry and a local file without an album tag
- **THEN** both are drawn in columns, and the local file's catalog number, album, side, year and for-sale cells are blank

#### Scenario: Narrow playlist
- **WHEN** the playlist is resized to 400 points wide
- **THEN** the header disappears and entries use the single-line format

#### Scenario: Sort by album
- **WHEN** a crate holds entries from albums "b", "A" and none, and the user clicks the Album header
- **THEN** the order is "A", "b", then the entry without an album
