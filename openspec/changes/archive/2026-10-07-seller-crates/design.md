## Context

- **Discogs API surface.** Probed with the user's token on 2026-10-07. Only the inventory is documented; the rest works but Discogs doesn't publish it.

  | Call | Documented | Behaviour seen |
  |---|---|---|
  | `GET /users/{seller}/inventory?per_page=100&page=N` | yes | Listings with `id`, `price{value,currency}`, `condition`, `sleeve_condition`, `ships_from`, `posted`, `status`, `uri` (`/sell/item/{id}`), and `release{id, artist, title, format, year, catalog_number, thumbnail}`. `pagination.items` gives the total. **Pages above 100 return 403** ("Pagination above 100 disabled for inventories besides your own"), so at most 10,000 listings. |
  | the same with `q=…` | no | Applied: decks.de 40,728 → 8,389 for `q=techno`. |
  | the same with `sort=listed\|price…&sort_order=asc\|desc` | yes | Applied. |
  | the same with `genre=`, `style=`, `format=` | — | **Ignored** (same total). |
  | `GET /purchases?per_page=100` | no | The user's orders as buyer (124 orders, 89 sellers): `seller{id,username}`, `created`, `items[{release_id,…}]`, `total` (with `converted` to the user's currency), `seller_ships_from`, `status`. `GET /marketplace/orders` returns orders the user *sold* instead. |
  | `GET /cart` | no | An array of per-seller carts: `seller`, `cart_items[{item_id, price, media_condition, sleeve_condition, item_url, release{id,…}}]`, `subtotal`, `shipping_methods`, `shipping_method_fees`, `payment_methods`, `tax`. |
  | `POST /cart/items` `{"item_ids":[…]}` | no | 201 "Cart merge complete", with `errors[{item_id, status, message}]` per item: 422 "already in your cart", 404 "not for sale". `POST /cart/item/{id}` does **not** add (404 "Invalid item ID"). |
  | `DELETE /cart/item/{id}` | no | 204, removed. |
  | `GET\|DELETE /cart/seller/{seller_id}` | no | One seller's cart / empty it. Not used. |
  | `DELETE /cart` | no | Allowed by OPTIONS, probably empties the whole cart. **Never called.** |
  | `GET /users/{me}/friends` | no | Works. Dropped in exploration: a friends list doesn't reliably mean "sellers I buy from". |

- **Today's pipeline.** A sent page becomes "listed" placeholders, then `intake::expand_next` fetches each record's details nearest the playhead first, cached, within the 60 req/min limiter (`discogs/ratelimit.rs`). Records listed twice come in once (`fix-repeats-and-silent-twins`). Records without clips become searchable tracks (`search-previews`).
- **Transport.** `transport.rs` sends POST and PUT with an empty body. The cart needs a JSON body.
- **Sidebar.** `crates` "Crate sidebar" pins the wantlist and collection crates under DISCOGS (`CrateInfo.wantlist` / `.collection`). There is no group or folding today.
- **Entries.** An entry is a track (`playlist::Entry` with `Origin`). Grouped view (`record-view`) builds record rows from `album_key()`. There is no notion of a copy for sale; `Origin.for_sale` is a release-level snapshot (count, lowest price).

## Goals / Non-Goals

**Goals:**
- Listen through a seller's stock with the same instant feel as any crate: a click shows it, a double-click is the only thing that reaches Discogs.
- Each copy for sale is visible and actionable on its own (price, condition), while the music is heard once per record.
- The cart badge and filter tell the truth about the real Discogs cart, not only about what the app did.
- Undocumented calls fail soft: the user can always fall back to discogs.com.

**Non-Goals:**
- Checkout, offers, shipping choices; "Dig this seller" from a copy in another crate; friends; genre and style filters before expansion; reading more than the 10,000 listings Discogs serves.

## Decisions

### 1. Sellers live in `sellers.ron`, crates stay crates

`<config>/dig/sellers.ron` holds:
- `seeded: bool`;
- the list in order: `username`, `seller_id`, `crate` id, `source` (`Purchases` or `Added`), `criteria`, `last_dug` (epoch seconds), and `total` (the count for sale at the last dig).

`CrateInfo` gains `seller: Option<String>`, so the sidebar can place the crate without loading `sellers.ron` first. The crate's entries are saved like any crate.

