# Design

## Context

- One function draws every playlist hover: `entry_tooltip` (`crates/ui/src/app.rs`), with a
  96 pt cover beside a grid of `format::entry_details` lines. It has two callers. The record row
  (~5065) passes its first entry's details with `format::record_counts` added as a "Record"
  line. The entry row (~5389) adds the tempo wait and "waiting for <source>" to the Status line.
- Covers: `dig::cover` runs one low-priority worker thread with two queues (the newest hover
  first, then the rows in view). It shrinks every image to `MAX_PX = 150` and saves
  `<cache>/covers/{release,master,bandcamp}-<id>.png`. The UI side (`app/covers.rs`,
  `CoverCache`) keeps up to 128 textures by `CoverKey`, asks after a 250 ms rest
  (`slot`) or for rows in view (`row_slot` + `want_in_view`), and handles `Stale` (403/404) by
  asking intake for fresh record data, once (`expand::fresh_cover`).
- `Origin.cover` holds one address. For Discogs that is the signed `uri150`/`thumb` URL, whose
  signature covers the size, so a large URL can't be built from it. The collection and wantlist
  `basic_information`, and search results, also carry `cover_image`. Release and master
  `images[]` carry `uri` next to `uri150`. Seller listings carry only `thumbnail`.
- `cover::allowed` lets requests go only to Discogs' and Bandcamp's image hosts.
- Skin colours available: `pl_bg`, `pl_text`, `pl_current`, `pl_selected_bg`, `pl_title`,
  `pl_owned`. There is no frame colour.

## Goals / Non-Goals

**Goals:**
- A record row opens its card on a click, a track row shows a text tooltip on hover.
- Playing from the card is the same play as from the list: same actions, same latency.
- Large covers cost no API budget, and no more image-host traffic than one fetch per record
  the card settles on.
- Old crates load and work as before, just with softer covers.

**Non-Goals:**
- A docked info pane, several cards at once, or a card for flat crates.
- Seeking, a waveform, or wantlist and cart buttons in the card (the record row's pills and
  menu keep those).
- Large covers for stores other than Discogs and Bandcamp, or a backfill of large addresses
  through API requests.
- Changing what the record row itself draws.

## Decisions

**1. Split the details into two builders in `format.rs`.**
- `track_details(e, marks, now, with_record: bool)` keeps today's `entry_details` order and
  leaves out the record lines (Album, Style, Label, Cat#, Year, For sale) when `with_record` is
  false. Flat rows pass true and tracks under an open record pass false.
- `record_details(members, marks, copies, playing, now) -> RecordCard` returns a plain struct:
  artist, album, a pressing line, styles, tracklist lines (entry id, position, title, bpm,
  state: playing, paused, dim, unavailable or normal), marks, and a sale line or copies line plus
  the counts. `record_counts` moves inside it. The tracklist lists every shown entry (it scrolls
  in the card), so the 8-line cap and "… N more" go.

Both are pure functions, so the spec scenarios can be unit-tested in `format.rs` without egui.
The other option, one builder with filters, would keep the record-vs-track logic in the drawing
code, where it can't be tested.

**2. The open card is app state: `card: Option<OpenCard { crate_id, key: AlbumKey, since }>`.**
- A plain click on a record row pushes `Action::OpenCard(key)`, which opens it, switches it to
  that record, or closes it when it is already on it (the click's `SelectRecord` still runs
  first, so the selection is unchanged). Modifier clicks only push `SelectRecord`.
- Each frame, before the list is drawn, the card closes when `crate_id` isn't the shown crate,
  the crate isn't grouped, or no shown row has `key` (removed, filtered away). That check reads
  the rows already built for the frame, so it costs nothing extra.
- `since` is when the card started showing that record: the large cover is asked for 250 ms
  after it (decision 5), so quick stepping fetches only where it stops.

Keying the card by `AlbumKey` and crate, not by row index, keeps it on its record when rows
above it open, close or are added. The other option, a hover with a "pin" button, would still
pop up while aiming elsewhere.

**3. The card is an `egui::Area`, not a tooltip, centred in the window.** A tooltip can't take
clicks. An `Area` at `Order::Foreground`, pinned to the window's centre with a centre pivot, drawn
with the skin's frame: `pl_bg` fill and a 1 px stroke at `lerp(pl_text, pl_bg, 0.6)`. Centring
(over anchoring to the row) means the card never jumps as it steps or the list scrolls, and
never needs flipping at the window's edge. Sizes are in skin points × `sk.scale` (cover 128, max
width 420, or the window's width when narrower), so 2× doubles everything. An × at the top right
pushes `CloseCard`.

The tracklist is an `egui::ScrollArea` capped at 8 lines high. Each line is one
`ui.allocate_exact_size` row: a ⏵/⏸ hit rect drawn with the skin's text colour, then side,
title clipped, and at the right the tempo, or for a waiting track its state ("queued", "40%",
"search", "next" when armed). Pills and the strip stay painter calls. ‹ › are two small hit
rects beside the artist line, dimmed when disabled.

**4. The card's buttons push the list's own actions.** ⏵ on a track pushes
`Action::PlayEntry(id)`, which is what a double-click does: it plays ready audio, arms a waiting
entry (the scheduler fetches the armed entry before any other, so "not downloaded yet" plays as
soon as it lands), and retries a failed clip or a track not found. Only no-clip and
already-in-crate tracks get no button, since there's nothing to try, ⏵ on the paused
current track pushes `Action::Play`, and ⏸ pushes `Action::Pause`. ‹ and › push
`Action::CardStep(-1 | 1)`. That action finds the previous or next `ListRow::Record` in the
frame's rows, then selects it (`SelectRecord`), moves the cursor to it, scrolls the list by
the least amount that shows it (the helper `P` and the keyboard already use), and switches the
card to it, with `since` reset. Nothing new touches the engine, so play from the card has the
list's 30 ms start.

**4b. The record's actions are the row's pills, described by the app.** The app builds a list of
`CardPill { label, tip, kind (button, badge or checkout), lit }` from the same logic the record row
uses (`dig_marks`, `copies_of`, `store_pill`, `checkout_page`): + WANT / WANTED, the copy's or the
store's cart pill, » checkout, N COPIES, OWNED, SOLD. `card.rs` draws them in the strip and
returns `CardAction::Pill(i)`; the app maps index `i` to the row's own action (`Want`/`Unwant`,
`CartAdd`/`CartRemove`, `StoreCart`, `Buy`, `OpenCart`, `ToggleRecord`). Keeping the decision in
the app means the card and the row can't disagree about what a pill does.

