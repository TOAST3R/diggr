## Why

Sending a label to a crate misses records. On Analogical Force, AF060LP and every other double or triple LP (30 releases, 82 listing items) never arrive. Two problems combine here:
- **The vinyl check is wrong.** It doesn't recognise the count prefix of multi-disc formats (`2x12"`, `3xLP`).
- **Vinyl only throws records away.** It's on by default and drops everything else, including records that only exist digitally or on CD, which a digger still wants to hear and know about.

The user wants every record of a page in the crate. When a record exists on vinyl and digitally, the vinyl release should be the one that brings the tunes, because that's the release they want, collect and buy.

## What Changes

- **BREAKING: Vinyl only goes.** Every send keeps records in every format:
  - the switch goes from Options ▸ Discogs… (the saved setting is ignored) and from the browser extension's menu;
  - the bridge accepts and ignores a `vinyl_only` sent by an older extension.
- **Multi-disc vinyl is recognised.** A leading count (`2x12"`, `3xLP`, `2x7"`) no longer hides the vinyl format.
- **Vinyl first.** In a crate filled from a page (not the wantlist or collection crate), when a record exists on vinyl:
  - its tunes belong to the vinyl release;
  - a non-vinyl release of the same record (same master release, or, without one, the same catalog number and title) leaves the crate, handing over the tracks the vinyl release also has. Tracks it has that the vinyl doesn't leave with it;
  - the expansion queue fetches a vinyl release before its non-vinyl twin;
  - this holds whichever release is fetched first.
- **Records with no vinyl release stay.** For example, a digital-only or CD-only release.
- **Formats on entries.** Each Discogs entry carries its record's formats, grouped as Vinyl, File, CD, Cassette or Other:
  - non-vinyl rows (single-line and record rows) show a dim `FILE`, `CD`, `CASS` or `OTHER` mark;
  - the column layout gets a Format column, which can be sorted, shown or hidden like the others;
  - older crates get their formats from the disk cache when shown, with no request.
- **Format filter.** A fourth record filter (see `record-filters`), offered in every crate with Discogs entries whose records have at least two formats:
  - in dig crates the footer shows a FORMATS button (style, artist and label stay limited to the wantlist and collection);
  - ☰ ▸ Filter by format… opens it at any width;
  - Show all records clears it too.
- Out of scope: pairing a twin across labels (a vinyl reissue elsewhere); a master listed on an artist page whose main release isn't vinyl (it keeps expanding through its main release); ranking formats beyond "vinyl first"; changing the wantlist and collection crates.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `discogs-intake`: the Filters requirement loses vinyl only (skip passed stays). New requirements for vinyl-first twins, for formats on entries (including the multi-disc parse), and for queue order.
- `record-filters`: a Format filter, with its own scope (every crate with Discogs entries) and its footer button and ☰ entry.
- `playlist-columns`: the Format column.
- `record-view`: the format mark on record rows.
- `chrome-extension`: the page button's menu no longer has the vinyl only switch.
- `browser-bridge`: a send carries skip passed only; a `vinyl_only` field is accepted and ignored.

## Impact

- `crates/dig`:
  - `discogs/model.rs`: the format groups from a listing string and from a `formats` array (count prefix stripped); `Listed` and `Record` carry formats instead of a vinyl flag;
  - `intake.rs`: no vinyl partition or `Outcome::Excluded`; the pending queue orders a vinyl item ahead of its non-vinyl twin (same catalog number and title);
  - `jobs.rs` and `config.rs`: `vinyl_only` read and ignored (`serde(default)`);
  - `bridge/mod.rs`: the field optional and ignored.
- `crates/ui`:
  - `playlist.rs`: `Origin.formats`, the Format facet, and the format field for columns and sort;
  - `app/digging.rs`: formats filled and backfilled, and the twin rule in `dig_record` (hand-over and leave). No twin handling in the wantlist and collection crates;
  - `app.rs`: the format mark and column, and the filter controls in dig crates;
  - the Options ▸ Discogs… checkbox removed;
  - `columns.rs`: `Field::Format`.
- `extensions/chrome`: the vinyl only toggle and its storage removed.
- README (vinyl first, the format mark, column and filter, the extension menu, the test count), help panel.
