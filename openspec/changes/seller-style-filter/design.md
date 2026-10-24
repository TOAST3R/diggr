## Context

Narrow down (`crates/ui/src/app/sellers.rs`, `dig_dialog_body`) splits its criteria in two:
- **Discogs side:** the search text (`q`) and "newest N". One `CountSeller` request gives an exact count across the whole stock.
- **Local:** format, price, condition and country. These need the listings read, and Discogs serves at most 100 pages of 100 for another user's stock, so for a shop like www.hhv.de (159,648 copies) the local filters see only the newest 10,000.

Probes against the public API (October 2026, www.hhv.de) showed:
- The inventory `q` is a full-text search over the release, including its genres and styles: the search `deep house` returns records with neither word in their title, whose styles include Deep House. Words are ANDed; `-word` excludes.
- A quoted phrase narrows to the phrase: `deep house` 4,999, `"deep house"` 4,429. Discogs' own website facet for the shop says 4,316. Genre names with punctuation work quoted (`"Funk / Soul"` 17,660; `"Folk, World, & Country"` 12,522), and so do several quoted phrases together (`"Electronic" "Deep House"` 4,419).
- Listings carry no genres or styles. There's no endpoint listing a seller's styles or all styles. The website (which has exact facet counts) answers 403 to the app.

## Goals / Non-Goals

**Goals:**
- A genre and a style that narrow the whole stock on Discogs' side, with the count the modal already makes and no other requests.
- A style list that puts the styles likely to matter first: this seller's, then the user's.
- Remove Ships from without breaking saved sellers.
- Stop showing the 10,000 cap as a match count.

**Non-Goals:**
- Per-genre or per-style totals, or hiding styles the seller doesn't stock (each would cost a request per style).
- Several styles at once, or a genre with a style (Discogs ANDs the search; "either" would need a count and a read per style, merged). The user asked for a genre *or* a style: picking one hides the other's dropdown.
- Exact style matching: the quoted phrase also matches titles and labels.
- Price and condition: Discogs can't search on them, and filtering them meant reading up to 100 pages of listings (about two minutes at the rate limit). Removed at the user's request for a dialog that never waits.
- Year range, exclude words, "wanted by ≥ N", "listed in the last N days", reading in price order. These are candidates for a follow-up.
- Reading the website's facets through the browser extension.

## Decisions

**1. Genre and style are their own fields, combined into the search sent to Discogs.**
`Criteria` gains `genre: Option<String>` and `style: Option<String>`. `query` stays the user's own text. `Criteria::discogs_query(&self) -> String` returns `"‹genre›" "‹style›" ‹query›`, leaving out the empty parts and trimming. The dialog only ever sets one of the two (decision 8), and a saved pair opens with the style kept. Every place that sends or compares the search uses it: `inventory_path` callers (`expand::inventory_listing`), `CountSeller` (the dialog, refresh counting), the `counted` / `ScanState.query` keys and `seller_counted`'s matching, and the "is this the seller's whole stock" check (`query.is_empty()` → `discogs_query().is_empty()`).
*Alternative:* write `"Deep House"` into the search text box. Rejected: the dropdown couldn't show what's selected, the user could break the quoting, and the saved criteria would lose the distinction.

**2. Quoted phrases, genre first.**
Quoting cuts the over-match (4,999 → 4,429, against an exact 4,316). Discogs ANDs everything in the search, so the order of the parts doesn't change the answer, but a fixed order keeps the query string stable. That matters because the count and the scan are keyed by it. `path_segment` already encodes `"`, `/`, `,` and `&`.

**3. The genre and style list ships as Rust data in `dig`.**
`crates/dig/src/discogs/styles.rs`: `pub const GENRES: &[(&str, &[&str])]`, Discogs' 15 genres, each with its styles. Discogs' guidelines page answers 403 to scripts, so the list comes from Essentia's `genre_discogs519` taxonomy (519 genre/style pairs taken from Discogs), plus Amapiano, Balearic, Ballroom, Gqom and Jersey Club under Electronic. Helpers: `genres()` and `all_styles()` (deduplicated and sorted; a few styles sit under two genres).
*Alternative:* a RON data file. Rejected: the list is compile-time constant and never user-edited, and a const can't fail to load. It's updated by hand when Discogs adds styles, which is rare.

