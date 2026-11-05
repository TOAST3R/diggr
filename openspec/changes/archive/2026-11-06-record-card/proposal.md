# Proposal

## Why

Hovering a record row shows its first track's tooltip. The cover is small (96 pt, from a 150 px
thumbnail), and the details belong to one track: side A1, track 1's tempo, length and state.
It reads as if they described the whole record. The tooltip is a stock egui one, with none of
the skin. Records are what a digger is choosing between, so the record row deserves a card
that shows the record: its cover, the pressing, what's on it and whether it can be bought, and
lets you play its tracks and move on to the next record without hunting for rows. A hover is
the wrong trigger for that: it can't hold buttons, and it pops up while you only aim elsewhere.
Track rows carry the cover and record facts that their record row already shows, so they can
stay small.

## What Changes

- **Record card, opened by a click.** A plain click on a record row (grouped crate) selects the
  record, as today, and opens its card beside the row; clicking the row again closes it. Shift-,
  Cmd- and Ctrl-clicks only select, double-click still plays, and hovering a record row shows
  nothing. The card stays open while the pointer moves, and closes on Esc, a click outside, or
  when its record or crate goes away. It is drawn in the skin's playlist colours with a skin
  border and has:
  - a cover of 128 pt (256 at 2×), so a thumbnail scaled up while the large cover comes stays
    readable;
  - the record's artist and album, large, with ‹ › to step to the previous and next record
    (← and → too, while the card is open);
  - label · catalog number · year, and its formats;
  - its styles as pills;
  - its tracklist, each line with ⏵/⏸ to play, pause and resume that track (or arm it while it
    downloads), side, title and tempo; the playing track highlighted, tracks with no clip
    dimmed, scrolling inside the card past 8 lines;
  - a status strip with the wantlist, owned and cart marks and the for-sale snapshot (in a
    seller crate, its copies and price range).

  A record of one entry has a card too. Stepping selects the record and scrolls the list to it,
  and never changes what plays.
- **Slim track tooltip.** Hovering a track row (flat crate, or a track under an open record)
  shows a text-only tooltip with no cover: the full name, then side, tempo, key, time, state
  (with "waiting for …"), source, preview search, and the wantlist, collection, passed, owned
  and cart notes. In a flat crate, where no record row is shown, it also keeps the record's
  text facts (album, style, label, catalog number, year, for sale). Under an open record it
  leaves them out, because the record row and its card show them. A local file still shows its
  album and path.
- **Large covers.** The card asks for a large cover (at most 600 px) once it has shown a record
  for 250 ms (so stepping quickly fetches only where it stops), never for rows in view. It is fetched from the image
  host, never from the Discogs API, and kept on disk next to the thumbnails. Until the large
  cover arrives (or when there is none), the card shows the thumbnail scaled up. Record rows
  keep the 150 px thumbnail.
- **Large cover addresses.** Discogs entries get a new saved address for the large image, taken
  from data intake already reads (the collection, wantlist and search `cover_image`, and the
  release or master `images[].uri`). Bandcamp's large art comes from rewriting the art size in
  its address. Other stores keep their covers as they are today. Seller listings and crates saved before this change
  have no large Discogs address until their records are read again. They show the scaled-up
  thumbnail, and no API request is made to fill the address.

## Capabilities

### New Capabilities

- `record-card`: the record card: opening and closing it, what it shows, playing from it,
  stepping through records, how it looks, and its large cover (when it is asked for, its
  fallback, and where it is kept).

### Modified Capabilities

- `playlist`: "Entry tooltip" becomes the slim track tooltip: no cover, and no album, label,
  catalog number, year or for-sale lines for a track that has a record row. Record facts go to
  the record card.
- `album-entries`: "Cover on hover" moves the cover from the entry tooltip to the record card,
  and says the thumbnail now serves record rows and the card's fallback, fetched first for the
  open card.
- `preview-fetch`: "Download ahead" puts the open card's record right after the armed and
  playing entries, so its tracks are ready to play from the card.
- `record-view`: "Record row" no longer shows a tooltip on hover; a click opens the record card,
  which takes over the Record counts line. "Acting on a record" adds that a plain click also
  opens the card and a modifier click doesn't.

## Impact

- `crates/ui/src/app.rs`: `entry_tooltip` becomes the slim track tooltip (entry-row hover). The
  record row loses its hover and its click opens the card; the app keeps which record's card is
  open, closes it when that record goes, and routes Esc, ← and → to it while it is open.
- `crates/ui/src/app/card.rs`: the card becomes an interactive area (play buttons, ‹ ›, a
  scrolling tracklist) instead of a tooltip.
- `crates/ui/src/format.rs`: `entry_details` is split into track details and record details.
  `record_counts` moves into the record details.
- `crates/ui/src/app/covers.rs` and `crates/dig/src/cover.rs`: a cover size (thumbnail or large)
  in the request and cache key, a `covers/large/` folder, and large requests only for the open
  card's record once it has shown for 250 ms.
- `crates/ui/src/playlist.rs` `Origin`: a new `cover_large` field, saved with the crate and
  defaulting to empty (old crates load unchanged).
- `crates/dig`: intake fills `cover_large` from Discogs collection, wantlist, search, release
  and master data. Bandcamp large addresses come from their thumbnails.
- Headless tests for the card and the slim tooltip. Spec scenarios in `playlist`, `album-entries`
  and `record-view` are rewritten.
- README (tooltip and cover description, cache folder, test count) and CHANGELOG.
- Nothing on the playback path. Covers stay on the low-priority cover thread at 4 per second.
