## 1. Planning a record

- [x] 1.1 `matching`: dedupe clips by video id, then build a `RecordPlan` (tracks in tracklist order as `Clip` or `Search`, extra unmatched clips, `full_album`); unit tests: Mezzanine shape (11 tracks, one "Teardrop (Official Video)" listed twice → 1 clip + 10 search, no extra), four tracks / three clips → 4, no tracklist → clips only, no clips → all search
- [x] 1.2 Full-album detection ("full album" in the folded title, or the only clip when it matches no track) and remix filtering of tracks and clips by `names_artist`; unit tests (full-album title among other clips, only unmatched clip, side rip with other clips stays extra, remix credit without its clip → one search entry)
- [x] 1.3 `intake.rs`: `Outcome::Entries(RecordPlan)` replaces `Clips` / `Tracks` in `expand_next` ("no clip" only with no clip and no tracklist); update `crates/dig/tests/intake.rs` and `crates/ui/tests/dig_playback.rs`

## 2. Search results by track

- [x] 2.1 Track key `track/<folded artist>/<folded title>`; `Remembered::Found.duration` (serde default); lookup hit only when durations agree within the trust tolerance; old `release/…` keys still read as-is; new results written under the track key; unit tests (CD position 1 and vinyl A1 share a result, 3:50 edit vs 6:20 misses, old key still found)
- [x] 2.2 `Origin.album_clip` (+ title) with serde default; old crates load unchanged; round-trip test

## 3. Applying a plan in the crate

- [x] 3.1 `digging.rs`: apply `Outcome::Entries` in tracklist order (clips `Queued`, tracks `Search` with `album_clip` set when the plan has one, extras after); skip clips the crate holds and tracks with the same record and position; headless tests (Mezzanine → 11 entries; sent twice → still 11)
- [x] 3.2 Vinyl-first with mixed plans: `silent` only when the plan has no clip; takeover matches non-vinyl entries by clip id or by track key, keeping found clips and playback; headless test (CD twin with a found-by-search playing entry, then vinyl → plays on, no leftover, no "already in crate")
- [x] 3.3 Search outcomes: a found video already another entry's clip → unavailable "already in crate" (no further search, no fallback); "not found by search" with `album_clip` → insert the full-album entry after the record's last entry once, clear `album_clip` on the record; headless tests (Exchange / (Exchange); full-album fallback; fallback after a save and reload)

## 4. Record row and docs

- [x] 4.1 Record row tooltip counting line ("11 tracks · 1 clip · 10 to search", zero counts left out, only for records with a search key); unit test in `format.rs` (no headless check: the rig can't read tooltip text)
- [x] 4.2 README (partial clips, full-album fallback, "already in crate", search results by track, test count) and help panel; run `cargo test --workspace`, clippy, fmt and the wasm check
