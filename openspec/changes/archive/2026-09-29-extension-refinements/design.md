## Context

- The extension decides where its button appears with coarse patterns (`pages.js`); the app parses the address itself (`dig::discogs::url`) and refuses what it doesn't know.
- "New crate…" is a bare `prompt()` in `content.js`. The extension's spec lets it read a page's address and title, and nothing else.
- A Discogs marketplace listing page names a listing id, not a release. The API's `GET /marketplace/listings/{listing_id}` returns the listing, including `release.id`. It needs no token, and counts against the same rate limit.

## Goals / Non-Goals

**Goals:**
- One click from a listing to its release in a crate.
- A sensible default crate name, still editable.

**Non-Goals:**
- Anything about the listing itself (its price, its seller).
- Reading the page's HTML.

## Decisions

**1. The app resolves listings (not the extension).**
A new `PageKind::Listing(id)` is resolved to `Release(release_id)` in the intake, with one request that is cached like other Discogs data. Then it is expanded exactly as a release: same entries, same filters, and the crate name "Release: …".

Alternative: the extension reads the release link from the page. Rejected, because it breaks the "address and title only" rule, depends on Discogs' HTML, and wouldn't help pasted links.

**2. A listing that doesn't exist, or has been sold and removed, is a page that isn't found.**
It gets the usual "page wasn't found" message, and no crate changes.

**3. The prefill comes from `document.title`.**
Discogs titles look like `D'Arcangelo* – TimeLss | Releases | Discogs`. Steps:
- drop the ` | Kind | Discogs` suffix (everything from the first ` | `);
- remove the `*` after names, and the ` (2)`-style number suffixes;
- turn the long dash into `-`, and collapse spaces;
- cut to 40 characters, at a word boundary when possible.

For a wantlist, use `Wantlist: user`. When the title yields nothing, leave the prompt empty.

## Risks / Trade-offs

- [Discogs changes its title format] → the prefill is only a suggestion. A bad one is edited, and the send itself doesn't depend on it.
- [A listing costs one extra request] → accepted. It is cached, so sending the same listing twice costs nothing.
