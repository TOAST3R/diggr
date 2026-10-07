## MODIFIED Requirements

### Requirement: Filters combine
An entry SHALL be shown when it passes every filter that is set: the BPM range, the style filter, the artist filter, the label filter and the CART switch (see `discogs-cart`). Everything the playlist-filters capability says of entries shown by the BPM filter SHALL hold for entries shown by all of them: what plays next, shuffle, the pre-warmed track, the previews downloaded ahead, keyboard navigation, the title bar's shown/all count, record rows and "k of N tracks". `P` on a hidden playing entry SHALL turn every filter off. The picked artists and labels SHALL be remembered per crate, across restarts, and a picked value that no record of the crate has any more SHALL stop filtering. Changing a filter SHALL update the list within one frame (16 ms) for a crate of 5,000 entries, SHALL NOT interrupt playback, and SHALL cause zero underruns.

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

#### Scenario: Cart and format
- **WHEN** in a seller crate the CART switch is on and the format Vinyl is picked
- **THEN** only vinyl records with a copy in the cart are shown and played

### Requirement: Filter controls
After the BPM filter control, and clear of the time readout's box, the playlist footer SHALL show, of the filters the shown crate offers (see "Format filter"; style, artist and label in the wantlist and collection crates) and whose crate has at least two values:
- the style chips (see `style-filter`) followed by an ARTISTS, a LABELS and a FORMATS button, when all of them fit;
- otherwise, a STYLES, an ARTISTS, a LABELS and a FORMATS button, when they fit;
- otherwise, none of them.

In a seller crate with a copy in the cart, the CART switch (see `discogs-cart`) SHALL come last, after the buttons, when it fits; it SHALL be lit while on.

A button SHALL be lit while its filter is set and SHALL then show how many values are picked ("ARTISTS 2"). Clicking a button SHALL open its list, and double-clicking it SHALL turn its filter off. The ☰ menu SHALL offer Filter by style…, Filter by artist…, Filter by label… and Filter by format… for the filters offered (with at least two values), which open the same lists at any width, and Show all records while a style, artist, label or format filter or the CART switch is set, which turns those off and leaves the BPM range as it is.

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

#### Scenario: Seller crate with a cart
- **WHEN** a seller crate with vinyl and CD records and 3 copies in the cart is shown 500 pixels wide
- **THEN** the footer shows FORMATS, then "CART 3 · €41.20"
