## 1. Tempo source

- [x] 1.1 Add `SongScore::dominant_bpm()` (longest tempo segment) with unit tests, including a score with no segments
- [x] 1.2 Add `format::dj_bpm(raw) -> u16` folding into 88–176 and rounding, with unit tests (87 → 174, 280 → 140, 124.3 → 124, 176 → 176)

## 2. Entry field and format

- [x] 2.1 Add `bpm: Option<u16>` (folded) to `Entry` and `SavedEntry` (`#[serde(default)]`, skipped when `None`); test that an old crate file still loads
- [x] 2.2 Rewrite `Entry::display_name()` to `(catno) Artist: Title (N BPM)` with optional parts; update the playlist tests
- [x] 2.3 Use it in `format::title_line`, and remove the catno from `format::origin_details`; update `the_title_line_keeps_an_origin_entrys_own_names` and the format tests

## 3. Filling the BPM

- [x] 3.1 The prepare worker (`crates/dig/src/prepare.rs`) reports `(clip, dominant bpm)` when a preview's score is ready; the app sets it on every entry with that clip
- [x] 3.2 When the playing track's score becomes available (cache hit or finished analysis), set the playing entry's BPM
- [x] 3.3 The metadata worker looks up `ScoreCache` by content hash for local files and reports a cached BPM; test with a `TestDir` cache
- [x] 3.4 Headless UI test: a prepared preview's entry shows "(124 BPM)" and the crate saves it

## 4. Docs and checks

- [x] 4.1 Update README (playlist format, BPM) and the test count
- [x] 4.2 Run fmt, clippy, the workspace tests and the wasm check
