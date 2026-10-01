## ADDED Requirements

### Requirement: Playlist footer
The playlist footer SHALL hold, from left to right: a `+` button, a `≡` button, the BPM filter control (when the crate has two different known tempos), and the "selected/total" time readout, right-aligned in its LCD box beside the resize grip (only the total when both don't fit in the box). Everything SHALL fit at the classic 275 skin pixel width. The buttons SHALL open menus:
- `+`: Add files…, Add folder…, Import M3U…;
- `≡`: Select all, Select none, Invert selection, Remove selected, Clear crate, Sort ▸ (every column's field), Show all tempos (only while a BPM range is set), Export M3U….

Each item SHALL act as the same item did in the footer's earlier menus. No app setting SHALL be in these menus.

#### Scenario: Two buttons
- **WHEN** the playlist is shown at 275 skin pixels wide
- **THEN** the footer shows `+`, `≡` and the time readout, and no ADD, REM, SEL, MISC or OPT button

#### Scenario: Add
- **WHEN** the user clicks `+` and chooses Add folder…
- **THEN** the folder dialog opens, as ADD ▸ Add folder… did

#### Scenario: Sort from the footer
- **WHEN** the user clicks `≡` and chooses Sort ▸ BPM
- **THEN** the crate is sorted by BPM, as by clicking the BPM column header
