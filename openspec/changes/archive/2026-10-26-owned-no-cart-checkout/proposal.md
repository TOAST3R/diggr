## Why

Every row with a record behind it offers + CART, even a record the user already owns: every row of the collection crate, and OWNED rows in label, artist and seller crates, where + CART sits right after the OWNED badge. One click puts a second copy in the cart by mistake. Once something is in a cart, getting to pay takes a detour: "Open cart on discogs.com" is in seller crates' menus only, and nothing leads to the Juno or Bleep basket.

## What Changes

- **No + CART on what you own.** A record the user owns (this pressing or another pressing of its master), every entry of the collection crate, and every entry sent from a collection page show no + CART pill and no Buy pill. In a seller crate, the copy rows of an owned record show no pill either.
- **In the cart and owned:** the IN CART pill stays, REMOVE on hover still takes it out, and its tooltip warns first: "You already own another pressing (AF001R, 2019). Remove €9.00 · VG+ / VG from your cart". Nothing is ever taken out of a cart automatically.
- **» checkout pill:** every row in a cart (IN CART pill or CART badge) shows a narrow `»` pill after it. A click opens that store's cart in the default browser: `https://www.discogs.com/sell/cart` for Discogs, the Juno or Bleep basket page for those stores. Its tooltip names the store ("Check out on discogs.com"). Diggr still never checks out or pays: the user does that on the store's page.
- **Open cart in every crate:** the list menu offers "Open cart on ‹store›" in every crate, for each store whose cart holds something, not only in seller crates.
- **One + CART per record:** in the list, a record's tracks share one pill, centred beside them between two thin lines, instead of one per track.
- Without a Discogs token the collection isn't known, so + CART stays outside the collection crate, as today.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `discogs-cart`: a new requirement that owned records get no + CART (and that an owned record in the cart keeps IN CART with a warning), and a new requirement for the `»` checkout pill.
- `store-carts`: the `»` pill on store items in a basket opens that store's basket page.
- `buy-chooser`: an owned record shows no Buy pill.
- `playlist`: the list menu's "Open cart on discogs.com" is offered in every crate while the cart holds something, with Juno and Bleep beside it.

## Impact

- **`crates/ui/src/app.rs`:** the pill choice on record rows, track rows and copy rows takes "owned" into account (asked of the collection directly, not through the OWNED badge, which the collection crate turns off). `Badges` and `entry_badges` draw the `»` pill, and its click is registered as the cart pill's is. List menu items.
- **`crates/ui/src/app/stores.rs`:** the in-cart tooltip gets the owned warning. A cart page per store.
- **`crates/ui/src/app/sellers.rs`:** `OpenCart` takes the store.
- **`crates/dig/src/stores.rs` (or `carts.rs`):** each reachable store's basket URL.
- **Tests:** headless tests in `store_cart_tests.rs` and `seller_tests.rs` (owned rows have no pill, owned + in cart keeps IN CART, the `»` pill opens the right page, the menu item in a label crate).
- **Docs:** CHANGELOG; README only if the docs list the badges.
- No change to playback, analysis, the extension or any request to Discogs.
