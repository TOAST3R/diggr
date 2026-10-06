## MODIFIED Requirements

### Requirement: Filter controls
After the BPM filter control, and clear of the time readout's box, the playlist footer SHALL show, of the filters the shown crate offers (see "Format filter"; style, artist and label in the wantlist and collection crates) and whose crate has at least two values:
- the style chips (see `style-filter`) followed by an ARTISTS, a LABELS and a FORMATS button, when all of them fit;
- otherwise, a STYLES, an ARTISTS, a LABELS and a FORMATS button, when they fit;
- otherwise, none of them.

A button SHALL be lit while its filter is set and SHALL then show how many values are picked ("ARTISTS 2"). Clicking a button SHALL open its list, and double-clicking it SHALL turn its filter off. The ☰ menu SHALL offer Filter by style…, Filter by artist…, Filter by label… and Filter by format… for the filters offered (with at least two values), which open the same lists at any width, and Show all records while a style, artist, label or format filter is set, which turns those off and leaves the BPM range as it is.

#### Scenario: Wide playlist
- **WHEN** the collection crate, with 6 styles, 300 artists, 120 labels and 2 formats, is shown 700 pixels wide
- **THEN** the footer shows the 6 style chips, then ARTISTS, LABELS and FORMATS

#### Scenario: Middle width
- **WHEN** the same crate is shown 450 pixels wide, where the chips and buttons don't all fit
- **THEN** the footer shows STYLES, ARTISTS, LABELS and FORMATS

#### Scenario: Classic width
- **WHEN** the same crate, whose records have tempos, is shown 275 pixels wide
- **THEN** the footer shows only the BPM control, and ☰ ▸ Filter by artist… opens the artist list

#### Scenario: Lit button
- **WHEN** two labels are picked
- **THEN** the LABELS button is lit and reads "LABELS 2", and a double-click on it shows every label again

#### Scenario: Show all records
- **WHEN** a style, an artist and a BPM range are set, and the user chooses ☰ ▸ Show all records
- **THEN** the style and artist filters are off and the BPM range is still set

#### Scenario: Dig crate
- **WHEN** a crate filled from a label, holding vinyl and digital records, is shown 400 pixels wide
- **THEN** the footer shows only FORMATS after the BPM control, and ☰ offers Filter by format… but not the style, artist or label lists

## ADDED Requirements

### Requirement: Format filter
In any crate holding entries from Discogs whose records have at least two formats (see `discogs-intake`), the user SHALL be able to filter the crate by format, in a list like the other filters' (each format with its number of records, a search field, checkboxes and Clear). With formats picked, an entry SHALL be shown when its record has any of them; an entry with no known format SHALL be shown only while the filter is off. It SHALL combine with the other filters as "Filters combine" says, and SHALL be remembered per crate across restarts.

#### Scenario: Only vinyl
- **WHEN** a label crate holds 40 vinyl records and 25 digital-only ones, and the user picks Vinyl
- **THEN** only the 40 vinyl records are shown, and only their tracks play

#### Scenario: Vinyl and CD
- **WHEN** a record is on vinyl and CD, and the user picks CD
- **THEN** that record is shown
