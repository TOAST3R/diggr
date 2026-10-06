## MODIFIED Requirements

### Requirement: Record row
A record row SHALL show, from left to right:
- the record's cover, as a square of the row's height, or an empty frame while it loads, or a record icon when there is none (a local album, no image, or a failed fetch);
- an open/close mark (⏵ or ⏷);
- on its first line, artist – album and the OWNED and ★ marks, and the record's Discogs styles right-aligned and dimmed (its genres when it has no style). The artist SHALL be the record's credited artist when known (see `album-entries`), else its first entry's artist;
- on its second line, dimmed, the catalog number, year, number of tracks and for-sale snapshot, where known.

When it holds the playing entry, its second line SHALL name that track with the play or pause sign, and the row SHALL be drawn in the highlight colour. In the column layout, a record row SHALL span the full width, track rows SHALL use the columns, and no column header SHALL be shown while the crate is grouped (its row goes to the list; ☰ › Sort still sorts). The crate sidebar SHALL show a grouped crate's number of records (albums, and entries of no album) instead of its tracks, and its tooltip both. Hovering a record row SHALL show the tooltip of its first entry.

#### Scenario: A record
- **WHEN** a grouped crate holds 3 entries of "Glasshouse EP" by Nightcraft, LT-012, 1994, with 6 for sale from €9.00, and its cover is cached
- **THEN** one record row shows the cover, "Nightcraft – Glasshouse EP", and "LT-012 · 1994 · 3 tracks · 6 for sale from €9.00"

#### Scenario: A compilation
- **WHEN** a grouped crate holds a compilation "Night Moves" credited to "Various", whose first track is by Nightcraft
- **THEN** its record row reads "Various – Night Moves"

#### Scenario: Styles and no header
- **WHEN** a grouped crate is shown in columns and a record's styles are Deep House and Minimal
- **THEN** no column header is drawn, and the record row's first line ends with "Deep House, Minimal"

#### Scenario: Records counted
- **WHEN** the collection crate holds 4,000 tracks from 1,234 records and is grouped
- **THEN** its sidebar row shows 1,234

#### Scenario: Playing inside a closed record
- **WHEN** track A2 "Tidepool" of a closed record plays
- **THEN** the record row is highlighted and its second line reads "⏵ A2 Tidepool"
