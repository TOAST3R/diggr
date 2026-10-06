## 1. Record artist on entries

- [x] 1.1 `Origin.artist` (saved, skipped when empty); `origin()` fills it from `RecordInfo.artist`; `records_to_backfill` also asks for records without it, and `backfill()` fills it from the cache; tests (compilation: same record artist on every entry, own artists unchanged; an older crate backfilled with no request; old crate files load)
- [x] 1.2 Record rows name `origin.artist` when set, else the first entry's artist; headless test ("Various – Night Moves")

## 2. Facet model

- [x] 2.1 `Facet` (`Style`, `Artist`, `Label`) with per-entry values; `Playlist.picked` per facet replacing `styles_on`; `set_pick` / `picked` / `clear_picks`; `filter(facet)` (gone values stop filtering); `Shown` checks every set facet; `clear_filters` clears all four; move the style tests over and add artist and label ones (exact credit match, compilation hidden, any-of, AND across facets and BPM)
- [x] 2.2 `counts(facet)`: records per value, most first then by name; saved `styles` / `artists` / `labels` selections (old files with `styles` load); timing test: picking a value on 5,000 entries < 16 ms in release (best of three), counting under the debug budget

## 3. Footer and lists

- [x] 3.1 Footer tiers after the BPM control: chips + ARTISTS + LABELS when all fit, else STYLES + ARTISTS + LABELS when they fit, else nothing; facets with fewer than two values left out; a button lit with its count while set, double-click clears it; per-facet count cache by crate and revision; headless tests at 700, 450 and 275 px (no overlap with the time box)
- [x] 3.2 One list popup for any facet (search per facet, counts, checkboxes, Clear), anchored at its footer button or at ☰; ☰ entries Filter by style/artist/label… and Show all records (replacing Show all styles), only in the Discogs crates; headless tests (☰ opens the artist list at 275 px, search "parrish", Show all records keeps the BPM range)
- [x] 3.3 Play and `P` under artist and label filters: next follows them, `P` on a hidden playing entry clears every filter; headless test

## 4. Docs and checks

- [x] 4.1 README (artist and label filters, the footer tiers, the ☰ entries, record rows naming the record artist, the test count), help panel
- [x] 4.2 fmt, clippy, the workspace tests (`--no-fail-fast`), the wasm check, `--bench` and `--startup-time`; try the filters in the app on the collection crate at classic and wide widths
