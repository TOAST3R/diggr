# style-filter Specification

## Purpose
Narrows the Discogs wantlist and collection crates to the records of chosen styles, as diggers look for them: footer chips (or a searchable list when there are many), any of the chosen styles, on top of the BPM filter, remembered per crate, with play following what's shown.
## Requirements
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

### Requirement: What the style filter shows
With no style selected, the style filter SHALL show every entry. With styles selected, an entry SHALL be shown when it has any of them. An entry with no style SHALL be shown only when no style is selected. An entry SHALL be shown only when both the BPM filter and the style filter show it. Everything the playlist-filters capability says of entries shown by the BPM filter SHALL hold for entries shown by both filters: what plays next, shuffle, the pre-warmed track, the previews downloaded ahead, keyboard navigation, the title bar's shown/all count, record rows and "k of N tracks". While a style is selected, the title bar SHALL show the shown and total counts. Changing the selection SHALL update the list within one frame (16 ms) for a crate of 5,000 entries, SHALL NOT interrupt playback, and SHALL cause zero underruns.

#### Scenario: Any of the styles
- **WHEN** records A (Deep House), B (Minimal, Techno) and C (Electro) are in the collection crate, and the user selects Deep House and Minimal
- **THEN** A and B are shown and C is hidden

#### Scenario: With the BPM filter
- **WHEN** the BPM range is 130–140 and Minimal is selected
- **THEN** only Minimal entries at 130 to 140 BPM are shown, and the title bar shows the shown and total counts

#### Scenario: Next under the style filter
- **WHEN** Deep House is selected and a Deep House track ends, followed in crate order by two Electro records and then a Deep House record
- **THEN** the Deep House record's first track plays next

#### Scenario: Big crate
- **WHEN** the user selects a style in a grouped collection crate of 5,000 entries while a track plays
- **THEN** the list updates within 16 ms, and playback has zero underruns

### Requirement: Style filter is remembered
The selected styles SHALL be remembered per crate, across restarts. A selected style that no entry of the crate has any more SHALL stop filtering. When no selected style remains in the crate, every entry SHALL be shown.

#### Scenario: After a restart
- **WHEN** Deep House is selected in the wantlist crate and the app is restarted
- **THEN** the wantlist crate shows only its Deep House records

#### Scenario: Style gone
- **WHEN** Electro is the only selected style, and the only Electro record leaves the wantlist crate
- **THEN** every entry of the crate is shown

