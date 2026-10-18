## Why

Diggr already turns Juno, Beatport, Traxsource (and, with `bleep`, Bleep) pages into crates of previews, and for Discogs it closes the loop: a copy you like goes into your real cart from its row, with + CART / IN CART. For the stores, a track you decide to buy still means finding it again in the browser. Buying where you dig should take one click on the row, the same way as Discogs.

## What Changes

- **A cart pill on store entries:** an entry sent from Juno, Beatport, Traxsource or Bleep shows **+ CART** on its row, drawn as the Discogs pill is. Clicking it puts the item in your real basket on that store; the pill then reads **IN CART** (REMOVE in red on hover, which takes it out again).
  - **What is added:** on Beatport and Traxsource, the track; on Juno, the record's product (its format is part of the product); on Bleep, the record in a format chosen from a small menu when it is sold in more than one.
- **The browser does it:** the app has no login to these stores, so the paired extension performs each cart action in the user's logged-in browser session, using the store's own basket call. The app asks; the extension, which already talks to the app, picks up the request and reports back.
- **Always a way through:** when the cart can't be reached (no extension running, not logged in, the store refused, sold out, no answer within a few seconds), the item's page opens on the store instead, and the main window says so.
- **CART badge:** an item in a store basket shows CART on every entry of it in any crate, with the store and price in its tooltip. Store baskets are read by the extension when it can, best effort, and kept in the cache so badges show at launch.
- **Never** checks out, pays, or empties a basket; the user always pays on the store.

## Capabilities

### New Capabilities
- `store-carts`: the cart pill on store entries, what is added per store, the outcomes and fallback, the CART badge for store baskets, and what the app keeps.

### Modified Capabilities
- `chrome-extension`: the extension may now call the four stores' basket endpoints, only to perform a cart action the user started in the app or to read the basket; still no reading of other pages and no other servers.
- `browser-bridge`: a paired extension can fetch pending cart actions and report their results and the baskets it read.
- `bleep-intake`: each format may carry its Bleep product id, read from its buy button, so it can be put in the basket.

(Builds on `bleep`, which also modifies `chrome-extension` and `browser-bridge`; `mix-tracklists` and `artist-follow` modify them too. Whichever is archived later is rebased on the earlier ones.)

## Impact

- **`crates/dig`:** a store cart model (items per store, cached as `store-carts.ron`), the queue of pending cart actions with their timeouts, and the bridge's new endpoints.
- **`extensions/chrome`:** per-store basket calls (add, remove, read) made from the store's origin in the user's session; a wait for pending actions while the app runs; `host_permissions` unchanged beyond `bleep`'s.
- **`crates/ui`:** the pill on store entries, Bleep's format menu, the CART badge and tooltips for store baskets, the fallback opening the store page.
- **README:** "Buying from the stores", the extension's new store calls, the cache file, test areas and count.
- **No change** to `audio`, `analysis` or `platform`.
