## Why

The playlist footer's `≡` menu is the crate's menu, but it is a second, hidden place for things the list already does (Cmd+A, Delete, column headers, ▤). In crates you can't edit it shows stray separators around nothing. Export M3U sits apart from Import M3U, under a different button. The footer is cleaner with `+` and the gear only, and the crate's actions belong where the crate is: on the list.

## What Changes

- **The `≡` button goes** from the playlist footer, which then holds `+`, the gear and the time readout. The skin drops its sprite.
- **Export M3U… moves to `+`**, under Import M3U…, so both M3U items are together. `+`'s hover text becomes "Add · M3U".
- **No feature is lost**: the other `≡` items move to the list's right-click.
  - A right-click on a row adds a **Crate ▸** submenu at the end of the row's menu.
  - A right-click on the list's empty space (below the last row, or in an empty crate) opens the same items directly.
  - The items are Select all, Select none, Invert selection, Remove selected and Clear crate (crates you can edit), Sort ▸, Group by record, and Open cart on discogs.com (seller crates).
- Separators only appear between groups that have items, so read-only crates show no empty band.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `playlist`: "Playlist footer" holds `+` and the gear; the crate's items move to a new "List menu" requirement on the list's right-click.
- `playlist-columns`: "Sort the crate by a column" names the list's Crate ▸ Sort instead of ≡ ▸ Sort.
- `record-view`: Group by record is in the list menu instead of the ≡ menu.
- `discogs-cart`: Open cart on discogs.com is in the seller crate's list menu.

## Impact

- `crates/ui/src/app.rs`: the footer drops `pl_menu`; the `+` popup gains Export M3U…; the `≡` popup's items become a `crate_items` method, used by the row menu (`entry_menu`, as Crate ▸) and by a right-click on the list's empty space.
- `crates/ui/src/skin/` (`generate.rs`, `mod.rs`), `layout.rs`: drop `pl_menu`; regenerate the skin with `skin-gen`.
- Headless tests that click `pl_menu` (`app.rs`, `record_tests.rs`, `seller_tests.rs`) open the list's right-click instead.
- README: the footer, Sort, Group by record, M3U and Open cart sentences.
- No new dependency, nothing on the playback path.