**4. The style sections are computed when the dialog opens, from crates already in memory.**
- "in this crate": `Entry::values(Facet::Style)` over the seller crate's playlist.
- "yours": the same over the wantlist and collection crates (`Crates::is_wantlist` / `is_collection`).
Each is counted, sorted most first then by name, and stored on `DigDialog`, so the dropdown's frame cost is filtering a few hundred strings. The sections are only computed from what's loaded: if a crate's playlist isn't in memory, its section is simply missing (no disk read on the UI thread).

**5. The dropdown is a popup with a search field, not a `ComboBox`.**
A ~600-style `ComboBox` can't be searched. Use a button showing the selection (or "Any style") that opens an `egui::Popup` holding a `TextEdit` and a scroll area with the section headers and selectable rows, the same way the playlist's style tab does it (search, case-insensitive containment). The genre dropdown is a plain `ComboBox` (16 entries).

**6. Ships from is dropped from `Criteria`.**
`Criteria` is `#[serde(default)]` without `deny_unknown_fields`, and RON ignores unknown struct fields, so a saved `ships_from: [...]` is ignored on load. `Copy.ships_from` / `SaleCopy.ships_from` stay: rows and copy details still show the country. `DigDialog.country` and its field-reading code go.

**7. The 10,000 cap wording.**
When `total > MAX_PAGES * PER_PAGE` and the criteria aren't local, `matching()` returns the cap (via `reachable`), and the body currently prints "10,000 match: narrow it down". The body tells that case apart and prints only the "Discogs only shows the first 10,000: a genre, a style, search text or newest N narrow it on its side" line, never "10,000 match".

**8. A genre or a style, never both.**
While a genre is picked, the Style row is hidden; while a style is picked, the Genre row is hidden; "Any genre" / "Any style" brings the other back. So the "all" section always lists every style (no genre narrowing), and nothing has to reconcile a genre with a style from another genre.
*Alternative (first version):* both together, the style list narrowed by the genre and a style from another genre resetting it. Dropped at the user's request: a style already implies its genre, so both together only added rules.

**9. Every criterion is a Discogs search word; no page reading.**
Probes at hhv: `vinyl` 151,307, `cd` 9,643, `cassette` 2,460, `2025` 20,665, `"Electronic" vinyl 2025` 6,555, so format and year narrow the whole stock with the same single count. Format is one choice (Vinyl, CD, Cassette: the search ANDs words, and `file` matched noise); year is one year (a range would need a count per year). With nothing left to filter locally, the scan (the intake's `ScanInventory`, `Scanned`, `ScanFailed`, the dialog's progress line and Cancel-on-scan) is removed, and `Criteria` loses `formats`, `min_cents`, `max_cents`, `min_condition`, `is_local` and `matches`.
*Alternative:* keep price and condition and apply them while the dig reads its pages (no count first). Not chosen: the user picked a smaller dialog that answers at once.

## Risks / Trade-offs

- **Text match, not a facet** → a style catches titles and labels (Deep House: 4,429 vs 4,316). Diggers can tidy up after the dig with the crate's footer style filter, whose styles come from the record details. The README says so.
- **A style the seller doesn't stock** → the count answers 0 after one request and Dig stays disabled. Cheap, and it needs no per-style counts.
- **The shipped list ages** → a new Discogs style is missing from "all" until updated, but still appears in "in this crate" / "yours" once records have it.
- **Saved country criteria vanish silently** → a seller narrowed by country may grow past 1,000 on its next refresh. The existing rule (the refresh changes nothing and opens Narrow down) covers it.

## Migration Plan

None beyond the fail-soft load. Old sellers keep their search text, newest N, format, price and condition.
