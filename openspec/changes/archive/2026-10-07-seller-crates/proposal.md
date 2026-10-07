## Why

A digger buys from a handful of shops again and again, but the app only digs pages (labels, artists, wantlists), never a seller's stock. Today you can't listen to what decks.de or Wavelength_Records has for sale, compare copies of the same record by price and condition, and put the one you want in your Discogs cart without leaving the player. The Discogs API can do all of this: a seller's inventory is public, and the cart and purchase history are reachable with the user's token (tested against a real account; see design).

## What Changes

- **Top Sellers.** A new group under DISCOGS in the crate sidebar, TOP SELLERS, which can be folded (the fold is remembered). It holds one crate per seller, "Seller: ‹name›".
  - **First list.** The first time a token is set, or on the first launch with a token after this change, the app reads the user's last 100 purchases and puts the 10 sellers bought from most often in the list. Ties go to the most recent order, and sellers with nothing for sale are skipped. This happens once; the list is the user's after that.
  - **Add seller….** Right-click TOP SELLERS › Add seller… opens a modal where the user pastes a seller's page or name. It shows the seller and how many copies they have for sale before adding.
  - **Remove seller.** Right-click a seller › Remove seller removes it from the list and deletes its crate (after confirming).
- **Double-click to dig.** Clicking a seller crate only shows it and never contacts Discogs. Double-clicking it connects:
  - **never dug:** one request to count the copies for sale. With 1,000 or fewer, a "Dig" modal confirms. With more, a "Narrow down" modal asks for criteria (search text, newest N, format, price range, minimum condition) and shows the resulting count before digging. Discogs serves at most the first 10,000 listings of another user's inventory, and the modal says so when a seller has more;
  - **dug more than 24 h ago:** refreshes it with its saved criteria;
  - **dug within 24 h:** shows it, with no request.
  
  Right-click › Refresh seller refreshes it whenever it was last done. A refresh that would take the crate over 1,000 copies asks to narrow down instead.
- **One row per copy.** A seller crate is grouped by record (`record-view`). Inside an open record, each copy for sale is its own row (price, media and sleeve condition, ships from), above the record's tracks, which appear once. The record row's second line gives the number of copies and their price range. The copy rows are a new kind of playlist row.
- **Refresh results.** New copies come in. A copy no longer for sale stays, dimmed and marked **SOLD**. Changed prices are updated. The main window summarises ("decks.de: 14 new, 6 sold, 3 cheaper").
- **Add to cart.** Right-clicking a copy offers Add to cart, which adds it to the user's Discogs cart through the API. Right-clicking a track of a record with several copies offers Add to cart ▸ one item per copy. When the copy is in the cart, the item reads Remove from cart. If the cart call fails for any reason other than the copy being sold, the listing opens on discogs.com instead. A copy that is no longer for sale gets SOLD.
- **CART badge.** A copy in the user's Discogs cart shows a **CART** badge, in a colour of its own. Its record row shows it too, and so does every wantlist or collection entry whose release has a copy in the cart. The mark follows the real Discogs cart, read lazily, so items added on discogs.com count.
- **Cart filter.** In a seller crate with copies in the cart, the playlist footer shows **CART n · subtotal**, a switch that shows and plays only records with a copy in the cart. ☰ offers Show cart only and Open cart on discogs.com.
- **Extension.** On a Discogs seller page (`/seller/‹name›/profile`, `/user/‹name›`), the extension's page button offers Add seller to ‹App›. It adds the seller to TOP SELLERS and shows its crate. If the seller is already there, it refreshes the crate instead. The app then asks to be brought to the front.
- **Out of scope:** "Dig this seller" from a copy in another crate; sellers from the friends list; checking out from the app; offers to sellers; genre and style filters before expanding (the API ignores them).

## Capabilities

### New Capabilities
- `seller-crates`: the TOP SELLERS group, the first list from purchases, Add seller and Remove seller, double-click to dig with the 1,000-copy limit and the Narrow down modal, refreshing (lazy and on demand), copies as rows, SOLD.
- `discogs-cart`: Add to cart and Remove from cart on a copy, the CART badge from the real Discogs cart, the cart filter in seller crates, and opening the cart on discogs.com.

### Modified Capabilities
- `crates`: "Crate sidebar" gains the foldable TOP SELLERS group under DISCOGS, with double-click and the seller menu.
- `record-view`: "Record row" gives a seller crate's records the copy count and price range, and an open record shows copy rows above its tracks.
- `record-filters`: "Filter controls" and "Filters combine" include the CART switch.
- `discogs-intake`: "Supported pages" accepts a seller's page (`/seller/‹name›/profile`, `/user/‹name›` without a further path), which adds the seller instead of sending tracks.
- `chrome-extension`: "Page button" shows on seller pages, with Add seller to ‹App›.
- `browser-bridge`: "Discogs references only" accepts an add-seller request, and the app asks to come to the front after it.

## Impact

- **`crates/dig`:**
  - `discogs/url.rs`: `PageKind::Seller`;
  - `discogs/transport.rs`: POST with a JSON body, which the cart needs (today POST sends an empty body);
  - new `discogs/seller.rs`: inventory pages and their filters, the purchases-based first list;
  - new `discogs/cart.rs`: read, add, remove, parse per-item errors;
  - `bridge/`: the add-seller request.
- **`crates/ui`:**
  - `crates.rs`: a seller crate kind, its saved criteria and last dig time;
  - `app.rs`: the sidebar's TOP SELLERS group, double-click, menus;
  - `app/digging.rs`: the modals, refresh, cart actions and badge state;
  - `playlist.rs`: copy rows and the copy on entries;
  - `format.rs`: badges and tooltips;
  - the footer's CART switch.
- **`extensions/chrome`:** seller pages in `pages.js`, and the Add seller action.
- **README:** Top Sellers, double-click, the cart, the new cache files, the test count. Help panel.
- **No new dependency.** All of it uses Discogs' rate limit and cache, nothing touches the playback path, and the cart and purchases endpoints are undocumented (see design: fallbacks).
