## Context

- **The list today:** one uniform row per entry, `row_h` points high, drawn in the single-line format or, from 480 points wide, in columns (`playlist_section`, `app.rs`).
  - Scrolling counts whole rows (`pl_scroll`, `scroll_rows`).
  - The keyboard cursor, selection, drag-to-reorder and the BPM filter all work on entry indices.
  - Shown entries come from the filter (`playlist-filters`).
- **Albums:** `Playlist::album_of(id)` and the album key: release, else master, else lower-cased local artist + album (`album-entries`).
- **Covers:**
  - `CoverCache` in `app/covers.rs` keeps up to 48 textures and asks after a 250 ms rest.
  - `CoverHandle` in `dig/src/cover.rs` has one low-priority thread. A newer request replaces a waiting one. Requests are paced to ≥ 250 ms apart and go only to the image host.
  - Files live in `<cache>/covers/<key>.png`, ≤ 150 px.
- **Title bar (classic 275 px):** `pl_max` (⇔) is at x 250 and `pl_close` at 262, both 9×9 at y 6. The crate name is drawn shortened to fit what's left. The skin is generated (`skin/generate.rs`, `cargo run -p ui --bin skin-gen`), and a test checks it is up to date.
- **Keys:** ←/→ seek ∓5 s whatever has focus. ↑/↓ move the playlist cursor while the playlist has focus. Shift+P maximizes. G and Shift+G are unused.
- **Crate kinds:** `discogs-write` introduces `CrateInfo.discogs: Option<Wantlist | Collection>`. Before it, only `collection: bool` exists.

## Goals / Non-Goals

**Goals:**
- One row per record with its cover, opening to show its tracks, in any crate, remembered per crate.
- No second, hidden play order: what you see is what plays.
- Fetch covers for what's in view only, at the existing pace. No effect on playback or launch.
- Drawing and hit-testing a 5,000-entry crate stays within the frame budget: the rows are built when the crate changes, not every frame.

**Non-Goals:**
- Embedded art for local files.
- A cover grid, or grouping by label or artist.
- Remembering which records are open across launches.
- Changing the flat view.

## Decisions

**1. Gather, don't permute.**
- Turning grouping on reorders the crate once, stably: entries keep their order, except that each album's entries move up to follow that album's first entry. Entries without an album key stay where they are.
- From then on, the crate's order *is* the grouped order, so next, previous, shuffle, pre-warm, previews ahead, saving and export need no change.
- While grouped:
  - an entry added whose album is already in the crate is inserted after that album's last entry (sends, intake and drops);
  - sorting sorts, then gathers again: records are ordered by their best-placed track, and tracks within a record follow the sort.
- Turning grouping off keeps the order.
- It's like a sort, so the header's sort mark clears.
- **Alternative considered:** a display permutation over an untouched crate order. It was rejected because every play path (next, shuffle, pre-warm, previews, the filter) would need to learn a second order, and what plays next would no longer match what's on screen.

