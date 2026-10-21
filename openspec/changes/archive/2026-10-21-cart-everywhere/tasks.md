## 1. Knowing the stores

- [x] 1.1 `crates/dig`: search address per store from artist and title (Various → track artist, catalogue brackets stripped, URL-encoded); unit test per store and for Various
- [x] 1.2 `crates/ui`: `known_stores(entry)` in the D1 order (Discogs copy, Juno/Bleep incl. `product_of(release)`, Beatport/Traxsource, Bandcamp, Discogs release with `for_sale`); unit tests for wantlist-only, Juno-linked release, Bandcamp, local file (none)

## 2. The pill everywhere

- [x] 2.1 Pass a `Pill` for every entry and record row with a record behind it (not local files); record rows with several unsold copies get a pill that opens the record (IN CART when a copy is in the cart); rig tests for flat wantlist track rows and the several-copies row
- [x] 2.2 One known store acts at once: existing Discogs add, Juno/Bleep basket, page opens for Beatport/Traxsource/Bandcamp, marketplace page for a Discogs release; tooltips name the store; rig tests with a fake opener

## 3. Buy chooser

- [x] 3.1 The dialog (KNOWN buttons with price/count, SEARCH links), registered in `dig_asking()`, closing on Esc, close button or an action; playback untouched; rig tests: two stores, known nowhere, N held, Esc
- [x] 3.2 Juno/Bleep entries also known elsewhere open the chooser; Bleep format menu still works from the chooser's Bleep button

## 4. Docs and checks

- [x] 4.1 `help.rs` cart lines, README cart section and test count
- [x] 4.2 Verify the six search addresses by hand in a browser (Bleep's was wrong: it is `/search/query?q=`, fixed)
- [x] 4.3 `cargo test --workspace`, clippy `-D warnings`, `fmt --check`, wasm check
