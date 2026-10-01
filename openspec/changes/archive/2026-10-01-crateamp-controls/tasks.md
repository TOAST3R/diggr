## 1. Low-pass filter

- [x] 1.1 `audio`: resonant low-pass biquad (Q ≈ 1.2) after the EQ and before the tap, exact bypass at 1.0 with a crossfade in and out, smoothed cutoff with coefficients every 32 samples, recomputed on a sample-rate change; `Engine::set_filter`; tests: bypass is sample-identical, a sweep has no step larger than the smoother allows, switching on and off has no step, `rt_alloc` stays green
- [x] 1.2 Skin: the knob sprite (ring and pointer) in `generate.rs`, the LP knob at x 46 in the EQ's top row (drag, double-click to reset, tooltip with the cutoff, not remembered); regenerate with `skin-gen`; headless test (drag sets the cutoff, double-click turns it off)

## 2. BPM filter

- [x] 2.1 Per-crate range saved with the crate (omitted at full span) and sanitised on load; the matching predicate and the shown-rows mapping (row → entry, crate numbers kept); unit tests (range, entries without BPM, full span, widened when tempos no longer reach it, 1,000 entries in under 16 ms)
- [x] 2.2 Filtered play order: next, previous, shuffle and pre-warm over shown entries; the hidden playing entry finishes; the queue is rebuilt on change; the preview horizon and preparation follow it; headless tests (next under a filter, hidden while playing, horizon skips hidden entries)
- [x] 2.3 Rows by entry id under the filter: drawing, click and Shift/Cmd selection, drag-to-reorder (to the crate position of the entry under the pointer), keyboard cursor, Select all and Invert over shown entries; P turns the filter off when it hides the playing entry; sort and export ignore the filter; headless tests
- [x] 2.4 The filter bar: ALL, the two-handle slider (sprites in `generate.rs`) with its text and "+n without BPM", shown only with two or more tempos; the title bar's "· shown/all"; "No tracks match" with ALL; headless tests

## 3. Crate sidebar

- [x] 3.1 The sidebar: rows with counts and the ⏵ / • marks, click to show, right-click Rename…/Delete…, + New crate, unreadable crates dimmed; the ☰ title-bar button (sprite in `generate.rs`, remembered in settings), shown only at 600 px or maximized; headless tests
- [x] 3.2 Dropping dragged entries on a sidebar crate (selection rules, duplicates skipped, the Send to crate message, nothing on the shown crate), the highlight while dragging, reordering still working in the list; headless tests

## 4. Docs and checks

- [x] 4.1 README (LP knob, BPM filter, sidebar, new settings key, test count), help panel
- [x] 4.2 Run the app to try the sweep, the filter and the sidebar; fmt, clippy, the workspace tests, the wasm check, `--click-test` and `--startup-time`
