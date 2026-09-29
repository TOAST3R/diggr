## Why

Two small frictions when digging from the browser:
- "New crate…" opens an empty prompt, so each dig crate has to be named by hand, usually after the page being sent.
- Marketplace listings (`/shop/item/…`) are where records are actually bought, but the extension shows no button there, and the app refuses the address.

## What Changes

- **Prefilled crate name:** "New crate…" suggests a name from the page's title.
  - a release or master: `Artist - Album`;
  - an artist, label or list: its name;
  - a wantlist: `Wantlist: user`.

  Discogs' naming quirks are cleaned up: the `*` on name variants, disambiguation suffixes like `(2)`, and its long dash, which becomes `-`. The name is cut to the 40-character limit, and the user can edit it before confirming.
- **Marketplace listings:** the app accepts listing addresses (`/shop/item/{id}` and the older `/sell/item/{id}`, with or without `www.`, a language prefix or a query) and digs the release the listing sells, which it looks up with one Discogs request. The extension shows its button on listing pages, and its right-click menu offers listing links.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `chrome-extension`: the page button on listing pages, and the prefilled New crate… name.
- `discogs-intake`: listing addresses are supported pages, resolved to their release.

## Impact

- `extensions/chrome/pages.js`: listing patterns in the page kinds and the link patterns.
- `extensions/chrome/content.js`: the prefill, from `document.title`.
- `crates/dig/src/discogs/url.rs`: `PageKind::Listing(u64)`.
- `crates/dig/src/discogs/expand.rs` (or client): resolve a listing to its release (`GET /marketplace/listings/{id}` → `release.id`), then expand it as a release.
- The extension still reads only the page's address and title.
