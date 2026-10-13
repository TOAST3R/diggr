## MODIFIED Requirements

### Requirement: Playlist footer
The playlist footer SHALL hold, from left to right: a `+` button, a gear button, and the "selected/total" time readout, right-aligned in its LCD box beside the resize grip (only the total when both don't fit in the box). The footer SHALL hold no filter control (see `filter-bar`) and no `≡` button. Everything SHALL fit at the classic 275 skin pixel width. The buttons SHALL open menus:
- `+`: Add files…, Add folder…, Import M3U…, Export M3U…;
- the gear: the Options menu (see the player window's Options menu), with the same items as the right-click.

Each item SHALL act as the same item did in the footer's earlier menus. No app setting SHALL be in the `+` menu. The crate's own actions SHALL be in the list menu (see "List menu").

#### Scenario: Three buttons
- **WHEN** the playlist is shown at 275 skin pixels wide
- **THEN** the footer shows `+`, the gear and the time readout, and no `≡`, ADD, REM, SEL, MISC or OPT button

#### Scenario: No filters in the footer
- **WHEN** the collection crate, with tempos and styles, is shown 700 pixels wide
- **THEN** the footer shows `+`, the gear and the time readout only, and the BPM control and style chips are in the filter bar

#### Scenario: Options from the footer
- **WHEN** the user clicks the gear and chooses Discogs…
- **THEN** the Discogs dialog opens, as from the right-click Options menu

#### Scenario: Add
- **WHEN** the user clicks `+` and chooses Add folder…
- **THEN** the folder dialog opens, as ADD ▸ Add folder… did

#### Scenario: M3U together
- **WHEN** the user clicks `+`
- **THEN** the menu holds Import M3U… and Export M3U…, and Export M3U… writes the shown crate as ≡ ▸ Export M3U… did

## ADDED Requirements

### Requirement: List menu
The shown crate's actions SHALL be reachable from the playlist list with no footer button: a right-click (or Control-click on macOS) on a row SHALL open the row's menu ending with a "Crate ▸" submenu, and a right-click on the list's empty space (below the last row, or anywhere in an empty crate's list) SHALL open the same items directly. The items SHALL be:
- Select all, Select none, Invert selection;
- Remove selected and Clear crate, in crates the user can edit (not the Discogs wantlist, collection and label crates);
- Sort ▸ (every column's field), Group by record (a checkbox, see `record-view`);
- Open cart on discogs.com, in seller crates (see `discogs-cart`).

Each item SHALL act as the same item did in the footer's `≡` menu. A separator SHALL be drawn only between groups that show an item.

#### Scenario: Sort while grouped
- **WHEN** the collection crate is grouped, so no column header is shown, and the user right-clicks a record row and chooses Crate ▸ Sort ▸ BPM
- **THEN** the crate is sorted by BPM, as by clicking the BPM column header

#### Scenario: Empty space
- **WHEN** a crate of 3 entries is shown and the user right-clicks below the last row and chooses Invert selection
- **THEN** the selection is inverted, and no row's menu opened

#### Scenario: Group from the menu
- **WHEN** the user right-clicks a row and ticks Crate ▸ Group by record
- **THEN** the shown crate is grouped, as with ▤

#### Scenario: No empty band
- **WHEN** the collection crate is shown and the user right-clicks its empty space
- **THEN** the menu shows no Remove selected or Clear crate, and no two separators in a row
