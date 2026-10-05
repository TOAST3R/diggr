## 1. Style filter model

- [x] 1.1 `Playlist`: `styles_on: BTreeSet<String>` (saved, `#[serde(default)]`), `style_filter()` (the selection ∩ the crate's styles, `None` when empty), `toggle_style`, `clear_styles`; `shows(e)` = BPM ok && style ok (OR, an entry with no style hidden while a style is selected); unit tests (any-of, with BPM, a style gone stops filtering, old crate files load)
- [x] 1.2 `crate_styles() -> Vec<(String, usize)>`: per-record counts over the whole crate, sorted by count then name, cached by revision like `tempo_span`; unit tests and a timing test (5,000 entries, a selection change rebuilds the shown list in < 16 ms)
- [x] 1.3 Check that next/prev, shuffle, pre-warm, previews ahead, `Rows` ("k of N tracks"), the title bar count and P follow the combined filter; tests for next under a style filter and a hidden record

## 2. Style filter control

- [x] 2.1 Footer layout after `bpm_control`: measure the chips against the room up to the time box; chips (lit/dim, skin font) when all fit and ≤ 20, else "STYLES n ▾"; only in the wantlist or collection crate with ≥ 2 styles; headless tests at 275 and 700 px (no overlap with the time box, chips vs button, other crates show nothing)
- [x] 2.2 Chip click toggles, double-click clears; the "STYLES" popup with a search field (case-insensitive contains), checkboxes with record counts, scrolling, Clear; ≡ ▸ Show all styles; headless tests (search "house", Clear, toggle)

## 3. Menus and keys in the Discogs crates

- [x] 3.1 `entry_menu` / `dig_entry_menu` by crate kind: no Remove, Remove album, Pass or Undo pass in the wantlist and collection crates (track and record rows); in the collection crate, no Add to wantlist, "In collection" or Add to collection; headless tests for both crates and a dig crate (unchanged)
- [x] 3.2 Delete and Backspace remove nothing in those crates, with a once-per-session hint; remove `dig_before_remove`, `UnwantConfirm` and the `confirm-unwant` modal; update the wantlist-crate tests that relied on it (Remove from wantlist (Y) on a record row still leaves the crate at once)

## 4. No hand edits in

- [x] 4.1 `Crates::accepts_manual(id)`; Send to crate skips those crates; sidebar drops refuse them (no highlight, hint); tests
- [x] 4.2 Add files, add URL, paste and page sends targeting those crates refuse with the hint, while app writers (wantlist add, collection add, sync, refresh, token merge) still insert; tests for each path; moving out and reordering inside still work

## 5. Remove from collection

- [x] 5.1 `dig`: `Command::Discard { release }`: GET `/users/{u}/collection/releases/{r}`, pick the latest `date_added` (ties to the highest `instance_id`), DELETE `/users/{u}/collection/folders/{f}/releases/{r}/instances/{i}`; no instance or 404 → done with no DELETE; events `Discarded { release, instance, remaining }` / failure; tests with the fake transport (two copies → newest removed, already gone, 404, server error), within the rate limit
- [x] 5.2 `Collection::discard(instance, release, remaining)`: drop the instance, lower `count`, drop the release and its `by_master` link when `remaining == 0`; save atomically; unit tests (another pressing of the master still owned; next incremental sync after a removal takes 1 request)
- [x] 5.3 Pending removals in dig memory (survive restarts), retried like wantlist changes (offline waits; 1, 2, 5, 15, 60 min after server errors), then ⚑ "collection removal failed" and Retry remove from collection; "removal pending" in the tooltip; tests with fake time
- [x] 5.4 UI: "Remove from collection…" in the collection crate's menu with a token, for a single record only (hidden for a multi-record selection); the confirmation modal (record, pressing, "notes and rating are lost", Enter/Esc), in `dig_asking()`; on `Discarded`, update the cache, drop the record's entries from the collection crate when no copy is left, refresh OWNED marks, notify; headless tests (sold it, two copies, several records selected, cancel)

## 6. Docs and checks

- [x] 6.1 README (the style filter, Remove from collection, the Discogs crates taking no hand edits, the test count), help panel (`help.rs`)
- [x] 6.2 Run the app on a big collection crate: filter by style with chips and with the list, play through filtered records, remove a copy from a test account's collection; fmt, clippy, the workspace tests, the wasm check, `--click-test`, `--startup-time` and `--bench`
