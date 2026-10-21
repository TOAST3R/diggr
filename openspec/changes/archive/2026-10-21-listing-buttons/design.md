## Context

`content.js` adds a small ▶ (closed shadow root, `PLAY_STYLE`) after each record's title in two places: `itemButtons()` for Discogs links to `/sell/item/‹id›` (sending that address with mode Play), and `bleepButtons()` for Bleep tiles (sending `WRBleep.readTile(tile)` with `sendStore`). Both run from `tick()` every 500 ms, so they cover content loaded later, and skip links that already have a ▶. The store readers already parse lists: `WRJuno.readListing` groups sample links by product, `WRTraxsource.readListing` walks `/track/` links to their rows, `WRBeatport.read` takes tracks from the page's Next.js payloads. The player accepts any supported Discogs or Bandcamp address with `send`, and any number of store records with `sendStore`.

## Goals / Non-Goals

**Goals:**
- One ▶ per record (per track on Beatport and Traxsource) on every store listing, including search results.
- Each sends only its own record, with mode Play, and never navigates.
- Nothing is read until a ▶ is clicked, beyond finding where to put it.

**Non-Goals:**
- A Send to crate menu per record (the page button keeps it).
- Changes in the player.
- Pages the extension can't match today outside these stores.

## Decisions

### D1. One placer per store, one shared ▶
A helper `playButton(after, onClick)` builds the ▶ (the existing shadow-root markup and style, marked `wr-item-play`, skipping an anchor that already has one); `itemButtons` and `bleepButtons` use it too. Each store adds a placer called from `tick()`.

### D2. Discogs: any release or master title link
The Discogs placer matches `/(xx/)?release/‹id›` and `/(xx/)?master/‹id›` links with text (the title, not the picture), one per id, on every Discogs page except a release's or master's own page (where the page button already plays it, and its tracklist links to other versions). Marketplace items keep their `/sell/item/` rule. The ▶ sends the link's absolute address.
*Alternative:* only on `/search`. Rejected: labels, artists and lists list records the same way.

### D3. Bandcamp: album and track links on search and music pages
On `bandcamp.com/search` and on an account's `/music` (or root) page, a ▶ after each album or track title link (`*.bandcamp.com/album/…`, `/track/…`) sends that address. The search result's link carries tracking parameters; the address is sent without its query.

### D4. Juno, Beatport, Traxsource: read one row on click
Each reader gains a function returning the record (Juno, by product id) or track (Beatport, Traxsource, by track id) the clicked row shows, reusing its list code, so a ▶ sends `items: [that one]` with `sendStore`. Placing the ▶ only finds title links (`/products/…` on Juno, `/track/…` on Beatport and Traxsource); the row is read on click, as Bleep does.

### D4b. Beatport search pages have nothing to read
Found when checking on the live sites: Beatport's search results come from its search service with a different shape (`track_id`, `track_name`) and no preview sample, which the reader and the player rely on. The page's own button never read them either. The Beatport placer only adds a ▶ for tracks its page data holds (read once per load), so search results get none rather than a ▶ that fails. Supporting them means the player taking Beatport tracks without a sample and searching YouTube for them, as for Traxsource: a later change.

### D4c. Hidden copies
Discogs keeps hidden copies of some title links for other layouts. One ▶ per record, put on the first link, landed on a hidden copy; links that aren't displayed are passed over.

### D5. Bleep search results
Bleep's search results (`/search/query`) are a `BleepPage` already; the tiles are checked to be the same markup, so `bleepButtons` covers them, or its tile lookup is widened.

## Risks / Trade-offs

- [Store markup changes and a ▶ goes missing or lands oddly] → each placer only adds after a matching link and fails quietly; the manual checklist covers each store.
- [Too many ▶ on dense Discogs pages (a label's 500 releases)] → one per id, inexpensive shadow roots, the same 500 ms tick the marketplace already uses.
- [A ▶ inside a link that the store's own script handles] → the click stops propagation and default, as today.
