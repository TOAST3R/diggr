## Why

On Discogs, a seller's shop has many addresses: its home (`/seller/‹name›/profile`), its other pages (`/seller/‹name›/mywants`, `/feedback`…), and the marketplace list filtered to that seller (`/sell/list?seller=‹name›`). Today only the home and `/user/‹name›` show the extension's button, the other catalogue pages show nothing, and a link to a seller has no menu item. A digger browsing a seller's stock should be one click from Top Sellers wherever they are in that seller's pages, with the same wording the label pages get.

## What Changes

- **Every seller page is recognised**, in the app (paste) and in the extension: `/seller/‹name›` and anything under it (`/profile`, `/mywants`, `/feedback`, …, with or without filters), `/sell/list?seller=‹name›` (any other filters), and `/user/‹name›` alone, as today. Each names the seller and adds it to Top Sellers (or refreshes it), never sends tracks. Filters on the address are ignored, as today.
- **The extension's seller button reads "‹App›: Add seller"**, or **"‹App›: In Top Sellers · Refresh"** when the seller is already in the list. It is the only item in the menu.
- **Right-clicking a link to a seller's page**, on any website, offers only "‹App›: Add seller".
- **Bridge:** the crates answer lists the Top Sellers' usernames, so the extension can show In Top Sellers.
- Choosing the item keeps today's seller behaviour: the seller is added (the dig flow opens) or refreshed, its crate is shown, and the app comes to the front.

Depends on `label-crates`: both change the extension's "Page button" and "Links anywhere" requirements. This change's versions include the label wording and assume `label-crates` is archived first.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `seller-crates`: "Seller pages" covers every page under `/seller/‹name›/` and `/sell/list?seller=‹name›`.
- `chrome-extension`: "Page button" uses "‹App›: Add seller" / "‹App›: In Top Sellers · Refresh" on every seller page; "Links anywhere" offers it on seller links.
- `browser-bridge`: "Crates and status" includes the Top Sellers' usernames.

## Impact

- **`crates/dig/src/discogs/url.rs`:** `/seller/‹name›/…` (any tail) and `/sell/list` with a `seller` query parameter parse as `PageKind::Seller`; tests (the `/seller/decks.de/feedback` case flips from Unsupported to Seller).
- **`crates/dig/src/bridge/mod.rs`:** `Snapshot.sellers` is serialised in the crates answer.
- **`extensions/chrome/`:** `pages.js` (seller patterns, seller link patterns), `content.js` (wording and followed state), `background.js` (seller link menu item).
- **Docs:** README (seller pages, extension checklist).
- No change to the app's seller logic, playback, analysis or visuals.
