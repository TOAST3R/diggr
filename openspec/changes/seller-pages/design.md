## Context

- `url::parse` recognises a seller only at `/seller/‹name›`, `/seller/‹name›/profile` and `/user/‹name›`. A test pins `/seller/decks.de/feedback` as Unsupported. `/sell/list?seller=…` isn't recognised.
- The extension's `pages.js` mirrors those patterns (`KINDS`) and has no seller entry in `LINK_PATTERNS`. `content.js` shows one item, "Add seller to ‹App›", on seller pages.
- The bridge's `send` already handles `PageKind::Seller` whatever the mode: add or refresh, show its crate, come to the front. `Snapshot.sellers` is kept up to date, but `#[serde(skip)]`, so the extension can't see it.
- `label-crates` changes the same extension requirements and adds `Snapshot.labels` and the label link item. This change builds on it.

## Goals / Non-Goals

**Goals:**
- One click to Top Sellers from any page of a seller's shop, and from links to it.
- The same wording pattern as labels: "‹App›: Add …" / "‹App›: In … · Refresh".

**Non-Goals:**
- Turning a catalogue page's filters (`?style=Techno`, price, condition) into dig criteria. The address's filters stay ignored, and the dig flow's Narrow down remains the place for criteria.
- Changing what adding or refreshing a seller does in the app.
- `/user/‹name›/…` pages other than the existing wantlist and collection ones.

## Decisions

### 1. Parsing: any `/seller/‹name›/…`, and `/sell/list?seller=`

In `url::parse`:
- `["seller", user, ..]` (any tail) gives `PageKind::Seller(user)`.
- `["sell", "list"]` with a `seller` query parameter gives `PageKind::Seller(seller)`; without one it stays Unsupported.
- `["user", user]` stays as it is.
- The language prefix (`/es/…`) is already stripped before matching.

The `/seller/decks.de/feedback` test changes from Unsupported to Seller, and new cases cover `/mywants`, `/sell/list?seller=decks.de&style=Techno` and `/sell/list` alone. `valid_user` still guards the name.

*Alternative:* only `/profile` and `/mywants`. Rejected: Discogs adds shop sub-pages over time, and every one of them is about one seller.

### 2. Extension patterns

`pages.js`:
- `KINDS` gets `^/${LANG}seller/[^/]+(?:/.*)?$` → Seller (replacing the profile-only pattern), and a query check for `/sell/list` with `seller`, like the existing wantlist `?user=` check.
- `SELLER_LINK_PATTERNS` (`*://www.discogs.com/seller/*`, `…/*/seller/*`, `…/sell/list*`) is exported separately.

`background.js` adds a "‹App›: Add seller" link item with those patterns, beside label-crates' label item. The match patterns can't check for `seller=` in `/sell/list*`. A link without it gets the item, the app answers "unsupported", and the toolbar badge shows "!". This is acceptable, and the same as today for other near-miss links.

### 3. Followed state

`Snapshot.sellers` loses `#[serde(skip)]` and is serialised in the `/v1/crates` answer as `sellers` (usernames, in list order). `content.js` compares the page's seller case-insensitively, as the bridge does, and shows "‹App›: In Top Sellers · Refresh" when it's there. It reads the name from the path, or from `seller=` on `/sell/list`. Older extensions ignore the new field.

### 4. Wording

"Add seller to ‹App›" becomes "‹App›: Add seller", matching label-crates' "‹App›: Add to Labels". Both come from the app's reported name, as every extension label does.

## Risks / Trade-offs

- [`/seller/‹name›/…` might one day hold a page that isn't about the seller's shop] → Every page under it names the seller, and adding a seller is reversible (Remove seller…).
- [Link items for `/sell/list` without `seller=`] → The app refuses them as unsupported, and the badge shows "!".
- [Ordering with label-crates] → This change's extension deltas are written on top of label-crates' versions. Archive label-crates first, or rebase this change's chrome-extension delta if the order changes.

## Migration Plan

None. Reload the unpacked extension after updating.
