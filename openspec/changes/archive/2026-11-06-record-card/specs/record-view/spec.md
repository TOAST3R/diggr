# Spec Delta

## MODIFIED Requirements

### Requirement: Record row
A record row SHALL show, from left to right:
- the record's cover, as a square of the row's height, or an empty frame while it loads, or a record icon when there is none (a local album, no image, or a failed fetch);
- an open/close mark (⏵ or ⏷);
- on its first line, artist – album and the OWNED, CART, SOLD, format and ★ marks, and the record's Discogs styles right-aligned and dimmed (its genres when it has no style). In a seller crate, a record with exactly one unsold copy SHALL show that copy's cart pill (+ CART or IN CART, see `discogs-cart`) in place of CART. The artist SHALL be the record's credited artist when known (see `album-entries`), else its first entry's artist. The format mark (FILE, CD, CASS or OTHER) SHALL be shown only for a record with formats but no vinyl (see `discogs-intake`);
- on its second line, dimmed, the catalog number, year, number of tracks and for-sale snapshot, where known. In a seller crate, the for-sale snapshot SHALL be replaced by that seller's unsold copies and their price range ("3 copies €9.00–€18.00", or "1 copy €9.00").

When it holds the playing entry, its second line SHALL name that track with the play or pause sign, and the row SHALL be drawn in the highlight colour. In the column layout, a record row SHALL span the full width, track rows SHALL use the columns, and no column header SHALL be shown while the crate is grouped (its row goes to the list; ☰ › Sort still sorts). The crate sidebar SHALL show a crate's number of records (albums, and entries of no album), whether the crate is grouped or flat, and its tooltip both its records and its tracks. Clicking a record row SHALL open its record card (see `record-card`), which counts the record's entries, clips and entries still to search; hovering it SHALL show no tooltip.

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

### Requirement: Acting on a record
In a grouped crate, a record row SHALL act on all of its entries:
- clicking a record row SHALL select all its entries and open its record card (see `record-card`). Shift-click and Cmd- or Ctrl-click SHALL extend or toggle the selection by whole records, and SHALL NOT open the card;
- right-clicking a record row SHALL open the entry menu with the record's entries selected, so that Remove, Send to crate and the Discogs items act on the whole record. Remove album and Select album SHALL NOT be offered there. In the wantlist crate and the collection crate, Remove SHALL NOT be offered either (see `discogs-write`);
- double-clicking a record row, or pressing Enter on it, SHALL play its first playable track, or arm it when none is playable yet, and SHALL leave its card open;
- dragging a record row SHALL move all its entries together, between other records. Dropping it on a crate in the sidebar SHALL send them all, unless that crate takes no hand edits;
- dragging a track row SHALL reorder it only within its record, and no insertion line SHALL be drawn outside it.

#### Scenario: Remove a record
- **WHEN** the user right-clicks a record row of 3 tracks in a dig crate and chooses Remove
- **THEN** its 3 entries are removed

#### Scenario: No Remove in the collection crate
- **WHEN** the user right-clicks a record row in the collection crate
- **THEN** the menu offers no Remove

#### Scenario: Play a record
- **WHEN** the user double-clicks a record row whose first track is still downloading and whose second is playable
- **THEN** the second track plays

#### Scenario: Move a record
- **WHEN** the user drags the third record row above the first
- **THEN** all of that record's entries come first in the crate, in their order

#### Scenario: Click opens the card
- **WHEN** the user clicks a record row
- **THEN** its entries are selected and its record card opens

#### Scenario: Cmd-click
- **WHEN** the user Cmd-clicks a second record row
- **THEN** both records are selected and no card opens
