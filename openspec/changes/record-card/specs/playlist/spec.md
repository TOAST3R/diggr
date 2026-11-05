# Spec Delta

## MODIFIED Requirements

### Requirement: Entry tooltip
Hovering an entry SHALL show a text-only tooltip, with no cover, with everything known about it:
- its full name, even when the row truncates it;
- its side, tempo, key and duration;
- its state and the reason;
- where its audio comes from, and how its preview was found;
- whether it is kept or passed, owned or in a cart, and whether a wantlist or collection change is pending or failed.

In a flat crate, the tooltip SHALL also show its album, style, label, catalog number and year, and its for-sale snapshot with how long ago it was fetched. Under an open record in a grouped crate, it SHALL leave these out, because the record row's card shows them (see `record-card`). Information that isn't known SHALL be left out. For a local file, the tooltip SHALL show its album and its path. Building the tooltip SHALL NOT read files or make requests.

#### Scenario: Discogs entry
- **WHEN** in a flat crate, the pointer rests on an entry from album Glasshouse EP, label Lowtide Tapes, catalog number LT-012, side A1, 1994, at 124 BPM, with 6 for sale from €9.00 fetched 3 hours ago
- **THEN** the tooltip shows all of these, including "Album: Glasshouse EP" and "fetched 3 h ago", and no cover

#### Scenario: Under an open record
- **WHEN** in a grouped crate, the pointer rests on track A1 of the open record "Glasshouse EP"
- **THEN** the tooltip shows its name, "Side: A1" and "Tempo: 124 BPM", and no album, label, catalog number, year, for-sale line or cover

#### Scenario: Truncated name
- **WHEN** an entry's name is cut off in its row
- **THEN** the tooltip shows the whole name

#### Scenario: Local file
- **WHEN** the pointer rests on a local file entry with the album tag "Geogaddi"
- **THEN** the tooltip shows its album and its path, and no Discogs fields and no cover
