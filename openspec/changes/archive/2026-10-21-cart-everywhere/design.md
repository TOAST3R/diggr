## Context

`entry_badges` (`crates/ui/src/app.rs`) draws a `Pill` when its caller passes one. Callers pass it for an unsold Discogs copy row, a record row with exactly one unsold copy, and an entry whose `store_item` finds a Juno, Bleep, Beatport or Traxsource item (`stores.rs`). An entry's `Origin` holds what is known: `release`, `for_sale`, `bandcamp`, `juno`, `bleep`, `beatport_page`, `traxsource_page`. `StoreMemory::product_of(release)` finds a Juno or Bleep product a store send linked to a Discogs release. Dialogs that hold the keyboard register in `dig_asking()`.

## Goals / Non-Goals

**Goals:**
- Every dig entry and record row has a pill, and a click always leads somewhere useful.
- No modal when exactly one store is known.

**Non-Goals:**
- Matching records on stores the user never sent from (no store requests from the app; stores are reached only through the extension).
- Choosing a Discogs copy automatically (cheapest, nearest).
- A pill on local files.

## Decisions

### D1. "Known" stores, in this order
For an entry (or a record row: its first entry), the known stores are:
1. **Discogs copy**: a copy for sale known in the crate (seller crate): the existing Add to cart.
2. **Juno / Bleep**: `origin.juno` / `origin.bleep`, else `product_of(release)` in that store's memory: the existing basket action.
3. **Beatport / Traxsource**: `origin.beatport_page` / `traxsource_page`: open the page (existing fallback).
4. **Bandcamp**: `origin.bandcamp`: open the album page (no basket is reachable).
5. **Discogs release**: `origin.release` with no copy known: open `https://www.discogs.com/sell/release/‹id›`, labelled with `for_sale` ("12 for sale from €9") when known.

Exactly one → act on it. Zero or several → the chooser.
*Alternative:* always open the chooser. Rejected: users asked for no modal with one store, and most entries know only one.

### D2. Unknown stores become search links, not disabled rows
A greyed row only says "we don't know"; a link gives the next step and costs nothing (no matching, no request from the app). The link opens the store's search page for "‹record artist› ‹record title›" (the track's artist and title for a single-track store entry) in the default browser.

| Store | Search address (query URL-encoded) |
|---|---|
| Discogs | `https://www.discogs.com/search/?q=‹q›&type=release` |
| Bandcamp | `https://bandcamp.com/search?q=‹q›&item_type=a` |
| Juno | `https://www.juno.co.uk/search/?q%5Ball%5D%5B%5D=‹q›` |
| Bleep | `https://bleep.com/search/query?q=‹q›` |
| Beatport | `https://www.beatport.com/search?q=‹q›` |
| Traxsource | `https://www.traxsource.com/search?term=‹q›` |

The addresses live in one function in `crates/dig` with a unit test per store.

### D3. The chooser is a small blocking dialog
Title "Buy ‹artist› – ‹title›"; a KNOWN section of buttons (store name, price or count when known), then a SEARCH section of links. Esc or a click outside closes it. It registers in `dig_asking()` so keys don't reach the player; playback continues. Acting on a known store closes it.

### D4. Record rows with several Discogs copies
The pill on such a row opens the record (as ⏵ does), so its copy rows with their own pills show. It no longer shows the CART badge in place of a pill; a copy in the cart still makes the pill read IN CART.

### D5. Pill states outside carts
For stores without a basket (Bandcamp, Discogs release, Beatport, Traxsource) and for the chooser, the pill always reads + CART; IN CART only reflects a real basket (Discogs cart snapshot, Juno/Bleep baskets).

## Risks / Trade-offs

- [Rows get crowded with a pill everywhere] → the pill keeps its fixed width and sits where badges are; the narrow single-line layout already fits it on store entries.
- [Store search URLs change] → one function, unit-tested shape; a wrong address only opens a store page with no results.
- [Artist/title on the record are noisy ("Various", catalogue in title)] → use the record artist except "Various", then the track artist; strip bracketed catalogue numbers as `bandcamp-intake` does.

## Open Questions

- Verify each store's search address by hand before release (the table is from their current sites).
