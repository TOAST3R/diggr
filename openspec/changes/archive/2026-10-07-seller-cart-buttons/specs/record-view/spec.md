## MODIFIED Requirements

### Requirement: Record row
A record row SHALL show, from left to right:
- the record's cover, as a square of the row's height, or an empty frame while it loads, or a record icon when there is none (a local album, no image, or a failed fetch);
- an open/close mark (⏵ or ⏷);
- on its first line, artist – album and the OWNED, CART, SOLD, format and ★ marks, and the record's Discogs styles right-aligned and dimmed (its genres when it has no style). In a seller crate, a record with exactly one unsold copy SHALL show that copy's cart pill (+ CART or IN CART, see `discogs-cart`) in place of CART. The artist SHALL be the record's credited artist when known (see `album-entries`), else its first entry's artist. The format mark (FILE, CD, CASS or OTHER) SHALL be shown only for a record with formats but no vinyl (see `discogs-intake`);
- on its second line, dimmed, the catalog number, year, number of tracks and for-sale snapshot, where known. In a seller crate, the for-sale snapshot SHALL be replaced by that seller's unsold copies and their price range ("3 copies €9.00–€18.00", or "1 copy €9.00").

When it holds the playing entry, its second line SHALL name that track with the play or pause sign, and the row SHALL be drawn in the highlight colour. In the column layout, a record row SHALL span the full width, track rows SHALL use the columns, and no column header SHALL be shown while the crate is grouped (its row goes to the list; ☰ › Sort still sorts). The crate sidebar SHALL show a grouped crate's number of records (albums, and entries of no album) instead of its tracks, and its tooltip both. Hovering a record row SHALL show the tooltip of its first entry.

#### Scenario: A record
- **WHEN** a grouped crate holds 3 entries of "Glasshouse EP" by Nightcraft, LT-012, 1994, with 6 for sale from €9.00, and its cover is cached
- **THEN** one record row shows the cover, "Nightcraft – Glasshouse EP", and "LT-012 · 1994 · 3 tracks · 6 for sale from €9.00"

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

### Requirement: Copy rows
In a grouped seller crate, an open record SHALL list its copies as copy rows above its tracks, cheapest first and sold ones last. Each copy row SHALL show its cart pill (+ CART or IN CART, see `discogs-cart`) when it is unsold, or the SOLD badge when it is sold, then the price, the media and sleeve condition, and the country it ships from. The pill's fixed width SHALL keep the prices of all unsold copy rows aligned. A copy row SHALL hold no audio:
- it SHALL be skipped by play order, shuffle, the next track, the preview horizon, and Enter or double-click to play;
- double-clicking it outside the pill SHALL open its listing on discogs.com in the default browser;
- its tooltip SHALL give the listing's date and comments;
- its right-click menu SHALL offer Open on discogs.com.

In the flat view, copies SHALL NOT be rows: a track's tooltip SHALL list its record's copies.

#### Scenario: Open record
- **WHEN** the user opens "Glasshouse EP" in a seller crate with copies at €12.00 (VG+/VG+) and €9.00 (VG+/VG), the €9.00 one in the cart
- **THEN** the record shows "IN CART €9.00 · VG+ / VG · Germany", then "+ CART €12.00 · VG+ / VG+ · Germany", both prices starting at the same x, then its tracks

#### Scenario: Skipped by play
- **WHEN** the last track of a record plays and the next record in the crate is open with two copy rows
- **THEN** that record's first track plays next, not a copy row

#### Scenario: Flat view
- **WHEN** the same seller crate is shown flat
- **THEN** no copy rows are shown, and a track's tooltip lists "€9.00 VG+/VG, €12.00 VG+/VG+"
