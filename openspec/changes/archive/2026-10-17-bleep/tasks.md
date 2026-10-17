## 1. Look at the real pages (D1, D3)

- [x] 1.1 Read real Bleep pages (live ones refuse non-browsers: 2025 Wayback copies of a release and a label page); record in design.md the address and id formats, where the page keeps its data, and the format and price fields; correct `bleep-intake`'s provisional rules to match before writing any check
- [x] 1.2 Find how Bleep's player gets a preview: issued per request by `/player/resolve/…`, so the search path (D3); recorded in design.md

## 2. The send (`dig`)

- [x] 2.1 `Store::Bleep`, `bleep.ron`, and `bleep.rs` checking a send into `StoreRecord`s with formats; unit tests for every rule (ids, sizes, formats, control characters, one bad record refused alone, 501 refused whole)
- [x] 2.2 The bridge accepts `bleep` sends up to 1 MiB, checked before anything happens, answered within 100 ms; verify with bridge tests (accepted, a bad record refused alone, too big, unknown field)

## 3. Previews

- [x] 3.1 Bleep tracks wait "to search" (and a list record by its title), verified with a headless test

## 4. In the player

- [x] 4.1 Merging (a release enqueued in order; a list record as one entry; 500 records in under 50 ms with no underruns), Discogs linking and a linked list record replaced by its release's tracks, the per-format price note with "may have changed", Open on Bleep, "found for Bleep", and pasted bleep.com / bleep.co.uk addresses refused; verify with headless tests

## 5. The extension

- [x] 5.1 `host_permissions` for `*.bleep.com`; `bleep.js` reading release and list pages only when the menu opens or a ▶ is clicked, never fetching; the button, none on basket, account or checkout pages, "Couldn't read this Bleep page" when nothing parses; a ▶ after each record on lists playing it alone
- [x] 5.2 Manual checklist in the README: each saved fixture and the live pages send the expected counts, the actions work, and the extension makes no request but the one to the app

## 6. Docs and checks

- [x] 6.1 README: Bleep in "From the browser" (and that a Discogs token brings list records' tracks), the extension's site access, `bleep.ron` in the cache table, test areas and count
- [x] 6.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check
- [x] 6.3 With the extension loaded: dig a live Bleep label page and a release, play a few, and open one on Bleep
