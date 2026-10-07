## 1. Discogs calls

- [x] 1.1 `transport.rs`: POST with a JSON body (`send_json`), PUT/DELETE unchanged; `Client::call_json`; fake transport records bodies; tests
- [x] 1.2 `url.rs`: `PageKind::Seller` for `/seller/‹name›/profile`, `/seller/‹name›`, `/user/‹name›` (no further path; `/user/‹name›/collection` stays a collection), with `www.`, language prefix and query; `SUPPORTED` text; unit tests
- [x] 1.3 New `discogs/seller.rs`: inventory count (`per_page=1`, optional `q`), listing pages (100 per page, stop at page 100, `sort=listed desc` for newest N), listing parsing (lenient: unknown fields ignored, missing ones unknown); tests on fixtures taken from the real responses
- [x] 1.4 `seller.rs`: the first list (read `/purchases`, keep the 100 most recent locally, rank by orders then latest order, take the first 10 with copies for sale); tests (ranking, tie, closed seller skipped, no purchases)
- [x] 1.5 New `discogs/cart.rs`: `read` → `CartSnapshot` (listing ids, release ids, per-seller count and subtotal), `add(ids)` with per-item results (added / already / sold / other), `remove(id)`; never calls `DELETE /cart` or `/cart/seller`; tests on fixtures
- [x] 1.6 `#[ignore]` tests against the real API with the user's token: inventory, `/purchases`, `/cart` read (no writes)

## 2. Top Sellers and digging

- [x] 2.1 `sellers.ron` in `<config>/dig/` (seeded flag; per seller: username, id, crate, source, criteria, last dug, total), atomic writes; `CrateInfo.seller`; renamed crates stay attached; tests (round trip, older index without the field)
- [x] 2.2 Seeding once on token save or at the first launch with a token, behind other work, retried after a failure; headless test with a fake client (10 crates, empty, not dug; not seeded again after removing all)
- [x] 2.3 Sidebar: TOP SELLERS under DISCOGS (fold ▸/▾ remembered, count, never-dug crates dimmed), heading menu Add seller…, seller menu Refresh seller / Narrow down… / Rename crate… / Remove seller…; headless tests
- [x] 2.4 Click shows from disk ("Double-click to dig ‹seller›" when empty); double-click state machine (never dug → count → Dig or Narrow down; > 24 h → refresh; ≤ 24 h → nothing); without a token → Connect dialog; headless tests with fake time
- [x] 2.5 Dig modal and Narrow down modal (search, newest N, format group, price range, minimum condition, ships from; live count within a frame; reading progress and Cancel; Dig enabled for 1–1,000; the 10,000 note); criteria saved; headless tests
- [x] 2.6 Digging: copies onto the crate (`copies`, saved), one record per distinct release through the usual expansion (vinyl first, searches), progress "‹seller›: n of N copies", grouped by default; headless tests (three copies → tracks once)
- [x] 2.7 Refresh: diff by listing id (new → copies and expansion, gone → SOLD, price changes), summary in the main window, over 1,000 → unchanged and Narrow down opens, "Refreshing…" disabled, offline changes nothing; headless tests
- [x] 2.8 Add seller… modal (page or name, lookup 500 ms after typing stops, count shown, unknown user, already there → Refresh) and pasting a seller page; Remove seller… with confirmation; headless tests

## 3. Copies, SOLD and the cart

- [x] 3.1 Copy rows in grouped seller crates (cheapest first, sold last; price, conditions, ships from, badges), skipped by play order, shuffle, next track, horizon and Enter; double-click opens the listing; flat view lists copies in the track tooltip; record row second line "n copies €a–€b"; headless tests
- [x] 3.2 SOLD badge (OWNED's style), dimmed copy rows and all-sold record rows, still playable; tests
- [x] 3.3 Cart snapshot: read at launch (behind other work), after digs, refreshes and cart actions; cached in `<cache>/dig/cart.ron`, shown in the first frame; CART badge (its own colour) on copies, record rows and any entry whose release has a copy in the cart, with the tooltip "In your cart from ‹seller›, ‹price›"; headless tests
- [x] 3.4 Add to cart / Remove from cart on copy rows, tracks and record rows (single item, ▸ submenu per copy, one request per add), the badge within a frame, outcomes (already → CART, sold → SOLD, other → open the listing and say so), Connect dialog without a token; headless tests with a fake cart
- [x] 3.5 CART switch in the footer ("CART n · subtotal", lit when on, last after the buttons), combined with the other filters, remembered per crate; ☰ Show cart only, Open cart on discogs.com, Show all records turns it off; headless tests (play order under the switch, 5,000-entry crate within 16 ms)

## 4. Extension and bridge

- [x] 4.1 Bridge: a seller address with any mode adds or refreshes the seller, answer "Added seller ‹name›" / "Refreshed seller ‹name›", then `ViewportCommand::Focus`, falling back to `RequestUserAttention`; bridge tests
- [x] 4.2 Extension: `pages.js` recognises seller pages; the page button there offers only Add seller to ‹App›; feedback as for other sends

## 5. Docs and checks

- [x] 5.1 README (Top Sellers, double-click, Narrow down, copies, SOLD, CART and the switch, the extension, `sellers.ron` and `cart.ron` in the config/cache tables, the undocumented endpoints, the test count) and help panel; note the `winamp-rust://` URL scheme as a follow-up in `macos-release`
- [x] 5.2 fmt, clippy, the workspace tests (`--no-fail-fast`), the wasm check, `--bench`; the `#[ignore]` real-API tests; by hand: first list from purchases, dig a small seller, narrow down a big one, add and remove a copy in the cart and check it on discogs.com, add a seller from the extension
