## Why

The wantlist and collection crates mirror Discogs, but they still behave like any other crate. Their menus offer Remove a track, Remove album and Pass, which mean nothing for a record you want or own: you can't remove one track from a record, and passing on a wanted or owned record does nothing. Meanwhile, the one removal that does matter, taking a record out of the Discogs collection, isn't offered at all. These crates are also the biggest ones (a collection of 1,000+ records), and the only way to narrow them is by BPM. A digger looks for "the Deep House and Minimal records" first, so the crate should filter by style.

## What Changes

- **Style filter** in the wantlist and collection crates:
  - the playlist footer shows the crate's styles after the BPM control and before the time readout, as chips you can turn on and off, sorted by number of records;
  - when the styles don't all fit, or there are more than 20, it shows one "STYLES n" button instead. The button opens a list with a search box, a checkbox and a record count per style, and Clear;
  - a record shows when it has any selected style (OR). With the BPM range, an entry must pass both. Nothing selected means no filter;
  - the filter is remembered per crate. Play order, previews, the title bar count and "k of N tracks" follow it, as they follow the BPM filter.
- **Trimmed menus** in the wantlist and collection crates:
  - **BREAKING:** Remove, Remove album and Pass are no longer offered on track rows or record rows there, and the Delete key removes nothing there. The main window says once how to remove a record instead;
  - the wantlist crate removes records only through Remove from wantlist (Y), which acts at once, as it already does. The "Remove N records from your Discogs wantlist?" confirmation guarded only Remove, Remove album and Delete in that crate, so it goes away.
- **Remove from collection…** in the collection crate's entry menu, for one record at a time (never a selection). After a confirmation, it removes **one copy**, the most recently added, from the user's Discogs collection. The local collection follows at once: when it was the last copy, the record's entries leave the crate and lose OWNED (unless another pressing of the same master is owned). A failed removal says why and offers Retry. Retrying is safe, because a copy already gone counts as removed.
- **No moving entries in:** **BREAKING:** the wantlist and collection crates no longer take entries from Send to crate, from drops on the sidebar, or from adding files, URLs or pages to them. Only the app's own Discogs actions fill them (Add to wantlist, Add to collection, sync, Refresh, connecting a token). Moving entries out and reordering inside them still work.
- Out of scope: a style filter in other crates, AND matching, filtering by genre or label, and removing several records from the collection at once.

## Capabilities

### New Capabilities
- `style-filter`: the style filter of the wantlist and collection crates, covering the footer chips, the STYLES list, OR matching, how it combines with the BPM filter, its per-crate memory, and how play follows it.

### Modified Capabilities
- `discogs-write`: the wantlist and collection crates don't offer Remove, Remove album or Pass, Delete doesn't remove there, and nothing is moved into them by hand. The wantlist crate's "Remove N records…" confirmation goes, since nothing can trigger it any more. A new Remove from collection requirement (one record, one copy, the most recent, with confirmation and retry).
- `discogs-collection`: the local copy follows the app's removals as well as its adds (the copy's instance is dropped, the count lowered, the release dropped when it was the last copy), so the next sync stays incremental.
- `dig-verdicts`: Pass isn't offered in the wantlist and collection crates (N still acts on the playing entry, as before).
- `record-view`: in the wantlist and collection crates, a record row's menu offers no Remove. Elsewhere it still does.

## Impact

- `crates/ui`:
  - `playlist.rs`: a per-crate style selection (saved), `shows()` combining BPM and styles, the styles in a crate with record counts;
  - `app.rs`: the footer chips and STYLES button with its list, the entry menu by crate kind, Delete, drops and Send to crate refusing the Discogs crates;
  - `app/digging.rs`: Remove from collection (menu item, confirmation, failure and retry);
  - `crates.rs`: refusing hand-made inserts into the Discogs crates.
- `crates/dig`:
  - `intake.rs`: a `Discard { release }` command: `GET /users/{u}/collection/releases/{r}` to find the newest copy and its folder, then `DELETE /users/{u}/collection/folders/{f}/releases/{r}/instances/{i}`, within the existing rate limit;
  - `collection.rs`: removing an instance from the cached collection.
- README (the style filter, Remove from collection, what the Discogs crates no longer allow, the test count), help panel.
