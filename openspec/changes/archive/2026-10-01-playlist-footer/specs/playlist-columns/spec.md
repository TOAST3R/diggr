## MODIFIED Requirements

### Requirement: Sort the crate by a column
Clicking a column header (other than the number column), or choosing that field in the playlist's ≡ ▸ Sort, SHALL reorder the shown crate by that field, ascending, and a second click SHALL reorder it descending. The rules:
- The sort SHALL be stable.
- Entries without a value SHALL go last in both directions.
- Catalog numbers and sides SHALL sort in natural order (LT-2 before LT-10, A2 before A10).
- For sale SHALL sort by lowest price, with "none for sale" after priced entries and before entries with no snapshot.

The new order SHALL be the crate's order for playback, saving and export. The playing entry SHALL keep playing, and the next track SHALL follow the new order. The header SHALL mark the sorted column and direction until the crate is reordered by hand. Sorting a 1,000-entry crate SHALL take less than 16 ms and SHALL NOT affect playback.

#### Scenario: Sort by BPM
- **WHEN** a crate holds entries at 128, 122, unknown and 140 BPM and the user clicks the BPM header
- **THEN** the order is 122, 128, 140, unknown, and the header marks BPM ascending

#### Scenario: Descending keeps unknowns last
- **WHEN** the user clicks the BPM header again
- **THEN** the order is 140, 128, 122, unknown

#### Scenario: Natural catalog order
- **WHEN** entries have catalog numbers LT-10, LT-2 and LT-1 and the user sorts by Cat#
- **THEN** the order is LT-1, LT-2, LT-10

#### Scenario: Next follows the sort
- **WHEN** entry "A" plays and a sort moves entry "B" directly after it
- **THEN** "A" keeps playing without interruption, and B plays next

#### Scenario: Manual reorder clears the mark
- **WHEN** the crate is sorted by Year and the user drags an entry to another position
- **THEN** the header no longer marks Year as sorted

#### Scenario: Big crate
- **WHEN** a 1,000-entry crate is sorted by Artist during playback
- **THEN** the sort takes less than 16 ms and playback has zero underruns
