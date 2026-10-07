## 1. Tracks without clips

- [x] 1.1 `matching::track_entries` and `Outcome::Tracks` for a record with no usable clip and a tracklist (artist: the track's, else the record's; side; duration; key `release/position` or `master/position`); intake tests (AF069-like fixture: five track entries; no tracklist: still "no clip")
- [x] 1.2 UI: `WaitKind::Search` ("to search"), entries from `Outcome::Tracks` in `dig_record` (also through the vinyl-first rule: a digital twin's searchable entries leave when the vinyl is there); saved and reloaded; headless test

## 2. Search in the preview worker

- [x] 2.1 yt-dlp search: arguments (`--ignore-config --no-warnings --flat-playlist --skip-download --print … -- ytsearch5:<query>`), query sanitising (no control characters, ≤ 120 chars), 30 s timeout, output parsing (only valid ids kept); `Fetcher::search` with the fake fetcher's canned results; tests (hostile title stays one argument after `--`, malformed lines ignored)
- [x] 2.2 Trust rule: normalised title containment, artist in title or channel, duration within max(10 s, 5 %), closest duration wins; unit tests (found, wrong length, another artist, no duration known, "Original Mix" suffix)
- [x] 2.3 `searches.ron` cache (found / not found with time, 7-day retry, atomic writes, kept when previews are cleared); the worker runs one search at a time ahead of downloads, drops requests that left the window, answers `Found` / `NotFound` / failed (not cached); scheduler tests with fake time

## 3. UI wiring

- [x] 3.1 `dig_horizon` sends the window's search requests in priority order (armed first); on `Found` set the clip and `origin.found` on every entry with that key in loaded crates and queue them; on `NotFound` make them unavailable; failed searches leave them "to search"; headless tests (only the window is searched; armed first; sent again uses the cache without yt-dlp)
- [x] 3.2 Tooltip "Preview: …" lines; README (records without clips, the search, the trust rule, `searches.ron` in the cache table, the test count) and help panel

## 4. Checks

- [x] 4.1 fmt, clippy, the workspace tests (`--no-fail-fast`), the wasm check, `--bench`; an `#[ignore]` test against the real yt-dlp search; send Analogical Force again and play AF069
