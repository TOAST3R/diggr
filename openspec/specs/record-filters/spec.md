# record-filters Specification

## Purpose
Narrows the Discogs wantlist and collection crates by who made a record and who put it out, as diggers look for them: the record's credited artist and first label, picked in searchable lists, combined with the style and BPM filters, reachable from the footer when it has room and from ☰ at any width.
## Requirements
### Requirement: Artist and label filters
In the user's Discogs wantlist crate and collection crate, the user SHALL be able to filter the crate by the records' artists and by their labels:
- a record's artist SHALL be its credited artist as Discogs shows it ("Nightcraft", "Various", "Theo Parrish & Marcellus Pittman"), compared exactly, not split into names and not a track's own credit;
- a record's label SHALL be its first label.

Each filter SHALL be chosen in a list showing every value of the crate with its number of records, the most first, then by name, with a search field that narrows the list to values containing the typed text, ignoring case, a checkbox per value, and Clear. With no value picked, a filter SHALL show every entry. With values picked, an entry SHALL be shown when its record has any of them. An entry whose record has no artist (or no label) SHALL be shown only while that filter is off. Other crates SHALL NOT offer these filters.

#### Scenario: Pick an artist
- **WHEN** the collection crate holds records by Nightcraft (2), Theo Parrish (3) and "Theo Parrish & Marcellus Pittman" (1), and the user picks Theo Parrish
- **THEN** only the 3 Theo Parrish records are shown; the joint record is not

#### Scenario: Compilation
- **WHEN** a compilation credited to "Various" has tracks by Nightcraft and others, and the user picks Nightcraft
- **THEN** the compilation is hidden, as its record's artist is "Various"

#### Scenario: Pick labels
- **WHEN** the wantlist crate holds records on Lowtide Tapes (4), Analogical Force (2) and others, and the user picks both labels
- **THEN** the 6 records on those labels are shown

#### Scenario: Search
- **WHEN** the artist list is open and the user types "parrish"
- **THEN** only "Theo Parrish" and "Theo Parrish & Marcellus Pittman" are listed

### Requirement: Filters combine
An entry SHALL be shown when it passes every filter that is set: the BPM range, the style filter, the artist filter and the label filter. Everything the playlist-filters capability says of entries shown by the BPM filter SHALL hold for entries shown by all of them: what plays next, shuffle, the pre-warmed track, the previews downloaded ahead, keyboard navigation, the title bar's shown/all count, record rows and "k of N tracks". `P` on a hidden playing entry SHALL turn every filter off. The picked artists and labels SHALL be remembered per crate, across restarts, and a picked value that no record of the crate has any more SHALL stop filtering. Changing a filter SHALL update the list within one frame (16 ms) for a crate of 5,000 entries, SHALL NOT interrupt playback, and SHALL cause zero underruns.

#### Scenario: Artist and style
- **WHEN** Theo Parrish is picked and the style Deep House is selected
- **THEN** only Theo Parrish records with the Deep House style are shown, and the title bar shows the shown and total counts

#### Scenario: Next under the filters
- **WHEN** the label Lowtide Tapes is picked and a Lowtide Tapes track ends, followed in crate order by a record on another label and then a Lowtide Tapes record
- **THEN** the next Lowtide Tapes record's first track plays next

#### Scenario: After a restart
- **WHEN** the label Lowtide Tapes is picked in the collection crate and the app is restarted
- **THEN** the collection crate shows only its Lowtide Tapes records

#### Scenario: Big crate
- **WHEN** the user picks an artist in a grouped collection crate of 5,000 entries while a track plays
- **THEN** the list updates within 16 ms, and playback has zero underruns

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

### Requirement: Format filter
In any crate holding entries from Discogs whose records have at least two formats (see `discogs-intake`), the user SHALL be able to filter the crate by format, in a list like the other filters' (each format with its number of records, a search field, checkboxes and Clear). With formats picked, an entry SHALL be shown when its record has any of them; an entry with no known format SHALL be shown only while the filter is off. It SHALL combine with the other filters as "Filters combine" says, and SHALL be remembered per crate across restarts.

#### Scenario: Only vinyl
- **WHEN** a label crate holds 40 vinyl records and 25 digital-only ones, and the user picks Vinyl
- **THEN** only the 40 vinyl records are shown, and only their tracks play

#### Scenario: Vinyl and CD
- **WHEN** a record is on vinyl and CD, and the user picks CD
- **THEN** that record is shown

