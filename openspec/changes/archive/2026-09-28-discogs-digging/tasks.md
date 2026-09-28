## 1. Crate and dependencies

- [x] 1.1 Create `crates/dig` (native only) with the module skeleton from design D1, add `ureq` (rustls) and `webbrowser`, and make `ui` depend on `dig` only for non-wasm targets; verify that `cargo check --workspace` and the wasm check of `audio` and `platform` pass

## 2. Discogs client (`dig::discogs`)

- [x] 2.1 Add the transport trait, the `ureq` transport (headers, timeouts) and a fake transport that serves recorded JSON fixtures and can inject 429s, delays and network errors; verify with a unit test that requests carry the User-Agent, Accept and token headers
- [x] 2.2 Add the rate limiter on an injectable clock (60 or 25 per 60 s, slower when few requests remain, backing off on 429); verify with a test that 800 requests never put more than 60 in any 60-s window
- [x] 2.3 Add the disk cache under `<cache>/discogs/` (record data kept, listings for 24 h, for-sale numbers refreshed after 24 h when their track starts); verify with temp-dir tests, and add `discogs/` to the README's "Where files are kept" table
- [x] 2.4 Add the token check (`/oauth/identity`, then the user's currency), the token file readable only by the user, and error mapping (rejected token, not found, private wantlist, offline with automatic resume); verify with fake-transport tests

## 3. Addresses and expansion (`dig::discogs`)

- [x] 3.1 Parse every supported address form (language prefix, no name part, legacy slug-first, query, fragment, both wantlist forms) and refuse the rest; verify with a table-driven unit test
- [x] 3.2 Implement listings for each page kind with pagination (release, master, artist with Main and Remix roles oldest first, label, wantlist, list), and vinyl detection from listing formats or record details; verify with fake-transport tests on fixtures
- [x] 3.3 Implement clip id extraction and validation, clip-to-track matching (normalized titles, duration tie-break, long unmatched clips kept), the remix-credit rule, "no clip" records and dedupe; verify with unit tests on real-world-style clip titles
- [x] 3.4 Fetch record details focus-first and persist send jobs in `<config>/dig/jobs.ron`, resuming without duplicates; verify with tests that move the focus mid-expansion and restart a half-done job

## 4. Sending into crates (`ui`)

- [x] 4.1 Implement the send modes (Enqueue, Play, Crate): intake events become waiting entries ("listed" placeholders replaced in place by clip entries or "no clip"), the for-sale snapshot is added to `Origin` with a serde default, and progress shows in the main window; verify with headless tests using the fake transport
- [x] 4.2 Make Cmd+V (Ctrl+V on Linux and Windows) paste a Discogs address into the shown crate, ignoring other text; verify with a headless paste-event test, and document paste and the supported pages in the README

## 5. Previews (`dig::preview`)

- [x] 5.1 Add the `Fetcher` trait, the yt-dlp fetcher (argument list without a shell, `--ignore-config`, M4A/AAC format, progress parsing, 120 s timeout and one retry) and `FakeFetcher`; verify with unit tests of the argument list and id validation, plus an `#[ignore]` test against a real yt-dlp
- [x] 5.2 Find yt-dlp (PATH or configured path, version, checked again every 30 s while needed), and show "needs yt-dlp" with a one-time install hint; verify with a test that uses a missing and then a present fake program
- [x] 5.3 Schedule downloads: armed entry first, then the playing entry and the next 3 (or the shown crate's current entry and the next 3 when stopped), 2 at a time, cancelling downloads that leave the horizon, with progress on the entries; verify with `FakeFetcher` tests
- [x] 5.4 Keep the preview cache under `<cache>/previews/` within its size limit, deleting the least recently played first and never the protected set; verify with temp-dir tests, and document `previews/` and the size setting in the README

## 6. Ready to navigate (`dig::prepare`, `analysis`)

- [x] 6.1 Add `analysis::overview::precompute` (build and cache without keeping the overview in memory); verify with a unit test that a precomputed overview is loaded from the cache on first play
- [x] 6.2 Add the prepare worker: score and overview for downloaded previews in horizon order, gated on the playing track's first 32 bars, skipping previews already cached; verify with a test that a prepared fixture has its full score and overview cached, and that nothing starts before the gate opens
- [x] 6.3 Measure preparation of a 6-minute preview on an M-series Mac while another track plays (target 20 s or less); if it's over, build the overview first; record the numbers in a Verification section of this design

## 7. Verdicts (`dig::memory`, `ui`)

- [x] 7.1 Store the dig memory in `<config>/dig/memory.ron` (kept and passed clips, releases the app added to the wantlist, pending wantlist changes); verify with a round-trip test
- [x] 7.2 Implement keep and its undo: the Keepers crate (created or adopted when needed, id in the dig settings), ✓ marks, wantlist add after a membership check, removal only when the app added it, and "wantlist pending" retries; verify with fake-transport tests for keep, undo, already on the wantlist, offline and no token
- [x] 7.3 Implement pass (dimmed, next track, left out of later sends, Undo pass, refused on kept entries) and open-for-sale (default browser behind a test seam, message for entries not from Discogs); verify with headless tests
- [x] 7.4 Bind Y, N and I in the player window and in fullscreen (the host keeps them), and add Keep, Pass and Open for-sale page to the entry menu; verify with headless key-routing tests, and add the keys to the help panel and the README's key table

## 8. Title line and settings (`ui`, skin)

- [x] 8.1 Add `·`, `€`, `£`, `$` and `¥` glyphs to the skin generator and regenerate the committed skin; verify with the skin tests
- [x] 8.2 Add the title-line suffix (side, catalog number, year, for-sale summary with a symbol or an ISO code); verify with `format` unit tests, including "none for sale" and unknown numbers
- [x] 8.3 Build OPT ▸ Discogs…: token entry and check (username shown, only the last 4 characters of the token), yt-dlp status and path, default filters, preview cache size; verify with headless tests, and document the setup (token, `brew install yt-dlp`) and what previews are for in the README

## 9. Integration checks

- [x] 9.1 Add `crates/ui/tests/dig_playback.rs`: an 800-release fake label expands, and `FakeFetcher` previews are downloaded and prepared, while the engine plays; verify zero underruns, and that a ready preview starts within the fast-start budget
- [x] 9.2 Launch with a token and an interrupted send: verify that `--startup-time` stays under 300 ms and that the fake transport sees no request before the window is interactive
- [x] 9.3 Test end to end by hand with real Discogs and yt-dlp: paste a label, a release, a master, an artist, a wantlist and a list; navigate a preview; press Y, N and I; check the wantlist on discogs.com
- [x] 9.4 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check; confirm the README's test count matches
