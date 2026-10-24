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

## 7. A genre or a style, not both

- [x] 7.1 Hide the Style row while a genre is picked and the Genre row while a style is picked; the "all" section lists every style; a saved pair opens with the style kept; drop `styles_of` and `genre_has`
- [x] 7.2 Headless tests: picking a style hides the genre, "Any style" brings it back, picking a genre hides the style; the dig and refresh test uses a style and text
- [x] 7.3 Spec, design, README and CHANGELOG say one at a time

## 8. Nothing waits for listings

- [x] 8.1 `Criteria`: `format` (Vinyl, CD, Cassette) and `year` as search words in `discogs_query()`; drop `formats`, prices, condition, `is_local`, `matches`; `inventory_page` keeps every copy for sale
- [x] 8.2 Remove the Narrow down scan: `ScanInventory`, `CancelScan`, `Scanned`, `ScanFailed` and the intake's scan step; the dialog's scan state, progress and price fields
- [x] 8.3 Narrow down: Format dropdown and Year field (counted at once / after the typing pause); Price and Condition rows gone; the cap note names the new criteria
- [x] 8.4 Tests: format and year words; old saved filters load; a headless test that format and year are counted with no listings read and dug; scan tests removed
- [x] 8.5 Spec, design, proposal, README and CHANGELOG

## 9. Genre and style together

- [x] 9.1 Both dropdowns always show; with a genre, every style section holds that genre's styles; changing the genre clears a style it doesn't have; `styles_of` and `genre_has` back; the Genre dropdown shows all 16 without scrolling
- [x] 9.2 Tests: Rock keeps the style dropdown with Rock styles only, Deep House cleared, Rock + Punk searched together; genre + style + text dug and refreshed
- [x] 9.3 Spec, design, proposal, README and CHANGELOG

## 10. Up to 5,000, a thousand at a time

- [x] 10.1 `Criteria.limit` and `window()` (pages and the newest cut use it), `MAX_DIG`, `Seller.matching`, `PageKind::Inventory.from_page` (the job starts there)
- [x] 10.2 Narrow down: Dig enabled up to 5,000, "Dig the newest 1,000" beyond 1,000, the count line says the rest come when asked; Dig saves the limit and the match count
- [x] 10.3 Refresh: up to 5,000 for a crate with a limit; keeps the limit and records the count
- [x] 10.4 `ListRow::More` at the end of the list ("+ ADD NEXT 1,000 · N MORE" / "+ ADD THE LAST N"), skipped by the cursor; a click reads the next ten pages into the crate
- [x] 10.5 Tests: window and pages; 3,412 copies dug 1,000 at a time with only the next pages read and nothing before a click; 1,200 now diggable
- [x] 10.6 Spec, design, proposal, README and CHANGELOG
- [x] 10.7 Fix: the row showed only once every record was expanded (15+ minutes for hhv). The intake sends `ListingRead`, the row shows from then, and a refresh applies then; tests for both