*Why separate:* the list's order, the criteria and the last dig time are about the seller, not the tracks. Removing a seller deletes its crate and its row. Renaming the crate doesn't detach it, the same as the wantlist and collection crates are recognised after a rename.

### 2. The first list, once

Seeding runs when a token is saved (`dig_token_checked`), or at startup with a token when `seeded` is false:
1. read one page of `/purchases?sort=created&sort_order=desc&per_page=100` (Discogs applies that sort, checked on 2026-10-07), and keep the 100 most recent by `created`, sorting locally as well in case Discogs stops applying it;
2. count orders per seller, breaking ties by the most recent order;
3. walk that ranking, asking each seller's inventory count (`per_page=1`), and keep the first 10 with at least one copy for sale;
4. create their crates, empty and not dug, and set `seeded`.

That's about 2 + 12 requests, behind other work. With no purchases, the group starts empty and says "Add seller…". A failure leaves `seeded` false, so it's tried again at the next launch.

### 3. Click shows, double-click digs

Sidebar double-click on a seller crate runs the state machine below. A single click only shows the crate, from disk.

```
            ┌───────────── never dug ──────────────┐
            │  GET inventory?per_page=1 (+ q)      │
            │  total ≤ 1,000 → Dig modal           │
            │  total > 1,000 → Narrow down modal   │
  dbl-click ┤                                      ├─→ dig with criteria → last_dug = now
            │  dug > 24 h ago → refresh            │
            │  dug ≤ 24 h ago → nothing            │
            └──────────────────────────────────────┘
```

**Narrow down.** The criteria applied by Discogs are `q` and newest-N (`sort=listed desc`, stop after N). Their count is known at once from `pagination.items` (newest-N is `min(N, items)`).

The local criteria are format group (the existing Vinyl/File/CD/Cassette/Other grouping), price range, minimum media condition and ships-from. They need the listings themselves. The modal reads pages in the background (≤ 100 requests, with a progress bar and Cancel) and recomputes the count on every change, within a frame. **Dig** is enabled when the count is ≤ 1,000.

The listings read for the modal are kept, so digging right after costs no second listing pass.

*Why 1,000:* listing pages are cheap (10 for 1,000), but every new record costs a release request at 60/min, and a crate of thousands of copies is unusable to browse. The limit is a constant, not a setting.

### 4. Copies are data on the crate, rendered as rows

A seller crate's playlist gains `copies: Vec<Copy>`, where a copy is `listing`, `release`, `price{value,currency}`, `condition`, `sleeve`, `ships_from`, `posted`, `uri`, `sold`. It's saved with the crate, as an optional field.

Tracks come from the release, through the usual expansion: one record per distinct release, so its clips, tracks and searches appear once.

The grouped view's open record lists its copies (cheapest first, sold last) as **copy rows** above its tracks (`ListRow::Copy` in `records.rs`; a record with copies opens even with one track). A copy row has no audio and selects no entry:
- double-click opens the listing on discogs.com, and its menu offers the cart;
- it isn't an entry, so play order, shuffle and the preview horizon never see it, and the arrow keys step over it;
- the copies live in the playlist as `SaleCopy` (the web build has no `dig`), converted from `dig`'s `Copy`.

In the flat view there are no copy rows: a track's tooltip lists its record's copies, and its menu offers Add to cart ▸ copies. Seller crates default to grouped, like the wantlist and collection crates.

*Alternative:* one record per copy (copy = album). Rejected: the same tracks would play once per copy, and the preview horizon would treat them as distinct entries.

### 5. Refresh is a diff by listing id

