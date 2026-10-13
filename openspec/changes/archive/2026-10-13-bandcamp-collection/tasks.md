## 1. Spike: can the app read a fan page?

- [x] 1.1 Throwaway test (ignored, network): get `bandcamp.com/toast3r` with the `ureq` agent from `discogs/transport.rs` and a browser `User-Agent`, 5 times; record whether the `data-blob` or the "Client Challenge" comes back, and whether `POST /api/fancollection/1/collection_items` answers. Note the result in `design.md` decision 2
- [x] 1.2 If `ureq` gets the challenge: check that `yt-dlp --write-pages -P temp:‹dir›` saves the page into a temp folder (`TestDir`), and pick that as the fallback; if neither works, stop and revisit the design
- [x] 1.3 Count, for the user's 99 + 103 items, how many have a catalogue number and how many get exactly one Discogs match (one-off script, results in `design.md`)

## 2. Fan pages and reading in `dig`

- [x] 2.1 `bandcamp::fan`: `parse` for `bandcamp.com/‹user›` and `/wishlist` (www, trailing `/`, query, fragment; name rules; reserved names refused; rebuilt address); unit tests incl. hostile addresses
- [x] 2.2 Parse the page's `data-blob` (fan id, counts, last tokens, first items) and the service's JSON into `FanItem { key, page, artist, title, catno, kind: Digital|Physical }`; items failing `bandcamp::parse` are kept as unreadable; fixtures trimmed from the real answers; unit tests
- [x] 2.3 `FanReader`: page read (ureq, or the fallback from 1.2), then paging at `count: 100`, ≥ 1 s apart, until `more_available` is false; Stop after the current request; fake-transport tests (two pages, challenge → fallback, service error)
- [x] 2.4 `<config>/dig/bandcamp.ron`: fan, fan id, `read_at`, items per list with their track pages and `discogs` state, last report; load (missing or broken → empty), atomic save; `TestDir` tests

## 3. Discogs side in `dig`

- [x] 3.1 `collection.rs`: `Pressing.digital` from `basic_information.formats` (File); `owned()` and the wantlist cleanup skip digital releases when looking by master; cache version bump → full read once; unit tests for the new scenarios of `discogs-collection`
- [x] 3.2 Client: `GET /database/search` (type release, artist, release_title, catno), through the rate limiter; fixture and tests
- [x] 3.3 Exact match: format by kind, normalized artist and title equal, catalogue number equal when both have one → `Found` / `NoMatch` / `Ambiguous(n)`; unit tests (digital vs vinyl, repress, no catno, Bandcamp-only)
- [x] 3.4 Worker commands and events: `ImportFan { page }` and the Bandcamp step of refreshes, with progress events per step (`items`, `albums`, `matching`), `StopJobs` honoured; intake test with fake transport and fake yt-dlp

## 4. Into the crates (UI)

- [x] 4.1 Paste and bridge: fan pages start an import whatever the send mode; "Not a Bandcamp fan page" when the blob has no fan
- [x] 4.2 Create and mark "Collection: ‹user›" / "Wantlist: ‹user›" when missing, with or without a token; a token saved later reuses marked crates (`Collection crate` change)
- [x] 4.3 Read each new item's page through the existing album path (≥ 1 s apart) and merge into the collection or wantlist crate with the `bandcamp-intake` merge rules; the shown crate doesn't change; the item's track pages are saved in `bandcamp.ron`
- [x] 4.4 Discogs writes for `Found` items: Add to collection when not owned as that release, Add to wantlist when not wanted, via the existing writes; then the record is sent into the crate so it takes over the Bandcamp entries
- [x] 4.5 Two sources: Refresh wantlist and Refresh collection keep an entry while Bandcamp holds its track page; each reads only its own Bandcamp list (collection or wishlist); offered without a token once a fan is imported, then read only Bandcamp
- [x] 4.6 Progress: the refresh window shows the three Bandcamp steps; Refresh wantlist gets the window with its own pages step and Stop; "Bandcamp import stopped"
- [x] 4.7 Report window: counts per crate and on Discogs, foldable "Not added to Discogs" (Bandcamp and Discogs-search links) and "Couldn't read", stopped note, Copy (plain text), Close; saved and reopened from Last Bandcamp import… on both crates

## 5. Extension

- [x] 5.1 `content.js`: recognise `bandcamp.com/‹user›` and `/wishlist` (same rules as 2.1, reserved names excluded); button with only ‹App›: Import collection & wishlist, sending only the address; `background.js` and bridge accept the new kind

## 6. Tests

- [x] 6.1 Headless: importing a fan of 3 collection items (1 physical) and 2 wishlist items fills both crates, with one already in the collection crate from Discogs not doubled
- [x] 6.2 Headless: without a token both crates are created and nothing goes to Discogs; a token saved later creates no other crate
- [x] 6.3 Headless: Found / already owned / ambiguous / no match give the right writes (none for the last three) and report counts; a second refresh doesn't search again
- [x] 6.4 Headless: Refresh wantlist keeps an entry on one list only, removes one off both; Refresh collection reads only new items
- [x] 6.5 Headless: Stop during matching keeps the crates, and the next refresh looks up the rest; the report reopens after a restart
- [x] 6.6 Headless: an import while a track plays keeps playback going (no underruns)

## 7. Docs and checks

- [x] 7.1 `README.md`: Bandcamp collection and wishlist (pages, what goes where, Discogs matching, report), `bandcamp.ron` in the file locations, the File-release OWNED change, the test count
- [x] 7.2 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo check -p audio -p platform --target wasm32-unknown-unknown` all clean
