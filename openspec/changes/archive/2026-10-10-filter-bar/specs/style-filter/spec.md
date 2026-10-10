## MODIFIED Requirements

### Requirement: Style filter control
When the shown crate is the user's Discogs wantlist crate or collection crate, and its entries have at least two different styles, the filter bar (see `filter-bar`) SHALL offer a style filter after the BPM filter control, laid out with the artist and label filters as `record-filters` says. An entry's styles SHALL be the record's Discogs styles (its genres when it has none). Styles SHALL be listed by the number of records that have them, most first, then by name, counted over the whole crate. The control SHALL be either:
- one chip per style, in the skin's font, lit when selected and dimmed otherwise, when there are at most 20 styles and the chips fit together with the ARTISTS and LABELS buttons; or
- a "STYLES" button showing how many styles are selected, which opens the filter panel on its STYLE tab (a search field, one checkbox per style with its number of records, and Clear), when the buttons fit; or
- the FILTERS button (see `record-filters` "Filter controls"), when they don't fit.

Clicking a chip or a checkbox SHALL select or unselect that style. Typing in the tab's search field SHALL narrow the list to styles containing the text, ignoring case. Clear, a double-click on the control, and the bar's × SHALL unselect every style. Other crates SHALL NOT show the control.

#### Scenario: Chips
- **WHEN** the collection crate is shown in a 700-pixel-wide playlist, and its records have 6 styles
- **THEN** the filter bar shows 6 chips after the BPM control, the style with the most records first

#### Scenario: Many styles
- **WHEN** the collection crate's records have 143 styles and the playlist is 700 pixels wide
- **THEN** the filter bar shows a "STYLES" button, and clicking it opens the filter panel on its STYLE tab, listing 143 styles with their record counts and a search field

#### Scenario: Narrow playlist
- **WHEN** the wantlist crate, whose records have tempos and 5 styles, is shown in a 275-pixel-wide playlist
- **THEN** the filter bar shows the FILTERS button and no style chips, and FILTERS opens the panel with a STYLE tab

#### Scenario: Search
- **WHEN** the STYLE tab is open and the user types "house"
- **THEN** only styles containing "house" are listed, such as "Deep House" and "Tech House"

#### Scenario: Other crates
- **WHEN** a dig crate whose records have 8 styles is shown
- **THEN** the filter bar shows no style filter
