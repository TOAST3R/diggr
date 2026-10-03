## 1. Order and state

- [ ] 1.1 `Playlist::gather()`: a stable reorder by album key (each album after its first entry, others in place), < 16 ms for 5,000 entries; unit tests (scattered album, no-album entries keep their place, the playing entry keeps playing), plus a timing test
- [ ] 1.2 Insertion while grouped: an entry whose album is in the crate goes after that album's last entry (sends, intake replacing "listed" entries, drops); sorting a grouped crate gathers after sorting; tests
- [ ] 1.3 `CrateInfo.grouped: Option<bool>`, saved in the index; the default by crate kind (Discogs wantlist/collection grouped, older `collection: true` too); gather on first grouping; tests with `TestDir` (remembered across restart, old index loads)

## 2. Rows

- [ ] 2.1 `Rows` model (`Record`, `Track`, `Single`) built from shown entries, the open set and the grouped flag, cached by crate revision, filter and open set; cumulative heights in `row_h` units, binary-search hit test; unit tests (single-entry album, filter hides a whole record, "k of N"), < 5 ms for 5,000 entries
- [ ] 2.2 Scrolling in units over `Rows` (trackpad accumulation, PgUp/PgDn by page, least-amount follow of the playing row, P); unit tests with mixed heights

## 3. Drawing and interaction

- [ ] 3.1 Draw record rows (cover square or empty frame or record icon, ▸/▾, line 1 with OWNED and ✓, dimmed line 2, playing line "▶ A2 Tidepool" and highlight), track rows indented, column layout with spanning record rows; tooltip from the first entry; headless render tests
- [ ] 3.2 Open/close: ▸ click, Space on the cursor's record row, per-crate session open set, the playing or armed record opens; ←/→ still seek; headless tests
- [ ] 3.3 Selection and menus: a record-row click selects the record, Shift and Cmd/Ctrl by whole records; right-click opens the entry menu with the record selected (no Remove album and Select album); double-click and Enter play the first playable track or arm; headless tests
- [ ] 3.4 Drag: record rows move as a block between records and onto sidebar crates; track rows only within their record (no insertion line outside); headless tests
- [ ] 3.5 Keyboard over rows (↑/↓, PgUp/PgDn, Home/End, Shift extends, a closed record is one step); headless tests

## 4. Toggle

- [ ] 4.1 `pl_group` sprite (normal, pressed, lit) in `skin/generate.rs` at (238, 6, 9×9); regenerate `assets/skin/default/` with `skin-gen`; the crate title fits the narrower space (update the `crate_title` expectations)
- [ ] 4.2 ▤ button, Shift+G and the ≡ "Group by record" checkbox, all the same action; no playback interruption; headless tests

## 5. Covers in view

- [ ] 5.1 `CoverHandle::want(list)`: an ordered waiting list replaced on each call, disk hits without pacing, network fetches at ≥ 250 ms with the 429 back-off; the hover request goes first; tests with the fake image transport (top-down order, out-of-view dropped, hover first)
- [ ] 5.2 `CoverCache`: send the in-view record rows' covers (≤ 40, top first) when the set in view changes; `KEEP` 128 with least-recently-drawn eviction; headless test (no file read or request while drawing)

## 6. Docs and checks

- [ ] 6.1 README (grouped view, ▤, Shift+G, Space, covers on rows, the test count), help panel (`help.rs`)
- [ ] 6.2 Run the app on a big collection crate to try grouping, opening, covers while scrolling and playback through records; fmt, clippy, the workspace tests, the wasm check, `--click-test`, `--startup-time` and `--bench`
