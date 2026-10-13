## 1. Year as a record filter

- [x] 1.1 Add `Facet::Year` (between Label and Format), its values the record's year as text from a static table, so filtering allocates nothing
- [x] 1.2 Save and restore the picked years per crate (`SavedPlaylist.years`, serde default, so old crate files load unfiltered)
- [x] 1.3 Unit test in `playlist.rs`: years counted by record, any picked year shows, an unknown year shows only while off, round trip, old file, the text table's bounds

## 2. Year grid in the filter panel

- [x] 2.1 YEARS button and YEAR tab from the facet code; the tab draws a five-column grid from the earliest year (rounded down to a multiple of 5) to the latest, "year / count" per cell, empty years "·" and disabled, then Clear
- [x] 2.2 A click toggles a year; a Shift-click picks every year with records from the last clicked one, unpicking none
- [x] 2.3 Headless test: YEARS opens the grid, a click picks, a Shift-click picks a run, × clears

## 3. Record filters in every crate

- [x] 3.1 `offered_facets()`: offer every facet with at least two values in the shown crate; update its doc comment and `is_discogs`'s
- [x] 3.2 Row menu: "Only this …" wherever that facet is offered; "Search ‹label›"/"Search ‹catno›" only where no label filter is offered
- [x] 3.3 Update the headless tests that asserted the old gating: a label crate offers ARTIST and STYLE but not LABEL; a crate of one style, artist and label offers only FORMAT; a crate with two labels offers Only this label
- [x] 3.4 Test: in a crate with styles on only some records, picking a style shows the fetched matches, and a record whose styles arrive later appears and is counted

## 4. Docs and checks

- [x] 4.1 README: the year filter and its grid; style/artist/label filters in every crate; the test count
- [x] 4.2 Run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo check -p audio -p platform --target wasm32-unknown-unknown`
- [x] 4.3 At archive: update the Purpose lines of `record-filters` and `style-filter`, which still say "the Discogs wantlist and collection crates"
