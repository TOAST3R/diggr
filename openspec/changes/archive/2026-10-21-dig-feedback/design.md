## Context

A track of a Discogs record without its own clip waits to be searched; searching happens only near the playhead (`preview-search`: the armed, playing and next 3 entries). A miss sets `Unavailable(NotFound)`, then `dig_album_fallback` may add the record's full-album upload and `bandcamp_fallback` queues a Bandcamp look (once per entry, `bandcamp.tried`). A store sample can later replace a "no clip"/"not found" entry. So a record's outcome arrives track by track, over minutes, while the user listens. Blocking dialogs register in `dig_asking()`, which makes `handle_keys` return early; playback is untouched by dialogs.

## Goals / Non-Goals

**Goals:**
- Tell the user, once per record, when nothing of it can be played, and why that may be temporary.
- Never fire early: only when no source is still pending for any of its tracks.
- Show partial results without interrupting.

**Non-Goals:**
- Automatic retries on a schedule (later change).
- New sources or better search matching.
- Logos or images of the stores.

## Decisions

### D1. When a record is "fully tried"
A record (its `record_key`) is fully tried when each of its entries in the crate is either playable (has audio, or is waiting for a download that will give audio) or `Unavailable(NoClip | ClipFailed | NotFound)`, and none has a fallback pending: no album upload not yet added, no Bandcamp look queued or running, no search queued or running. `AlreadyInCrate` and passed entries are left out. A record is **not found** when fully tried with no playable entry; **partly found** when fully tried with some.
The check runs where outcomes arrive (`NotFound`, `Done`, `ClipFailed`, the Bandcamp fallback's end), only for the record touched, so it costs one record's entries per event.

### D2. One blocking dialog, chained
The first not-found record opens "No preview found"; later ones are appended while it is open; it closes only by Close, Esc or Enter. It lists per record: artist – title (year), the sources tried ("YouTube, the album upload, Bandcamp"), and Retry. Footer: Retry all, Close. Playback continues (the skipped tracks are already skipped by play order).
*Alternative:* a non-blocking card. Rejected by the user: they want it noticed.

### D3. "Probably not out yet" only when it may be true
If the record's year is the current year or later, or unknown on a Bandcamp pre-order ("no clip (not streamable)"), the line says "Probably not released yet: try again after its release". Otherwise "No preview anywhere yet". The dialog's header text says previews often appear after release.

### D4. Shown once per record per session
A record shown in the notice is not shown again in the session unless the user retries it and it fails again. Records already not-found when a crate loads at launch don't raise the notice (no outcome arrived).

### D5. Retry reuses `retry_of`
Retry re-queues each entry's `retry_of` (clip download or search), clears its Bandcamp `tried` mark, and removes the record from the notice. Searches still only run near the playhead, so a retried record's entries become "to search" and are tried when reached; "Download all tracks" in a label crate still forces them.

### D6. Partly found
In the grouped view the record row's dimmed second line ends with "· ‹n›/‹m› previews" when 0 < n < m (record rows have no tooltip; the missing tracks' own rows say "not found by search"). The right-click menu of the record and its tracks gets "Retry missing tracks".

### D7. Empty crate hint: text links in brand colours
Each store name is drawn in the skin font in the store's brand colour as a link (hand cursor, underline on hover), opening the store's home page. Using names, not logos, keeps the pixel skin consistent and stays clear of logo-usage rules. "Set up the extension" opens Options ▸ Browser… (the pairing dialog), which already explains loading the extension. When the list is too short for every line, the hint falls back to today's four lines.

## Risks / Trade-offs

- [The dialog pops up mid-listening, since searches run near the playhead] → it never stops playback, takes Esc/Enter, and chains instead of stacking.
- [A record counted fully tried while a source is still pending] → D1 counts pending work explicitly; tests drive each fallback's order.
- [Brand colours unreadable on the playlist background] → each colour is checked for contrast against `pl_bg` in a test and lightened when below 3:1.

## Open Questions

- Should the notice also offer the record's "+ CART" (search on stores, `cart-everywhere`) for records that aren't out yet? Left out until `cart-everywhere` lands.
