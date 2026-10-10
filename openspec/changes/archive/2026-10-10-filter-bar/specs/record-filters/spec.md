## MODIFIED Requirements

### Requirement: Artist and label filters
In the user's Discogs wantlist crate and collection crate, the user SHALL be able to filter the crate by the records' artists and by their labels:
- a record's artist SHALL be its credited artist as Discogs shows it ("Nightcraft", "Various", "Theo Parrish & Marcellus Pittman"), compared exactly, not split into names and not a track's own credit;
- a record's label SHALL be its first label.

Each filter SHALL be chosen in its tab of the filter panel (see `filter-bar`), showing every value of the crate with its number of records, the most first, then by name, with a search field that narrows the list to values containing the typed text, ignoring case, a checkbox per value, and Clear. With no value picked, a filter SHALL show every entry. With values picked, an entry SHALL be shown when its record has any of them. An entry whose record has no artist (or no label) SHALL be shown only while that filter is off. Other crates SHALL NOT offer these filters.

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
- **WHEN** the ARTIST tab is open and the user types "parrish"
- **THEN** only "Theo Parrish" and "Theo Parrish & Marcellus Pittman" are listed

### Requirement: Filters combine
An entry SHALL be shown when it matches the search (see `crate-search`) and passes every filter that is set: the BPM range, the style filter, the artist filter, the label filter, the format filter and the CART switch (see `discogs-cart`). Everything the playlist-filters capability says of entries shown by the BPM filter SHALL hold for entries shown by all of them: what plays next, shuffle, the pre-warmed track, the previews downloaded ahead, keyboard navigation, the title bar's shown/all count, record rows and "k of N tracks". `P` on a hidden playing entry SHALL clear the search and turn every filter off. The picked artists and labels SHALL be remembered per crate, across restarts, and a picked value that no record of the crate has any more SHALL stop filtering. Changing a filter SHALL update the list within one frame (16 ms) for a crate of 5,000 entries, SHALL NOT interrupt playback, and SHALL cause zero underruns.

#### Scenario: Artist and style
- **WHEN** Theo Parrish is picked and the style Deep House is selected
- **THEN** only Theo Parrish records with the Deep House style are shown, and the title bar shows the shown and total counts

#### Scenario: Search and label
- **WHEN** the label Lowtide Tapes is picked and the search is "012"
- **THEN** only Lowtide Tapes records matching "012" are shown

#### Scenario: Next under the filters
- **WHEN** the label Lowtide Tapes is picked and a Lowtide Tapes track ends, followed in crate order by a record on another label and then a Lowtide Tapes record
- **THEN** the next Lowtide Tapes record's first track plays next

#### Scenario: After a restart
- **WHEN** the label Lowtide Tapes is picked in the collection crate and the app is restarted
- **THEN** the collection crate shows only its Lowtide Tapes records

#### Scenario: Big crate
- **WHEN** the user picks an artist in a grouped collection crate of 5,000 entries while a track plays
- **THEN** the list updates within 16 ms, and playback has zero underruns

#### Scenario: Cart and format
- **WHEN** in a seller crate the CART switch is on and the format Vinyl is picked
- **THEN** only vinyl records with a copy in the cart are shown and played

### Requirement: Filter controls
After the BPM filter control, the filter bar (see `filter-bar`) SHALL show, of the filters the shown crate offers (see "Format filter"; style, artist and label in the wantlist and collection crates) and whose crate has at least two values:
- the style chips (see `style-filter`) followed by an ARTISTS, a LABELS and a FORMATS button, when all of them fit;
- otherwise, a STYLES, an ARTISTS, a LABELS and a FORMATS button, when they fit;
- otherwise, one FILTERS button, which reads "FILTERS ‹n›" and is lit while n of those filters (and the CART switch) are set.

In a seller crate with a copy in the cart, the CART switch (see `discogs-cart`) SHALL come after the buttons when it fits, and SHALL otherwise be in the filter panel; it SHALL be lit while on.

A button SHALL be lit while its filter is set and SHALL then show how many values are picked ("ARTISTS 2"). Clicking a button SHALL open the filter panel on its tab, and double-clicking it SHALL turn its filter off. The ☰ menu SHALL NOT offer filter items.

#### Scenario: Wide playlist
- **WHEN** the collection crate, with 6 styles, 300 artists, 120 labels and 2 formats, is shown 700 pixels wide
- **THEN** the filter bar shows the 6 style chips, then ARTISTS, LABELS and FORMATS

#### Scenario: Middle width
- **WHEN** the same crate is shown 450 pixels wide, where the chips and buttons don't all fit
- **THEN** the filter bar shows STYLES, ARTISTS, LABELS and FORMATS

#### Scenario: Classic width
- **WHEN** the same crate, whose records have tempos, is shown 275 pixels wide with one label picked
- **THEN** the filter bar shows the search field, the BPM control and "FILTERS 1", lit, and FILTERS opens the panel on its LABEL tab

#### Scenario: Lit button
- **WHEN** two labels are picked
- **THEN** the LABELS button is lit and reads "LABELS 2", and a double-click on it shows every label again

#### Scenario: Menu
- **WHEN** the user opens ☰ in the collection crate with a style and an artist set
- **THEN** it offers no Filter by… item and no Show all records

#### Scenario: Dig crate
- **WHEN** a crate filled from a label, holding vinyl and digital records, is shown 400 pixels wide
- **THEN** the filter bar shows only FORMATS after the BPM control, and the panel offers only the FORMAT tab

#### Scenario: Seller crate with a cart
- **WHEN** a seller crate with vinyl and CD records and 3 copies in the cart is shown 500 pixels wide
- **THEN** the filter bar shows FORMATS, then "CART 3 · €41.20"
