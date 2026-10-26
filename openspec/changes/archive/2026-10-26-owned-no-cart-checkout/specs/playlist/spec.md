## MODIFIED Requirements

### Requirement: List menu
The shown crate's actions SHALL be reachable from the playlist list with no footer button: a right-click (or Control-click on macOS) on a row SHALL open the row's menu ending with a "Crate ▸" submenu, and a right-click on the list's empty space (below the last row, or anywhere in an empty crate's list) SHALL open the same items directly. The items SHALL be:
- Select all, Select none, Invert selection;
- Remove selected and Clear crate, in crates the user can edit (not the Discogs wantlist, collection and label crates);
- Sort ▸ (every column's field), Group by record (a checkbox, see `record-view`);
- Open cart on discogs.com, in seller crates and in every crate while the Discogs cart holds a copy (see `discogs-cart`); Open cart on juno.co.uk and Open cart on bleep.com, in every crate while that basket holds an item (see `store-carts`).

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

#### Scenario: Open cart from a label crate
- **WHEN** a label crate is shown, the Discogs cart holds 2 copies and the Juno basket is empty
- **THEN** the list menu offers Open cart on discogs.com and not Open cart on juno.co.uk, and choosing it opens `https://www.discogs.com/sell/cart`

#### Scenario: Nothing in any cart
- **WHEN** a label crate is shown and every cart is empty
- **THEN** the list menu offers no Open cart item
