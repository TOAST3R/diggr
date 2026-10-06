## 1. Formats

- [x] 1.1 `model.rs`: `Format` groups; parse a listing string (strip `\d+x`, skip descriptors, Other when nothing matched) and a `formats` array (skip Box Set / All Media); `Listed.formats` and `Record.formats` replace `vinyl` (old `jobs.ron` with `vinyl` loads); tests (2x12", 3xLP, 17xFile FLAC, CD+Vinyl, Box Set, the Analogical Force listing strings)
- [x] 1.2 `Origin.formats` filled from the record (placeholders from the listing) and backfilled from the cache; tests (old crate backfilled without requests)

## 2. No vinyl only

- [x] 2.1 Remove `vinyl_only` from `DigSettings`, `Filters`, the intake partitions and `Outcome::Excluded`; the bridge accepts and ignores `vinyl_only`; Options ▸ Discogs… loses the checkbox; old settings and jobs files load; tests (a label with CD-only releases keeps them; an older extension's send accepted)
- [x] 2.2 Chrome extension: remove the toggle, its storage and the field in sends; update the extension's checklist in README

## 3. Vinyl first

- [x] 3.1 Queue order: a non-vinyl listed item goes right after a pending vinyl item with the same catalog number and title (on listing and on focus reorder); intake tests
- [x] 3.2 `dig_record` twin rule outside the wantlist and collection crates: a non-vinyl record with a vinyl twin (same master, else catno + title) adds nothing and its placeholder leaves; a vinyl record takes over the shared clips of non-vinyl twin entries (origin swapped in place, ids kept) and the twin's other entries leave (the playing one once it stops); headless tests (vinyl first, digital first while playing, digital-only stays, wantlist untouched)

## 4. Format in the UI

- [x] 4.1 Format mark (FILE / CD / CASS / OTHER) on single-line and record rows for records with formats but no vinyl; `Field::Format` column (default on, sort Vinyl > File > CD > Cassette > Other > unknown, saved column settings without it get it); headless and unit tests
- [x] 4.2 `Facet::Format`: values from `Origin.formats`, saved `formats` picks, offered in any crate with Discogs entries and two formats (style/artist/label stay in the Discogs crates); FORMATS button and ☰ Filter by format…; Show all records clears it; headless tests (dig crate shows only FORMATS; picking Vinyl hides digital records and play follows)

## 5. Docs and checks

- [x] 5.1 README (every format, vinyl first, the format mark, column and filter, no vinyl only in Options and the extension, the test count), help panel
- [x] 5.2 fmt, clippy, the workspace tests (`--no-fail-fast`), the wasm check, `--bench` and `--startup-time`; send Analogical Force again and check AF060LP and the double LPs arrive as vinyl, with digital twins gone and digital-only records marked FILE
