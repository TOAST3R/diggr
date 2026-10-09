## 1. Recognising seller pages

- [ ] 1.1 `url::parse`: `/seller/‹name›/…` (any tail) and `/sell/list?seller=‹name›` as `PageKind::Seller`; `/sell/list` without `seller` stays Unsupported; update the `/seller/decks.de/feedback` test and add cases (`/mywants`, `/sell/list?seller=decks.de&style=Techno`, language prefix, `/sell/list` alone)
- [ ] 1.2 Headless or intake test: pasting `/sell/list?seller=decks.de` adds decks.de to Top Sellers and adds no tracks to the shown crate

## 2. Bridge

- [ ] 2.1 `Snapshot.sellers` serialised in the `/v1/crates` answer as `sellers`; test in `crates/dig/tests/bridge.rs` (the answer lists them; a seller catalogue page sent with mode Enqueue adds or refreshes the seller)

## 3. Extension

- [ ] 3.1 `pages.js`: seller page pattern for any `/seller/‹name›/…`, the `/sell/list?seller=` check, and `SELLER_LINK_PATTERNS`
- [ ] 3.2 `content.js`: "‹App›: Add seller" or "‹App›: In Top Sellers · Refresh" from the crates answer's `sellers`, the seller's name read from the path or the `seller` parameter
- [ ] 3.3 `background.js`: "‹App›: Add seller" link item with `SELLER_LINK_PATTERNS`, beside the label item from label-crates; run the extension's manual checklist for seller pages, catalogue pages and seller links

## 4. Docs and checks

- [ ] 4.1 README (seller pages and links in the extension section, the manual checklist, test count); run `cargo test --workspace`, clippy, fmt and the wasm check
