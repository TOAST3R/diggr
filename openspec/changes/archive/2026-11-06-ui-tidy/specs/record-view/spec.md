# Spec Delta

## MODIFIED Requirements

### Requirement: Group by record
A crate SHALL be shown either flat (one row per entry, as before) or grouped by record. While it is grouped:
- each album of the crate (see `album-entries`) SHALL be one record row, two rows high;
- an album with one entry SHALL be a record row that acts as that entry, with nothing to open;
- an entry that belongs to no album SHALL be an ordinary row.

The choice SHALL be made with the mini player's ▤ button (lit while the shown crate is grouped, see `player-window` "Transport and sliders") or with "Group by record" in the list menu (see `playlist` "List menu"). It SHALL be remembered per crate across launches. A crate that has never been toggled SHALL be grouped, whatever its kind; a crate the user toggled SHALL keep its choice. Toggling SHALL NOT interrupt playback.

#### Scenario: Toggle
- **WHEN** a flat crate of 12 entries from 4 releases is shown and the user clicks ▤
- **THEN** the crate shows 4 record rows and ▤ is lit; clicking ▤ again shows the 12 entries flat

#### Scenario: Discogs crates default to grouped
- **WHEN** "Collection: digger" is shown for the first time
- **THEN** it is grouped

#### Scenario: Every crate defaults to grouped
- **WHEN** the user creates crate "Friday", follows a friend, and opens a Bandcamp crate, and shows each for the first time
- **THEN** each is grouped by record, as the wantlist, collection, seller and label crates are

#### Scenario: Older crates never toggled
- **WHEN** the app starts with a crate "Keepers" saved flat by default (never toggled) and a crate "Sets" the user ungrouped
- **THEN** "Keepers" is shown grouped and "Sets" stays flat

#### Scenario: Remembered
- **WHEN** the user ungroups crate "Friday" and restarts the app
- **THEN** "Friday" is still flat

#### Scenario: No Shift+G
- **WHEN** a flat crate is shown and the user presses Shift+G
- **THEN** the crate stays flat

### Requirement: Record row
A record row SHALL show, from left to right:
- the record's cover, as a square of the row's height, or an empty frame while it loads, or a record icon when there is none (a local album, no image, or a failed fetch);
- an open/close mark (⏵ or ⏷);
- on its first line, artist – album and the OWNED, CART, SOLD, format and ★ marks, and the record's Discogs styles right-aligned and dimmed (its genres when it has no style). In a seller crate, a record with exactly one unsold copy SHALL show that copy's cart pill (+ CART or IN CART, see `discogs-cart`) in place of CART. The artist SHALL be the record's credited artist when known (see `album-entries`), else its first entry's artist. The format mark (FILE, CD, CASS or OTHER) SHALL be shown only for a record with formats but no vinyl (see `discogs-intake`);
- on its second line, dimmed, the catalog number, year, number of tracks and for-sale snapshot, where known. In a seller crate, the for-sale snapshot SHALL be replaced by that seller's unsold copies and their price range ("3 copies €9.00–€18.00", or "1 copy €9.00").

When it holds the playing entry, its second line SHALL name that track with the play or pause sign, and the row SHALL be drawn in the highlight colour. In the column layout, a record row SHALL span the full width, track rows SHALL use the columns, and no column header SHALL be shown while the crate is grouped and holds an album (its row goes to the list; ☰ › Sort still sorts). A grouped crate that holds no album (only entries of no album, drawn as ordinary rows) SHALL show the column header as a flat crate does, whatever the search or filters show. The crate sidebar SHALL show a crate's number of records (albums, and entries of no album), whether the crate is grouped or flat, and its tooltip both its records and its tracks. Clicking a record row SHALL open its record card (see `record-card`), which counts the record's entries, clips and entries still to search; hovering it SHALL show no tooltip.

#### Scenario: A record
- **WHEN** a grouped crate holds 3 entries of "Glasshouse EP" by Nightcraft, LT-012, 1994, with 6 for sale from €9.00, and its cover is cached
- **THEN** one record row shows the cover, "Nightcraft – Glasshouse EP", and "LT-012 · 1994 · 3 tracks · 6 for sale from €9.00"

#### Scenario: Counting tracks and clips
- **WHEN** a grouped crate holds the 11 entries of "Mezzanine", one with its Discogs clip and 10 to search, and the user clicks its record row
- **THEN** the record card for "Mezzanine" opens, and its status strip reads "11 tracks · 1 clip · 10 to search"

#### Scenario: All clips
- **WHEN** the user opens the card of "Glasshouse EP", whose 3 entries all have Discogs clips
- **THEN** the record card shows no track and clip counts

#### Scenario: A compilation
- **WHEN** a grouped crate holds a compilation "Night Moves" credited to "Various", whose first track is by Nightcraft
- **THEN** its record row reads "Various – Night Moves"

#### Scenario: A digital-only record
- **WHEN** a grouped crate holds a record whose only format is File
- **THEN** its record row's first line shows the dim mark FILE

#### Scenario: Styles and no header
- **WHEN** a grouped crate is shown in columns and a record's styles are Deep House and Minimal
- **THEN** no column header is drawn, and the record row's first line ends with "Deep House, Minimal"

#### Scenario: Records counted
- **WHEN** the collection crate holds 4,000 tracks from 1,234 records and is grouped
- **THEN** its sidebar row shows 1,234

#### Scenario: Playing inside a closed record
- **WHEN** track A2 "Tidepool" of a closed record plays
- **THEN** the record row is highlighted and its second line reads "⏵ A2 Tidepool"

#### Scenario: In a seller crate
- **WHEN** a seller crate holds "Glasshouse EP" with copies at €9.00, €12.00 and €18.00, none sold
- **THEN** its second line reads "LT-012 · 1994 · 3 tracks · 3 copies €9.00–€18.00", and its first line shows no cart pill

#### Scenario: One copy in a seller crate
- **WHEN** a seller crate holds "Glasshouse EP" with one unsold copy at €9.00, not in the cart
- **THEN** its first line shows + CART and its second line reads "LT-012 · 1994 · 3 tracks · 1 copy €9.00"

#### Scenario: Loose tracks keep the header
- **WHEN** the default Playlist holds 30 local files without album tags, is grouped, and is shown in columns
- **THEN** the column header is drawn, and clicking its Artist header sorts the crate by artist
