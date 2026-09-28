## 1. Sorting model

- [ ] 1.1 `Field` / `Dir` and a natural-order compare; unit tests (LT-2 < LT-10, A2 < A10 < B1)
- [ ] 1.2 `Playlist::sort_by(field, dir)`: stable, unknown values last in both directions, the for-sale ordering rule; unit tests for each field
- [ ] 1.3 The app action: sort the shown crate, `mark_crate`, rebuild the queue; headless test that the playing entry continues and next follows the new order
- [ ] 1.4 Track the "sorted by" mark on the crate; clear it on manual reorder or an out-of-order add; tests
- [ ] 1.5 Benchmark test: sorting 1,000 entries takes < 16 ms (release)

## 2. Column layout

- [ ] 2.1 Settings: column widths (fractions) and visibility, with defaults and sanitising
- [ ] 2.2 Draw column mode at ≥ 480 pt: the header row, cells, blank empty values, status in the time cell
- [ ] 2.3 Header interactions: click to sort (toggle the direction), divider drag to resize, right-click to show or hide columns
- [ ] 2.4 OPT ▸ Sort menu with the same fields

## 3. Docs and checks

- [ ] 3.1 README (columns, sorting) and the test count; shortcuts help mentions header clicks
- [ ] 3.2 Run fmt, clippy, the workspace tests and the wasm check
