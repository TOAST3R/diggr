## MODIFIED Requirements

### Requirement: Filter panel
The style, artist, label, year and format filters SHALL be tabs of one panel (STYLE, ARTIST, LABEL, YEAR, FORMAT, showing only the filters the crate offers with at least two values). The STYLE, ARTIST, LABEL and FORMAT tabs SHALL each hold a list: each value with its number of records, the most first, a search field, a checkbox per value and Clear. The YEAR tab SHALL hold the year grid (see `year-filter`). In a seller crate with a copy in the cart, the panel SHALL also hold the CART switch. Each filter button in the bar SHALL open the panel on its tab; the FILTERS button SHALL open it on the first tab with a set filter, or on the first tab.

#### Scenario: Open on a tab
- **WHEN** the user clicks LABELS in the bar
- **THEN** the panel opens on its LABEL tab

#### Scenario: Switch tabs
- **WHEN** the panel is open on LABEL and the user clicks the ARTIST tab
- **THEN** the artist list is shown, with its own search field, and the picked labels are still picked

#### Scenario: Year tab
- **WHEN** the user clicks YEARS in the bar
- **THEN** the panel opens on its YEAR tab, showing the year grid

### Requirement: Filters from a row
Right-clicking an entry or record row SHALL offer, where the crate offers that filter (see `record-filters`, `style-filter`): "Only this artist", "Only this label" and "Only this style ▸" (one item per style of the record), each of which picks only that value in its filter, leaving the other filters as they are. Where the crate offers no label filter (a label crate, or a crate with fewer than two labels), it SHALL offer "Search ‹label›" and, when the record has one, "Search ‹catalogue number›", which set the search to that text and give the search field the keyboard.

#### Scenario: Only this label
- **WHEN** in the collection crate the user right-clicks a Lowtide Tapes record and chooses Only this label
- **THEN** only Lowtide Tapes records are shown, and LABELS is lit and reads "LABELS 1"

#### Scenario: Only this label in a seller crate
- **WHEN** in "Seller: decks.de", with records on many labels, the user right-clicks a record on Lowtide Tapes
- **THEN** the menu offers Only this label, and no Search Lowtide Tapes

#### Scenario: Search a catalogue number in a label crate
- **WHEN** in a label crate the user right-clicks a record with catalogue number AF014 and chooses Search AF014
- **THEN** the search reads "AF014", only matching records are shown, and the search field has the keyboard
