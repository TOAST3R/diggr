## 1. Numbers

- [x] 1.1 `crates/ui/src/insights.rs`: `Stats::of(&Playlist)`: records by album key, vinyl only ("Vinyl" in formats), years, styles, style × decade heat (8 largest styles), top 10 labels and artists (no "Various" / "Unknown Artist"), value (main currency sum, priced, other currencies, median, five dearest with their entry, rare ≤ 1 for sale), BPM buckets of 10 and tracks analysed
- [x] 1.2 Unit tests: vinyl only ("Vinyl, CD" counts, CD-only and File-only don't), one record many tracks counts once, decades, styles order, value and median with a USD record apart, rare, BPM buckets
- [x] 1.3 A timing test: `Stats::of` on a 5,000-record, 40,000-entry crate within 16 ms in an optimized build (100 ms in a debug test build)

## 2. Label totals

- [x] 2.1 `dig`: `Command::LabelTotals(Vec<(String, u64)>)` and `Event::LabelTotal { name, id, total }`: the label id from a release of it (cache first), the total from `/labels/{id}/releases?per_page=1`, cached 7 days
- [x] 2.2 Intake tests with the fake transport: id from a cached release, one request per total, a second ask within 7 days sends none, a name matching no label of the release answers nothing for it

## 3. Window

- [x] 3.1 "Open Insights" in the collection crate's context menu (sidebar and title-bar menu); `insights_open` on the app; Escape and × close
- [x] 3.2 Recompute `Stats` when the window is open and the collection crate's revision changed; ask for label totals for the top labels when they change (with a token)
- [x] 3.3 `insights::window`: title with the count; Years (bars per year, decade bands), Styles, Style × decade heatmap, Top labels (n of N, Follow / Following), Top artists, Value, Tempo; skin font and LCD colours; hover texts; empty parts say so; two columns, one when narrow
- [x] 3.4 Actions: Filter (clear, then set picks; decades and heat cells to their years), ShowRecord (select, scroll into view), Follow (follow the label by id), Close
- [x] 3.5 Motion: the bars grow in over 0.7 s when the window opens (eased out); every bar, row, band and cell lights up and fades with the pointer (brighter bar, name in the title colour, soft row background, heat cells outlined with their count, year and tempo bars widen and show their number). Checked by hand, as asked for the blink

## 4. Tests

- [x] 4.1 Headless: Open Insights is on the collection crate's menu only; the window opens with the vinyl count
- [x] 4.2 Headless: a style bar shows the collection filtered to that style; the 90s band sets the year filter to the 90s years present; a dear record is selected and in view
- [x] 4.3 Headless: a label row reads "n of …", then "n of N releases" after the fake total; Follow makes the label crate and the row reads "Following"

## 5. Docs and checks

- [x] 5.1 README: Insights (how to open it, what it counts, clicks filter, label loyalty) and the Tests section (new areas, count)
- [x] 5.2 CHANGELOG `## Unreleased` entry, following the `changelog` skill
- [x] 5.3 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo check -p audio -p platform --target wasm32-unknown-unknown`

## 6. Asked along the way

- [x] 6.1 The collection and wantlist crates offer no Rename crate or Delete crate (sidebar and title-bar menus), and renaming or deleting them is refused whatever asks (the Delete key); a headless test
