## MODIFIED Requirements

### Requirement: Style filter control
When the shown crate is the user's Discogs wantlist crate or collection crate, and its entries have at least two different styles, the playlist footer SHALL offer a style filter after the BPM filter control and before the time readout, clear of the time readout's box, laid out with the artist and label filters as `record-filters` says. An entry's styles SHALL be the record's Discogs styles (its genres when it has none). Styles SHALL be listed by the number of records that have them, most first, then by name, counted over the whole crate. The control SHALL be either:
- one chip per style, in the skin's font, lit when selected and dimmed otherwise, when there are at most 20 styles and the chips fit together with the ARTISTS and LABELS buttons; or
- a "STYLES" button showing how many styles are selected, which opens a list with a search field, one checkbox per style with its number of records, and Clear, when the buttons fit; or
- nothing in the footer, when they don't fit.

Clicking a chip or a checkbox SHALL select or unselect that style. Typing in the search field SHALL narrow the list to styles containing the text, ignoring case. Clear, a double-click on the control, and ☰ ▸ Show all records SHALL unselect every style. ☰ ▸ Filter by style… SHALL open the style list at any width. Other crates SHALL NOT show the control.

#### Scenario: Chips
- **WHEN** the collection crate is shown in a 700-pixel-wide playlist, and its records have 6 styles
- **THEN** the footer shows 6 chips after the BPM control, the style with the most records first

#### Scenario: Many styles
- **WHEN** the collection crate's records have 143 styles and the playlist is 700 pixels wide
- **THEN** the footer shows a "STYLES" button, and clicking it opens a list of 143 styles with their record counts and a search field

#### Scenario: Narrow playlist
- **WHEN** the wantlist crate, whose records have tempos and 5 styles, is shown in a 275-pixel-wide playlist
- **THEN** the footer shows no style control, nothing overlaps the time readout's box, and ☰ ▸ Filter by style… opens the style list

#### Scenario: Search
- **WHEN** the style list is open and the user types "house"
- **THEN** only styles containing "house" are listed, such as "Deep House" and "Tech House"

#### Scenario: Other crates
- **WHEN** a dig crate whose records have 8 styles is shown
- **THEN** the footer shows no style filter
