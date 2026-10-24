## 1. Genres and styles data

- [x] 1.1 Add `crates/dig/src/discogs/styles.rs` with `GENRES: &[(&str, &[&str])]`: Discogs' 15 genres and their styles, from Essentia's `genre_discogs519` Discogs taxonomy (the guidelines page refuses scripts), plus a few newer Electronic styles
- [x] 1.2 Add `genres()`, `styles_of(genre)`, `all_styles()` (deduplicated, sorted) and `genre_has(genre, style)`, with unit tests (Deep House is under Electronic, not Jazz; no duplicates in `all_styles()`; every genre has at least one style)

## 2. Criteria

- [x] 2.1 Add `genre` and `style` to `Criteria`, remove `ships_from` (and its clause in `matches` and `is_local`), and update the doc comment on which criteria Discogs applies
- [x] 2.2 Add `Criteria::discogs_query()` (`"‹genre›" "‹style›" ‹query›`, empty parts left out, trimmed), with unit tests: style only, genre + style + text, text only, nothing; `inventory_path` encodes `"Folk, World, & Country"` correctly
- [x] 2.3 Test that a RON seller list saved with `ships_from: ["Germany"]` loads, keeping the other criteria

## 3. Requests use the full Discogs query

- [x] 3.1 `expand::inventory_listing` reads with `criteria.discogs_query()`
- [x] 3.2 In `crates/ui/src/app/sellers.rs`, send `discogs_query()` in every `CountSeller` (dialog open, typing pause, refresh counting) and key `counted`, `ScanState.query`, `scan_fits`, `total` and `seller_counted`'s matching on it; the "whole stock" total update stays tied to an empty Discogs query
- [x] 3.3 Fix the `dig` integration tests (`tests/seller_api.rs`, `tests/intake.rs`) for the new criteria; add one that a style-only criterion's inventory request carries `q=%22Deep%20House%22`

## 4. Narrow down modal

- [x] 4.1 Remove the Ships from row, `DigDialog.country` and its field reading
- [x] 4.2 Add the Genre `ComboBox` ("Any genre" + `genres()`) above Search; changing it clears a style the genre doesn't have and starts a count after the typing pause, as Search does
- [x] 4.3 Compute the "in this crate" and "yours" style lists when the dialog opens (`Entry::values(Facet::Style)` over the seller crate and the wantlist and collection crates in memory; counted, most first then by name) and keep them on `DigDialog`
- [x] 4.4 Add the Style dropdown under Genre: a button showing the style or "Any style", opening a popup with a search field, "Any style", and the "in this crate" / "yours" / "all" sections (the last limited to the picked genre); picking a style outside the genre resets the genre to "Any genre"; picking closes the popup and starts a count
- [x] 4.5 Reword the cap: when more than 10,000 match the Discogs-side criteria, show only "Discogs only shows the first 10,000: a genre, a style, search text or newest N narrow it on its side", never "10,000 match"

## 5. Headless tests (`seller_tests.rs`)

- [x] 5.1 Picking Deep House in the style dropdown sends one count with the quoted style; the count shown is the fake's answer and Dig stays disabled above 1,000
- [x] 5.2 Genre + style + typed text → the count and the dig read with `"Electronic" "Deep House" vinyl`
- [x] 5.3 Style dropdown sections: "yours" lists collection styles; "in this crate" appears only for a dug seller; searching "deep" narrows every section; Jazz limits "all" to Jazz styles; picking a non-Jazz style resets the genre
- [x] 5.4 Over 10,000 copies: the cap line is shown and "10,000 match" is not
- [x] 5.5 A refresh of a seller saved with a style counts and reads with that style; the dropdowns themselves send no request

## 6. Docs and checks

- [x] 6.1 README: Top Sellers (Genre, Style, the three sections, text-match caveat, Ships from gone) and the Tests section (new cases, updated test count)
- [x] 6.2 CHANGELOG `## Unreleased` entry, following the `changelog` skill
- [x] 6.3 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo check -p audio -p platform --target wasm32-unknown-unknown` all clean