A refresh re-reads the listings with the saved criteria, then:
- **new listing:** add a copy (and expand its release if the crate doesn't have it yet);
- **listing gone:** set `sold`;
- **changed price:** update it, remembering the previous one for the summary.

A record whose copies are all sold stays, dimmed, until the user removes the seller or digs it again from scratch. If the refreshed total would go over 1,000, nothing changes and the Narrow down modal opens with the saved criteria.

*Why SOLD rather than removal:* the user may be listening to it, or have it in the cart, and it's useful to see what sold.

### 6. The cart

`discogs/cart.rs` provides:
- `read() -> CartSnapshot`: listing ids, release ids, and per seller `{count, subtotal}`;
- `add(ids) -> Vec<(id, AddResult)>`;
- `remove(id)`.

`transport.rs` gains a JSON body for POST (`send_json`); PUT and DELETE are unchanged. Calls go through the same client and limiter.

**When the snapshot is read:**
- at startup with a token, behind other work;
- after every dig or refresh of a seller crate;
- after every cart action.

It's cached in `<cache>/dig/cart.ron`, so badges show at once on the next launch.

**After an add, per item:**
- 201 with no error for it: in the cart;
- 422: already in the cart, counted as success;
- 404 "not for sale": SOLD;
- anything else, a transport failure, or an unexpected shape: the listing's `uri` opens in the browser and the main window says so.

The badge updates within a frame, optimistically, and the snapshot that follows confirms it. **The app never calls `DELETE /cart` or `DELETE /cart/seller/…`.**

**The badge:** CART on a copy whose listing is in the snapshot, on its record row, and (the bonus) on any entry anywhere whose release has a listing in the snapshot. Its colour is distinct from OWNED's amber.

**The filter** is a boolean in the crate's filter state (`cart_only`, saved), combined like the others: an entry passes when its release is in the cart. The playlist holds the cart's releases, which the app sets on the shown and playing crates each frame (a comparison, so the view is rebuilt only when they change). The footer label shows that seller's count and Discogs' formatted subtotal.

**A cart read that crosses an add:** a read sent before an add can answer after it and would take the new badge away. Copies whose add hasn't been answered are kept and applied again on top of every read until their answer comes.

### 7. Seller pages and the extension

`url.rs` gains `PageKind::Seller(String)` for:
- `/seller/{name}/profile` and `/seller/{name}` (with any query);
- `/user/{name}` with no further path. `/user/{name}/collection` stays a collection.

Sending a seller page (paste, or the bridge) never adds tracks: a new seller is looked up in the Add seller dialog (Add then opens the Dig flow, counted); one already in the list is shown and refreshed.

The bridge accepts a seller address with any mode and ignores the mode. It answers "Added seller ‹name›" or "Refreshed seller ‹name›", from the seller names in its snapshot (a field never serialized into any answer). The extension's `pages.js` recognises seller pages, and the page button there offers only "Add seller to ‹App›".

**Bringing the app forward:** after a bridge add-seller, the app sends `ViewportCommand::Focus`, falling back to `RequestUserAttention`. macOS may refuse to let a background app take focus (the Dock icon bounces instead). A `diggr://` URL scheme, which macOS honours, needs the app bundle and is left to `macos-release`.

## Risks / Trade-offs

- [Discogs changes or removes the undocumented `/cart` or `/purchases`] → Cart actions fall back to opening the listing. Seeding fails soft, and the user can add sellers by hand. Response parsing is lenient (unknown fields ignored, missing ones mean "unknown"). An `#[ignore]` test hits the real endpoints.
- [Adding to the cart by accident] → Add is one action per copy or selection, never automatic. The user reviews the cart on discogs.com before paying. The app never empties the cart.
- [Big sellers (hhv.de: 159,183)] → Only 10,000 are reachable. The modal says so and steers to search text and newest-N, which Discogs applies on its side.
- [Mixed currencies] → Prices are shown in the listing's currency. The CART subtotal is per seller, in that seller's currency. Nothing is converted.
- [Copy rows complicate the playlist] → They exist only in grouped seller crates, carry no audio, and are skipped by every play-order path. Headless tests cover play order, selection, and the horizon with copy rows present.
- [Purchases are personal data] → Read only to rank sellers, never saved beyond the resulting list, never sent anywhere else.
- [Rate limit] → A first dig of 1,000 copies costs 10 listing requests plus up to one release request per new record. Those are cached and fetched nearest the playhead first, as for any page.

## Migration Plan

- **New files:** `sellers.ron` and `cart.ron`.
- **New optional fields:** `CrateInfo.seller` and the playlist's `copies`. Older builds ignore them; an older build shows a seller crate as an ordinary crate of tracks.
- **Seeding:** existing users with a token are seeded once, at the first launch after this change.

## Open Questions

None left. Both were answered on 2026-10-07: `/purchases` honours `sort=created&sort_order=desc`, and `/cart` subtotals come only in the seller's currency (REWARM's is "£2.00"), so the CART switch shows Discogs' `formatted` subtotal as it is.
