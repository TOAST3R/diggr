## Context

- **Discogs entries:** they carry an `Origin` (page, release, master, label, catno, year, side, clip, for-sale). `RecordInfo` already holds the release `title`, but nothing passes it to `Origin`.
  - Release and master JSON is cached on disk. It includes `images[]`, each with a signed `uri150` thumbnail address.
  - Label and artist listing items carry a `thumb`.
- **Local files:** the metadata worker produces `audio::TrackInfo`, which already has `album`. The playlist keeps only the artist, title and duration.
- **Tooltip:** `format::entry_details` builds the entry tooltip only for the hovered row. The spec forbids reading files or making requests while building it.
- **Context menu:** right-click on an unselected entry first makes it the selection, and Remove and Send to crate act on the selection.
- **Images:** the `image` crate is compiled with PNG only, for the skin. `ureq` is used in `dig`. `transport.rs` only reaches `api.discogs.com`.

## Goals / Non-Goals

**Goals:**
- See which record each track belongs to, in the row and the tooltip.
- Act on a whole record in one move.
- See the sleeve without leaving the player.

**Non-Goals:**
- A maximum or median price (not in the Discogs API; scraping is out).
- Covers for local files (embedded art) — possible later; Discogs only for now.
- Covers anywhere but the tooltip (rows, main window).
- Undo for Remove.

## Decisions

**1. What an entry's album is.**
- A Discogs entry: `Origin.album`, the release's title, or the master's own title for a master. It's saved with the crate and omitted when empty.
- A local file: `Entry.album`, from its album tag, saved with the crate. Tags never overwrite an origin, as now.
- Older Discogs entries without an album get theirs from the cached release or master JSON when their crate is shown. A "listed" entry gets its album from the listing item's `title`. None of this makes a request.

**2. Which entries share an album (the album key).**
- **A Discogs entry:** `Release(id)` when it has a release, else `Master(id)`.
  - A master's entries all carry its main release, so they group together.
  - A different pressing of the same master is a different album, which is the useful answer when buying.
  - "Listed" and "no clip" entries belong to their record's album too.
- **A local file:** `Local(artist folded to lower case, album folded to lower case)`. Using the artist as well keeps two "Greatest Hits" albums apart. A file without an album tag has no key.
- Only entries of the shown crate are considered.

**3. The row.**
- Single line: "N. (catno) Artist: Title · Album (T BPM)". The " · Album" part is left out when there is no album or when it equals the title (case-insensitive, trimmed).
- When the row is too narrow, it is clipped on the right as now. The album comes after the title, so it is cut before any of the title (the BPM, last, goes first).
- Only the rows and the tooltip's first line carry the album. Messages and the main window's title line keep the shorter name.
- Columns: `Field::Album`, between Title and BPM, shown by default and hideable like the others. In columns, the Title cell doesn't repeat the album. It sorts as text (case-insensitive), and empty albums go last.

**4. The right-click tint.**
- While an entry's context menu is open, every other entry of the shown crate with the same album key is drawn with a tint: the selection colour at about 35% opacity, under the text.
- The selection does not change, so the existing "right-click outside the selection" rule holds.
- The tint clears when the menu closes.
- Menu, after Remove:
  - **Remove album (N tracks)**, where N counts every entry with the key, visible or not. It removes them all, including the right-clicked one. If the playing entry is among them, it keeps playing until its end, as with Remove.
  - **Select album**: the selection becomes exactly the album's entries, and the cursor goes to the right-clicked entry.
  - Both are absent when the entry has no album key, and disabled when the album has only that one entry.

**5. Covers.**
- **Source:** `images[0].uri150` from the release JSON (a master uses its own `images`, else its main release's). A listed entry uses the listing item's `thumb`. The address is stored as `Origin.cover`.
- **Fetching:** a `CoverFetcher` in `dig` on one low-priority thread.
  - Requests go to the image host only, never to `api.discogs.com`, so they don't use the API's 60-per-minute budget. They are still paced: at most 4 per second, one at a time.
  - Each image is fetched once, decoded (JPEG, WebP or PNG as served), scaled to at most 150 px, and stored as PNG under `<cache>/covers/<release or master id>.png`.
  - A failed fetch is remembered for the session and not retried until the next launch.
- **The tooltip:**
  - It asks a UI-side `CoverCache` for the entry's cover. That only looks in memory: a small LRU of up to 48 textures.
  - On a miss, the row asks the fetcher for that one cover, once the pointer has rested on it for 250 ms. A request that is already waiting is replaced by the newer one, so sweeping the pointer down a list fetches only where it stops.
  - The fetcher checks the disk first. When the cover arrives, the UI repaints and the tooltip shows it.
  - Until then, the tooltip shows the same layout with an empty 96 px frame, so it doesn't jump.
- **Spec rule:** the tooltip spec's "building the tooltip SHALL NOT read files or make requests" stays true. Files are read and requests made only by the fetcher's thread.
- **Prefetch:** none in this change. Covers load only on hover.

**6. Dependencies.**
- `image` gains `jpeg` and `webp` (decoding only).
- The fetcher uses `ureq`, which `dig` already uses.
- All of this is native only, behind `cfg(not(target_arch = "wasm32"))` like the rest of digging.

## Risks / Trade-offs

- [The image host turns out to be rate limited] → the fetcher is already paced and single-threaded. A 429 makes it back off for the session, and the tooltip simply shows no cover.
- [Signed thumbnail addresses expire] → when an address fails with 403 or 404, the cover is refetched once using the address in a fresh copy of the release JSON (one API request, through the normal limiter), and only for the hovered entry.
- [Long rows with album names] → the album is cut first and is always in the tooltip.
- [Remove album takes more than intended] → the item states its count. Scattered entries of the album (after a sort) are tinted, so they're visible when on screen.
- [Two local albums with the same artist and name] → treated as one album. Acceptable.
- [Disk use] → covers are about 10 KB each, so a 5,000-record dig is about 50 MB. The covers folder is part of the cache dir, so it is safe to delete.
