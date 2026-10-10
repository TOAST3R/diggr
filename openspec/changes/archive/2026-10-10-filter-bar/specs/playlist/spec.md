## MODIFIED Requirements

### Requirement: Playlist footer
The playlist footer SHALL hold, from left to right: a `+` button, a `≡` button, a gear button, and the "selected/total" time readout, right-aligned in its LCD box beside the resize grip (only the total when both don't fit in the box). The footer SHALL hold no filter control (see `filter-bar`). Everything SHALL fit at the classic 275 skin pixel width. The buttons SHALL open menus:
- `+`: Add files…, Add folder…, Import M3U…;
- `≡`: Select all, Select none, Invert selection, Remove selected, Clear crate, Sort ▸ (every column's field), Group by record (a checkbox, see `record-view`), Export M3U…;
- the gear: the Options menu (see the player window's Options menu), with the same items as the right-click.

Each item SHALL act as the same item did in the footer's earlier menus. No app setting SHALL be in the `+` and `≡` menus.

#### Scenario: Three buttons
- **WHEN** the playlist is shown at 275 skin pixels wide
- **THEN** the footer shows `+`, `≡`, the gear and the time readout, and no ADD, REM, SEL, MISC or OPT button

#### Scenario: No filters in the footer
- **WHEN** the collection crate, with tempos and styles, is shown 700 pixels wide
- **THEN** the footer shows `+`, `≡`, the gear and the time readout only, and the BPM control and style chips are in the filter bar

#### Scenario: Options from the footer
- **WHEN** the user clicks the gear and chooses Discogs…
- **THEN** the Discogs dialog opens, as from the right-click Options menu

#### Scenario: Add
- **WHEN** the user clicks `+` and chooses Add folder…
- **THEN** the folder dialog opens, as ADD ▸ Add folder… did

#### Scenario: Sort from the footer
- **WHEN** the user clicks `≡` and chooses Sort ▸ BPM
- **THEN** the crate is sorted by BPM, as by clicking the BPM column header

#### Scenario: Group from the menu
- **WHEN** the user clicks `≡` and ticks Group by record
- **THEN** the shown crate is grouped, as with Shift+G
