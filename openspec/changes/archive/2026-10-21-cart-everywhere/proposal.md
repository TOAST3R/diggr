## Why

The + CART pill shows only on a Discogs copy for sale (seller crates) and on entries sent from Juno, Bleep, Beatport or Traxsource. Everywhere else (wantlist, label crates, pasted pages, Bandcamp) a user who likes a record has no button to buy it, and early users read the missing pill as "can't be bought".

## What Changes

- **+ CART on every dig entry and record row**: any entry with a record behind it (Discogs, Bandcamp or a store) shows the pill. Local files don't.
- **One known store: act at once.** A Discogs copy for sale goes in the Discogs cart as today; a Juno or Bleep item goes in that basket through the extension; Discogs without a known copy opens the release's marketplace; Bandcamp, Beatport and Traxsource open the record's page.
- **No known store, or several: a "Buy" chooser.** It lists the stores where the app knows the record (with price when known) as buttons, and every other integrated store as a "Search on ‹store› ↗" link for the record's artist and title.
- A record row with several copies for sale opens the record so the user picks a copy, instead of showing no pill.

## Capabilities

### New Capabilities
- `buy-chooser`: which stores a record is known on, the chooser dialog, its search links.

### Modified Capabilities
- `discogs-cart`: Add to cart: the pill is no longer limited to copy rows and one-copy record rows.
- `store-carts`: Store cart pill: entries without a store item get the pill too, through the chooser.

## Impact

- **`crates/ui`:** `entry_badges` / `Badges` / `Pill` callers in `app.rs` (record rows, track rows), `stores.rs` (`store_item`, `store_pill_of`), a new chooser dialog beside the existing dig dialogs, reusing `dig_asking` for focus.
- **`crates/dig`:** `StoreMemory::product_of(release)` already links Juno and Bleep products to a release; a small function building each store's search URL.
- **README:** the cart pill rows in the help and README, test count.
