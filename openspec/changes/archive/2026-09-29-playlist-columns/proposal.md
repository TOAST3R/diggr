## Why

With the playlist able to grow wide (`player-layout`), cramming catalog number, artist, title, BPM, side, year and price into one string wastes the space and can't be sorted. Diggers want to line a crate up by BPM, label order or price before playing through it.

## What Changes

- **Columns when wide:** once the playlist is at least 480 points wide (at 1×), rows switch from the single-line name to columns:
  `# · Cat# · Artist · Title · BPM · Side · Year · For sale · Time`.
  Narrower, the single-line format from `dig-titles` stays.
- **Header row:** a header above the rows names each column.
  - Dragging a divider resizes the column to its left.
  - Right-clicking the header shows or hides columns (`#`, Title and Time can't be hidden).
  - Column widths and visibility are remembered.
- **Sort by any column:** clicking a header sorts the shown crate by that column. Clicking it again sorts the other way.
  - The sort **reorders the crate** (a one-off sort, as in Winamp): play order, saving and export follow the new order.
  - Entries without a value (no BPM yet, no catalog number) go last in both directions.
  - The sort is stable, so ties keep their current order.
  - The header shows an arrow on the last-sorted column until the order is changed by hand.
- **Also in OPT:** OPT ▸ Sort offers the same fields, so a narrow playlist can sort too.

## Capabilities

### New Capabilities
- `playlist-columns`: the column layout, the header, column widths and visibility, and sorting the crate by a field.

### Modified Capabilities
(none; the single-line format in `playlist` still applies below the width threshold)

## Impact

- `crates/ui/src/app.rs`: the playlist drawing gets a column mode and a header row; the OPT menu gets Sort.
- `crates/ui/src/playlist.rs`: `sort_by(field, direction)`, stable, with unknown values last; unit tests.
- `crates/ui/src/settings.rs`: column widths and visibility.
- **Depends on `dig-titles`** (the BPM field) and **`player-layout`** (the wide playlist). Implement after both.
