## Context

The cart pill on a row is chosen in three places in `crates/ui/src/app.rs`: the record row (a lone Discogs copy's pill, the pill that opens a record of several copies, or `store_pill`), the track row (`store_pill` only) and the seller crate's copy row (`copy_row`). `store_pill_of` (`app/stores.rs`) gives a store's pill or the Buy pill. The only filter today hides a Buy pill when the release already has a copy in the Discogs cart.

Owned comes from `Dig::owned(e)` (`app/digging.rs`): with a token and a synced collection, `Collection::owned(release, master)` gives `ThisPressing` or `Another { catno, year }`. `dig_marks` reads it but turns it off in the collection crate and for entries sent from a collection page, so they don't all wear OWNED. The pill must not take its rule from that badge flag.

The Discogs cart page is `sellers::CART_PAGE` (`https://www.discogs.com/sell/cart`), opened today by `SellerAction::OpenCart` from the list menu of seller crates only. Discogs checks out per seller from that page. There's no single checkout URL, so the cart page is the destination.

## Goals / Non-Goals

**Goals:**
- No + CART or Buy pill on a record the user owns, wherever it is shown.
- An owned record already in a cart keeps the means to take it out, and says it's owned.
- One click from any row in a cart to that store's cart page.

**Non-Goals:**
- Checking out, paying or emptying a cart from Diggr (both cart specs forbid it).
- Taking owned records out of a cart automatically.
- A Buy route for a second pressing of an owned record (the user can still buy it on the store's page).
- Beatport and Traxsource: their baskets can't be reached, so their items never show IN CART or `»`.

## Decisions

### D1. One "owned" rule, asked directly

`DiggrApp::owns(e) -> Option<String>` (the owned pressing as `owned_text` words it, empty when only the view says so), the single source the three pill sites use:
- the shown crate is the collection crate, or the entry was sent from a collection page » owned, with no detail;
- else `Dig::owned(e)` (this pressing or another of its master).

A copy row asks with the copy's release and its record's master, through the record's first entry, which carries them.

*Alternative:* reuse `DigMarks::owned`. Rejected: it's `None` in the collection crate by design, which is the very place the bug shows most.

### D2. Owned decides the pill, then in-cart wins

```
in cart?   owned?   pill
  no         no     as today (+ CART, record-opener, store, Buy)
  no         yes    none (OWNED badge only, where it is drawn)
  yes        any    IN CART (REMOVE on hover), + »
```

For a record with several unsold copies: no pill when owned, unless one of them is in the cart (then IN CART that opens the record, as today). The CART badge (no pill) rows are unchanged, and get `»`.

### D3. The owned warning lives in the tooltip

An owned row in a cart reads "You already own ‹owned_text›. " ("this pressing", "another pressing (AF001R, 2019)") before its usual in-cart tooltip (`format::owned_warning`). In the collection crate, where owned has no detail: "You already own this. ". There's no new badge or colour, since OWNED already shows outside the collection crate.

### D4. `»` is its own narrow pill, after IN CART or CART

The label is `PAY »`: a bare `»` said nothing until hovered, so it got a word, and the `»` (not an arrow) because egui's default fonts, the only ones the app loads, have no arrow (`→`, `➜`, `➡`, `⇨` all missing, checked in a test), and `⏵` already opens a record. Every row with a cart pill keeps the `»` slot after it, drawn or blank, so going in or out of the cart never moves the row's text and copy rows stay lined up.

It has a fixed width that fits "»" in the badge font, drawn in the CART colour as an outline (filled while hovered), so it doesn't read as a second IN CART. Its rectangle comes back from `entry_badges` next to the cart pill's, and its click is registered after the row's the same way (`cart_pill_click`), so it never selects, plays or drags the row.

Which cart it opens: the store of the row's pill (a copy pill » Discogs; a store pill » its store). For a CART badge: Discogs when the Discogs cart holds a copy of the release, else the store whose basket holds the item.

*Alternative:* the words "→ CHECKOUT". Rejected for width: it would push titles off a narrow playlist on every row in a cart. The tooltip carries the words.

### D5. Store cart pages

`Store::cart_page() -> Option<&'static str>`: Juno `https://www.juno.co.uk/cart/` (the page the extension reads the basket from), Bleep `https://bleep.com/cart` (the live site serves scripts a bot challenge, so it couldn't be checked from here; `/cart` is the root of the cart routes the extension already calls, such as `/cart/remove/…`), Beatport and Traxsource `None`. Discogs stays `sellers::CART_PAGE`. `SellerAction::OpenCart` becomes an action carrying the cart (Discogs or a store), shared by the pill and the list menu.

### D6. List menu

"Open cart on discogs.com" shows in every crate while the Discogs cart snapshot holds a copy (in seller crates it also stays when empty, as today), and "Open cart on juno.co.uk" or "Open cart on bleep.com" while that basket holds an item.

### D7. One pill per record in the list, centred on its run

A record's tracks used to carry a pill each, though each one buys the same record. In the list, consecutive rows of one record (`Entry::album_key`, flat rows only) form a run (`record_run`). The run's mark (its pill, or CART, then `»`) is drawn once: by the first row of the run in view, centred vertically on the part of the run in view, so it stays visible while scrolling. Every row of the run keeps the mark's slot blank (`Badges::slot`), so the titles line up, and a run of two or more rows gets a thin line above and below it to set records apart. The mark's clicks are registered after all the rows, since it lies over rows drawn after it. Finding a run walks only the neighbours of the visible rows. In crate order a record's tracks are consecutive. Sorted by another column, they scatter, and each run gets its own mark. Tracks under an open record leave it to the record row. Beatport and Traxsource items are bought per track and have no `album_key` unless linked, so each is a run of one.

## Risks / Trade-offs

- **The collection isn't synced yet, or there's no token** » owned is unknown and + CART shows outside the collection crate. That's the same as the OWNED badge today. The token hint already explains it.
- **"Another pressing" hides + CART for a pressing the user may want** » the user chose this. The store's page is still one search away.
- **Bleep's basket URL is unverified** » it's a task to confirm it. If it's wrong, the fallback is `https://bleep.com/` and the tooltip still names the store.
- **Row width** » the `»` pill adds about two characters only on rows in a cart.
