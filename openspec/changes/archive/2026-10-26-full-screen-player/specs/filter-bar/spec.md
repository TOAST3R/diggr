## MODIFIED Requirements

### Requirement: Filter bar in the frame
The playlist SHALL show a filter bar between its title bar and its list (above the column header when there is one), in every crate and at every width. It SHALL be drawn from the skin, as part of the playlist's frame, with the search field in a sunken box. It SHALL NOT take a row of the list: the playlist's height SHALL grow by the bar's height, and the list SHALL show as many rows as before. All its text SHALL be drawn in the skin's font and LCD colour.

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

#### Scenario: Very wide
- **WHEN** a crate with tempos and record filters is shown in a playlist 1,000 skin pixels wide
- **THEN** the search field is at most 500 pixels wide, and the BPM control and filter buttons follow it

#### Scenario: Clear all
- **WHEN** the search is "parrish", a BPM range and two labels are set, and the user clicks × in the bar
- **THEN** the search is empty, the BPM range covers every tempo, no label is picked, and × is gone
