## Why

When no preview can be found for a track, the only sign is a greyed row saying "not found by search" in its tooltip. Early users can't tell whether Diggr is still looking, failed, or whether the record has no audio anywhere yet (often because it isn't released). And a new, empty crate says only "PASTE A DISCOGS LINK": users don't learn that Bandcamp works too, or that Juno, Bleep, Beatport and Traxsource come in through the browser extension.

## What Changes

- **Whole record not found → a blocking notice.** Once every source has been tried for every track of a record (YouTube search, the album upload, Bandcamp, store samples) and none plays, a dialog says so: which sources were tried and, for a record from this year or later, that it is probably not out yet and worth retrying later. Records found empty while it is open are added to it; it stays until the user closes it. Retry and Retry all re-run the search.
- **A clicked track not found hands over**: the next track of its record is searched for (or downloaded) and plays, without another click, until one is found or the record runs out.
- **Search the other store by hand**: "Search on Bandcamp" for a Discogs record, "Search on Discogs" for a Bandcamp one, in the notice and in a track's right-click menu, opening that store's search in the browser.
- **A track being searched says so**: its row ends with a pulsing "SEARCHING…" while YouTube is searched for it (until now it looked like any track waiting), and its tooltip says "searching YouTube…".
- **Record partly found → no dialog**: its record row says how many tracks have a preview ("2/5 previews"), and its menu offers Retry missing tracks.
- **The empty crate explains how to fill it**: Discogs and Bandcamp as clickable store names in their brand colours with "paste a link here", then Juno, Bleep, Beatport and Traxsource "with the browser extension", a link to set the extension up, and the existing drop-files and help lines.

## Capabilities

### New Capabilities
- `not-found-notice`: when a record counts as fully tried, the blocking notice, its chaining, the partial-record count, retries.

### Modified Capabilities
- `preview-search`: A found track says so: also while its search runs (row and tooltip), and the worker says when a search starts and ends.
- `playlist`: Empty crate hint: new content, store links, extension link.

## Impact

- **`crates/ui`:** `app/digging.rs` (`PreviewEvent::NotFound`, `dig_album_fallback`, `retry_of`), `app/bandcamp.rs` (`bandcamp_fallback`, its `tried` set), a record-level "sources pending" check, the dialog beside the other dig dialogs (`dig_asking`), the record row in `app.rs`, `empty_crate_hint`.
- **Skin:** brand colours for six store names as constants (no logos, no new images).
- **README:** the notice, the hint, test count.
