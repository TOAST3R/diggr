# Spec Delta

## MODIFIED Requirements

### Requirement: What the bar holds
From left to right, the bar SHALL hold: the search field (see `crate-search`), taking the room the other controls leave, at least 60 skin pixels and at most half the bar's width; the crate's time readout; the BPM filter control (see `playlist-filters`); the filter controls (see `record-filters` "Filter controls"); the CART switch (see `discogs-cart`) when it fits; and, while the search or any filter is set, a × that clears the search and every filter, the BPM range included. None of it SHALL overlap.

#### Scenario: Wide
- **WHEN** the collection crate, with tempos, 6 styles, 300 artists, 120 labels and 2 formats, is shown 700 pixels wide
- **THEN** the bar shows the search field, the time readout, the BPM control, 6 style chips, ARTISTS, LABELS and FORMATS

#### Scenario: Very wide
- **WHEN** a crate with tempos and record filters is shown in a playlist 1,000 skin pixels wide
- **THEN** the search field is at most 500 pixels wide, and the time readout, the BPM control and the filter buttons follow it

#### Scenario: Clear all
- **WHEN** the search is "parrish", a BPM range and two labels are set, and the user clicks × in the bar
- **THEN** the search is empty, the BPM range covers every tempo, no label is picked, and × is gone

## ADDED Requirements

### Requirement: Time readout in the bar
The bar SHALL show the shown crate's "selected/total" time right after the search field, in a sunken box like the search field's: each time as m:ss, or h:mm:ss from an hour, followed by `+` when an entry's duration isn't known yet. It SHALL count the whole crate, as the footer's readout did, whatever the search and filters show. When the bar is too narrow for both, it SHALL show the total only, and narrower still, nothing; the search field SHALL keep its 60 skin pixel minimum.

#### Scenario: Selected and total
- **WHEN** a crate whose entries last 2:14:05 is shown 700 pixels wide, with entries lasting 7:30 selected
- **THEN** the box after the search field reads "7:30/2:14:05"

#### Scenario: Unknown durations
- **WHEN** a crate shown 700 pixels wide holds 40 previews, 3 of them not downloaded yet, and nothing is selected
- **THEN** the readout's total ends with `+`

#### Scenario: Narrow
- **WHEN** the collection crate, with tempos and styles, is shown 275 skin pixels wide
- **THEN** the readout shows the total only, or nothing, and the search field is at least 60 pixels wide with nothing overlapping

#### Scenario: The whole crate
- **WHEN** a crate of 120 entries lasting 8:02:11 is shown and the user types "parrish", which shows 9 entries
- **THEN** the readout's total still reads 8:02:11
