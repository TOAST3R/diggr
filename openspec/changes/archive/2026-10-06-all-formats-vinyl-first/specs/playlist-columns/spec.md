## MODIFIED Requirements

### Requirement: Column layout
When the playlist is at least 480 points wide (at 1×), each entry SHALL be drawn as aligned columns:
- number, catalog number, artist, title, album, format and BPM;
- side, year and for sale ("6 · €9.00", "none");
- time: the duration, or the entry's status where the duration would be.

A header row SHALL name the columns. The album SHALL appear only in its column, not in the title. The format column SHALL show the record's formats ("Vinyl", "File", "Vinyl, CD"), and sorting by it SHALL order Vinyl first, then File, CD, Cassette and Other, with unknown formats last. Narrower than 480 points, entries SHALL be drawn in the single-line format, without a header. Empty values SHALL leave their cell blank. Column settings saved before the format column existed SHALL show it at its default width.

#### Scenario: Wide playlist
- **WHEN** the playlist is 700 points wide and shows a Discogs entry and a local file without an album tag
- **THEN** both are drawn in columns, and the local file's catalog number, album, format, side, year and for-sale cells are blank

#### Scenario: Narrow playlist
- **WHEN** the playlist is resized to 400 points wide
- **THEN** the header disappears and entries use the single-line format

#### Scenario: Sort by album
- **WHEN** a crate holds entries from albums "b", "A" and none, and the user clicks the Album header
- **THEN** the order is "A", "b", then the entry without an album

#### Scenario: Sort by format
- **WHEN** a crate holds File, Vinyl, unknown and CD entries, and the user clicks the Format header
- **THEN** the order is Vinyl, File, CD, then the unknown one