**2. A row model, rebuilt on change.**
- `Rows` is a `Vec<Row>` with `Row::Record { key, first, len, shown }`, `Row::Track { idx, child }` and `Row::Single { idx }`.
- It's built from the shown entries, the per-crate open set and the grouped flag.
- It's rebuilt when the crate's revision, the filter or the open set changes, and is cached by those three. `Playlist::rev()` is bumped by every change to the entries, their order, their albums or the filter (selection and cursor don't count).
- Additions while grouped (sends, digs, tags read) are placed by `Playlist::settle()`, which the app calls once a frame for the shown and playing crates; toggling and sorting gather at once.
- A single-entry album is a `Single` drawn as a record row, so it plays directly.
- **Heights:** a record row is 2 × `row_h` and a track row is 1 × `row_h`. Scrolling and paging count in `row_h` units, and `pl_scroll` becomes a unit offset into `Rows`, so trackpad accumulation stays the same.
- **Hit-testing** is a binary search over the cumulative heights.

**3. The record row.**
- **Left:** the cover, a square of the row's height (26 points at 1×) with a 1-point inset, or an empty frame while it loads.
  - A local album has no cover and shows a record icon.
  - A missing cover (no image, or failed) shows the same icon.
- **Line 1:** "⏵ Artist – Album" (⏷ when open), plus "OWNED" and "★" as in entry rows. Marks use glyphs the bundled fonts have (a test checks every string the UI draws).
- **Line 2, dimmed:** "catno · year · N tracks · for sale", or "2 of 4 tracks" under a filter.
- **When the record holds the playing entry:** line 2 becomes "⏵ A2 Tidepool" (⏸ when paused), and the row takes the highlight colour.
- **In columns:** the record row spans the whole width (no cells), and track rows use the columns as they are (no indent); in the single-line layout they're indented past the cover.
- **The tooltip** of a record row is the tooltip of its first entry, with its album details and cover.
- **Styles:** `Origin.styles` (from the release or master data's `styles`, else `genres`; filled for older crates from the disk cache, never a request) is drawn right-aligned on line 1, taking at most 45% of the room; the name is cut before it.
- **No header while grouped:** record rows span the width, so the column header's labels would name nothing; its row goes to the list.
- **Sidebar counts:** a grouped crate shows its records (albums, and entries of no album), counted once per playlist revision and saved in the index as `records` for crates not loaded.

**4. Opening and closing.**
- ⏵ / ⏷ is drawn at the left of line 1. Clicking it, or pressing Space on the cursor's record row while the playlist has focus, opens or closes the record; Space on one of an open record's track rows closes it.
- The open set is per crate and kept for the session only. Records start closed.
- A newly added record starts closed, except that the record of an entry being armed or played opens.
- **Alternative considered:** →/← to open and close, as in tree views. It was rejected because ←/→ are the seek keys everywhere in the player.

**5. Selection, menus, play and drag.**
- **Clicking a record row** selects all its entries and puts the cursor on the record row. Shift and Cmd/Ctrl extend or toggle by whole records.
- **Right-click on a record row** opens the entry menu as for its first entry, with the record selected. Remove, Send to crate and the `discogs-write` items then act on the whole record. Remove album and Select album are not shown there, because the row already is the album.
- **Double-click or Enter** on a record row plays (or arms) its first playable track.
- **Dragging a record row** moves all its entries as a block, to before or after another record row. Dropped on a sidebar crate, it sends them all. Dragging a track row reorders it within its record only: a drop outside its record is refused, with the insertion line not drawn, so records stay contiguous.
- **Keyboard:** ↑/↓ move over `Rows`, PgUp/PgDn by page and Home/End to the ends. Shift extends by row, and a record row counts as all its entries.

**6. P and following the playing track.**
- P shows the playing crate and scrolls to the playing entry's row: its track row when the record is open, otherwise the record row.
- When the playing entry changes and the previous one was in view, the list scrolls by the least amount to show the new row.
- When the next track is in the same closed record, nothing scrolls, because the record row stays in place and only its line 2 changes.

**7. The toggle.**
- **▤ button:** a new skin region `pl_group` at x 238, y 6, 9×9, left of `pl_max`, drawn with `btn_group` (normal, pressed) and `btn_group_on` (lit, pressed), generated by `skin-gen`. The crate title already keeps 40 px clear at each end of the bar, so it needs no change.
- **Shift+G** does the same while the player or the playlist has focus.
- **≡ menu:** a "Group by record" checkbox.
- **Saved state:** `CrateInfo.grouped: Option<bool>`, saved in the index. `None` means the default: grouped for a crate whose `discogs` kind is set (wantlist or collection), or with the older `collection: true`; flat otherwise.
- **First use:** the first time a crate is grouped (by default or by the toggle), it is gathered (decision 1). Gathering a 5,000-entry crate SHALL take < 16 ms. It's a stable partition by album key using a hash map of first positions.
- **Maximized strip:** unchanged. ▤ lives in the playlist's own title bar, which is still drawn when maximized.

**8. Covers for rows in view.**
- **The UI side:** each frame in which the set of record rows in view changes, `CoverCache::want(keys)` sends the worker the ordered list of covers to load, top to bottom: those in view without a texture, not failed, at most 40.
- **The worker:** `CoverHandle::want(Vec<(RecordKey, url)>)` replaces its waiting list. The worker takes from the front.
  - A cover already on disk is loaded without pacing.
  - A network fetch keeps `MIN_GAP` (≥ 250 ms) and the 429 back-off.
- The hover request becomes `want([hovered])` placed at the front, so a rested hover still wins.
- **Textures:** `KEEP` grows from 48 to 128, and the least recently drawn are evicted first. A 128 × 150 px RGBA set is about 11 MB.
- **Unchanged:** the image host only, no API budget, failures not retried before the next launch, the disk cache.
- **Alternative considered:** prefetching covers for the whole crate. It was rejected because a 1,000-record collection would take minutes of steady fetching at 4 per second, mostly for records never scrolled to.

**9. The filter.**
- `Rows` is built from the shown entries only.
- A record with no shown entry has no row.
- Line 2 says "k of N tracks" when some are hidden, and the counts include only shown entries.
- Select all, Invert selection and the cursor work over shown entries, as today.

## Risks / Trade-offs

- [Gathering changes a crate's order when the toggle is first turned on, and turning it off doesn't put it back] → This matches sorting, which users already know. The ≡ menu item's tooltip says it groups each record's tracks together.
- [Two-unit rows complicate scrolling, paging and "least amount" scrolling] → Everything is done in `row_h` units over `Rows`. Unit tests cover the scroll maths with mixed heights.
- [Covers in view at 4 per second feel slow on a first scroll through a big collection] → The disk cache makes it a one-time cost, and empty frames keep the layout stable while covers arrive.
- [Texture memory with 128 covers] → About 11 MB, bounded.
- [The ▤ button crowds the crate title] → The title already keeps 40 px clear at each end; the button sits inside that.
- [Drag rules (a track can't leave its record) may surprise] → No insertion line is drawn outside the record, so the refusal is visible. Send to crate still copies single tracks anywhere.

## Migration Plan

- `grouped` is a new optional index field. Old indexes load with `None`, which applies the per-kind default: the collection crate (and the wantlist crate, once `discogs-write` is built) open grouped and are gathered on first show.
- **Rollback:** older builds ignore `grouped`. Gathered crates stay in their gathered order, which is a valid order.

## Open Questions

- Should open records be remembered across launches? The design says no, to keep the index small.
- Should a record row's tooltip show the whole tracklist with each track's state? The design keeps the first entry's tooltip.
