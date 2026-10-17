## 1. Look at the real pages

- [x] 1.2 Fetch one Beatport `LOFI.mp3` sample by its rebuilt address from the app's HTTP client (not the browser) and confirm it answers `audio/mpeg` without cookies; if it doesn't, stop and revisit D3 before going on

## 2. The shared store layer (`dig::stores`, D1)

- [x] 2.1 Move Juno's record and track model, merging, Discogs linking, price notes and "Open on ‹store›" into `dig::stores` with a `Store` enum (Juno, Beatport, Traxsource); Juno keeps `juno.ron` and every existing Juno test passes unchanged
- [x] 2.2 Beatport send checks (D2) and Traxsource send checks, with unit tests for every rule in the two specs (ids, sample UUID, Camelot code, BPM range, sizes, control characters, one bad track refused alone, 501 refused whole)
- [x] 2.3 The bridge accepts `beatport` and `traxsource` sends up to 1 MiB, checked before anything happens, answered within 100 ms; verify with bridge tests (accepted, a bad track refused alone, too big, unknown field)

## 3. Previews

- [x] 3.1 `Source::Beatport` and the generalized `StoreHttp` fetcher (D3): rebuilt address, redirect hosts, `audio/mpeg`, 10 MB, MP3 check, `bp.‹uuid›.mp3`; limiting on 403/429/HTML, unreachable pause, 404 as a failed try; Download all tracks skips it; verify with the local test server as Juno's tests do
- [x] 3.2 Traxsource tracks wait "to search" with a `SearchRequest` from artist, title with version and length (D4); verify with a scheduler test that a Traxsource track near the playhead is searched and the found upload plays

## 4. In the player

- [x] 4.1 Store BPM as the entry tempo until analysis replaces it (D5), and `origin.key` saved with the crate (D6); verify with headless tests: a Beatport chart filters by BPM before any download, analysis replaces the figure, the key survives a restart
- [x] 4.2 BP badge, tooltips (source, "128 BPM (from Beatport)", "Key: 8A (A minor)", price note with "may have changed" after 30 days), Open on Beatport and Open on Traxsource, and pasted store addresses refused; verify with headless tests
- [x] 4.3 Merging (enqueue a chart in order; a Discogs "not found" entry fixed by a Beatport sample; a Traxsource track not found keeps its BPM, key and Open on Traxsource), and adding 500 tracks in under 50 ms with no underruns; verify with headless tests

## 5. The extension

- [x] 5.1 `host_permissions` for `*.beatport.com` and `*.traxsource.com`; a Beatport reader from `__NEXT_DATA__` and a Traxsource reader from track rows, run only when the menu opens, never fetching; the button on the listed page kinds, none on cart or account pages, "Couldn't read this … page" when nothing parses
- [x] 5.2 Manual checklist in the README: each saved fixture page and the live pages send the expected counts, Play / Enqueue / Send to crate work, and the extension makes no network request but the one to the app

- [x] 5.3 One Play action (asked during the live test): the menus offer Play in ‹App› without Enqueue, and Play adds to the crate on screen and plays the first track it adds; verify with headless tests (Discogs and a Beatport Play into a crate that already holds a track). At archive, reword the living specs' older Play/Enqueue scenarios to match
- [x] 5.4 Preview search accepts any one artist of a shared credit ("Basement Jaxx, Reboot"), found when a live Traxsource track wasn't; verify with a unit test

- [x] 5.5 Send to crate lists only crates a send can fill (no seller, label, Discogs or friend crates), and a ▶ after each record on Discogs for-sale lists plays it; verify with the bridge test and the README checklist

## 6. Docs and checks

- [x] 6.1 README: Beatport and Traxsource in "From the browser", the extension's site access, `beatport.ron` / `traxsource.ron` and `bp.*` previews in the cache table, the BP badge, test areas and count
- [x] 6.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check
- [x] 6.3 With the extension loaded: dig a live Beatport chart and a Traxsource chart, filter by BPM before anything downloads, play a few, and open one on each store