**5. Keys and closing.** In the key loop, while a card is open: Esc closes it (before the
help panel's Esc and the playlist's own); ← and → push `CardStep` instead of closing and
opening records. A click closes the card when it lands outside both the card's area (its
response rect, from this frame) and the list's record rows (a record row's click reroutes the
card). Clicks inside the card are consumed by the card, so they never reach rows under it.

**6. A cover size in the cover worker's key, not a second worker.** Add
`CoverSize { Thumb, Large }`. Requests become `(CoverKey, CoverSize, url)`. Large is stored at
most 600 px under `<cache>/covers/large/<same name>.png`. The 2 MB download cap stays as it
is: Discogs' full images and Bandcamp's 700 px art are a few hundred KB. Large requests go only through the hover queue. `want()` (rows in view) stays
thumb-only, so sweeping rows never fetches large covers. Pace, 429 backoff and the
failed-this-session sets are shared, keyed by `(key, size)`. A second worker would double the
request rate against the same hosts.

**7. The UI asks for both once the card settles.** `CoverCache::card_slot(ctx, e, since)`
returns `(thumb, large)`. Once the card has shown the record for 250 ms (`since`), it asks for
the thumbnail (if missing) and then the large cover (if the entry has a large address and the
large cover hasn't failed). It's called only for the open card's record, never on hover. The hover queue keeps only the newest request, so it changes to
hold the newest *record*: both sizes for one key, thumb first. Large textures count toward the
128-texture cap like any other. Each costs about 1.4 MB of GPU memory at 600², so the cap
becomes 128 thumbs + 16 large covers, least recently drawn dropped first.

**8. `Origin.cover_large: String`, `#[serde(default)]`.**
- `RecordInfo`/`Listed` get the same field. `model.rs` fills it from `basic_information.cover_image`
  (collection, wantlist), `cover_image` (search) and the primary image's `uri` (release, master,
  next to `cover()`), through the same `thumb()` https/spacer check.
- Bandcamp has no stored field. `cover::bandcamp_large(url)` rewrites `_<n>.jpg` to `_16.jpg`
  (700 px) on the same host.
- When a stale thumbnail triggers `fresh_cover`, the fresh data also refills `cover_large`, at
  no extra cost. A stale *large* cover is treated as failed for the session (no refresh), and
  the card keeps the thumbnail.

**9. Hover routing in `app.rs`.** The record row has no hover. The entry row's hover calls
`track_details` with `with_record = !track` (a track under an open record), and draws the
existing grid with no cover. `entry_tooltip` loses its cover branch and becomes the track
tooltip.

## Risks / Trade-offs

- [Discogs' signed large URLs may expire like thumbnails] → They are treated as failed for the
  session, and the thumbnail refresh path refills them for free when it runs.
- [A 600 px decode on the cover thread takes ~10–20 ms] → It is a low-priority thread, already
  off the UI and audio paths. The UI only uploads a texture when the result arrives.
- [The open card covers the rows under it] → It opens under its record row, at most 420 pt
  wide, and ← → step without reaching for rows. Esc or a click elsewhere puts it away.
- [A click on a record row now pops a card where it used to only select] → Modifier clicks
  only select, clicking the row again closes the card, and the selection is the same as
  before, so Remove, drag and the menu work as they did.
- [← and → mean something else while the card is open] → Only while it is open; they open and
  close records again as soon as it closes.
- [Old crates and seller crates look soft] → This is accepted. The scaled thumbnail is drawn
  with linear filtering, and the skin's nearest-neighbour setting doesn't apply to cover
  textures.
- [Track tooltips lose the cover users may rely on in flat crates] → The user chose this. Grouping
  (Shift+G) shows the card.

## Migration Plan

No data migration: `cover_large` defaults to empty, so crates saved before this change load
unchanged and fill it as their records are read again. `covers/large/` is created on first
write. Rolling back leaves an unused folder and an ignored RON field.
