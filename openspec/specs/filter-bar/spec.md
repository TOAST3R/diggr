# filter-bar Specification

## Purpose
One always-visible strip of skin chrome between the playlist's title bar and its list that holds everything narrowing the crate (search, BPM range, record filters, CART switch) at every width, with one tabbed filter panel, a clear-all ×, and filter shortcuts on a record's right-click.
## Requirements
### Requirement: Filter bar in the frame
The playlist SHALL show a filter bar between its title bar and its list (above the column header when there is one), in every crate and at every width, in the normal and maximized layouts. It SHALL be drawn from the skin, as part of the playlist's frame, with the search field in a sunken box. It SHALL NOT take a row of the list: the playlist's height SHALL grow by the bar's height, and the list SHALL show as many rows as before. All its text SHALL be drawn in the skin's font and LCD colour.

#### Scenario: Classic size
- **WHEN** the playlist is shown 275 skin pixels wide
- **THEN** the filter bar sits under the title bar, the list's first row is under the bar, and the list shows as many rows as it did without the bar

#### Scenario: Nothing to filter
- **WHEN** a crate of local files without tempos is shown
- **THEN** the filter bar is shown, holding the search field only

#### Scenario: Column header
- **WHEN** the playlist is wide enough for columns
- **THEN** the order from the top is title bar, filter bar, column header, rows

### Requirement: What the bar holds
From left to right, the bar SHALL hold: the search field (see `crate-search`), taking the room the other controls leave, at least 60 skin pixels and at most half the bar's width; the BPM filter control (see `playlist-filters`); the filter controls (see `record-filters` "Filter controls"); the CART switch (see `discogs-cart`) when it fits; and, while the search or any filter is set, a × that clears the search and every filter, the BPM range included. None of it SHALL overlap.

#### Scenario: Wide
- **WHEN** the collection crate, with tempos, 6 styles, 300 artists, 120 labels and 2 formats, is shown 700 pixels wide
- **THEN** the bar shows the search field, the BPM control, 6 style chips, ARTISTS, LABELS and FORMATS

#### Scenario: Maximized
- **WHEN** a crate with tempos and record filters is shown in a maximized playlist 1,000 skin pixels wide
- **THEN** the search field is at most 500 pixels wide, and the BPM control and filter buttons follow it

#### Scenario: Clear all
- **WHEN** the search is "parrish", a BPM range and two labels are set, and the user clicks × in the bar
- **THEN** the search is empty, the BPM range covers every tempo, no label is picked, and × is gone

### Requirement: Filter panel
The style, artist, label and format lists SHALL be tabs of one panel (STYLE, ARTIST, LABEL, FORMAT, showing only the filters the crate offers with at least two values), each tab keeping its list's content: each value with its number of records, the most first, a search field, a checkbox per value and Clear. In a seller crate with a copy in the cart, the panel SHALL also hold the CART switch. Each filter button in the bar SHALL open the panel on its tab; the FILTERS button SHALL open it on the first tab with a set filter, or on the first tab.

#### Scenario: Open on a tab
- **WHEN** the user clicks LABELS in the bar
- **THEN** the panel opens on its LABEL tab

#### Scenario: Switch tabs
- **WHEN** the panel is open on LABEL and the user clicks the ARTIST tab
- **THEN** the artist list is shown, with its own search field, and the picked labels are still picked

### Requirement: Filters from a row
Right-clicking an entry or record row SHALL offer, where the crate offers that filter (see `record-filters`, `style-filter`): "Only this artist", "Only this label" and "Only this style ▸" (one item per style of the record), each of which picks only that value in its filter, leaving the other filters as they are. In crates that don't offer the artist and label filters, it SHALL offer "Search ‹label›" and, when the record has one, "Search ‹catalogue number›", which set the search to that text and give the search field the keyboard.

#### Scenario: Only this label
- **WHEN** in the collection crate the user right-clicks a Lowtide Tapes record and chooses Only this label
- **THEN** only Lowtide Tapes records are shown, and LABELS is lit and reads "LABELS 1"

#### Scenario: Search a label in a seller crate
- **WHEN** in "Seller: decks.de" the user right-clicks a record on Lowtide Tapes and chooses Search Lowtide Tapes
- **THEN** the search reads "LOWTIDE TAPES", only matching records are shown, and the search field has the keyboard

