## 1. Fully tried

- [x] 1.1 Record-level check (D1): playable / unavailable / pending per entry of one `record_key`, counting queued and running searches, downloads, album-upload and Bandcamp fallbacks; unit tests for each pending source and for passed / already-in-crate entries
- [x] 1.2 Run it where outcomes arrive (`NotFound`, `Done`, `ClipFailed`, Bandcamp fallback end) for the touched record only; emit not-found / partly-found once per record per session (D4); no event for records already not found at crate load

## 2. No preview notice

- [x] 2.1 The dialog (records with year and sources tried, D3 wording, Retry, Retry all, Close), in `dig_asking()`, closing on Esc/Enter; chaining appends; rig tests: one record, chained three, older record wording, N held, playback still running
- [x] 2.2 Retry and Retry all via `retry_of` plus clearing the Bandcamp `tried` mark; rig test: entries back to "to search", record shown again only after failing again

## 3. Partly found

- [x] 3.1 Record row "n/m previews" on its second line in the grouped view (no record-row tooltip exists: the track rows say it), "Retry missing tracks" in its menu; rig tests

## 4. Empty crate hint

- [x] 4.1 Store brand colours as constants with a contrast test against `pl_bg` (lighten below 3:1); `empty_crate_hint` lays out the long form with clickable names, Set up the extension opening the Browser dialog, and the short fallback; rig tests for both forms, a click opening the URL through the fake opener, and the wantlist still hint-free

## 5. Docs and checks

- [x] 5.1 `help.rs` if any new mouse action, README (notice, hint, test count)
- [x] 5.2 `cargo test --workspace`, clippy `-D warnings`, `fmt --check`, wasm check

## 6. Searching on down the record

- [x] 6.1 An armed track that ends without a preview arms the record's next track that may still play (`check_armed`), saying so; rig test: T4 and T5 not found, T6 plays without another click (fails without the change)
- [x] 6.2 "Search on Bandcamp" (Discogs records) / "Search on Discogs" (Bandcamp records) in the notice and in the track menu, opening the store's search; tests: the notice's link opens Bandcamp's search, `cross_search` both ways and none for a local file
- [x] 6.3 The preview worker says when a search starts and ends (`PreviewEvent::Searching`); the row being searched ends with a pulsing "SEARCHING…" and its tooltip says "searching YouTube…"; repaints only while searching; tests: the worker's start/end order, the row and tooltip during and after (deterministic: YouTube paused by a limit)
