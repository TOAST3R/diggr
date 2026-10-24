## Why

A big seller is hard to dig. www.hhv.de has 159,648 copies for sale, and Discogs serves only the newest 10,000 of another user's stock. The Narrow down modal's format, price and condition filters see only those 10,000, and they take minutes to read. Diggers think in genres and styles ("Deep House at hhv"), and Discogs' inventory search already matches them across the whole stock: `q="Deep House"` gives 4,429 copies with one request. The modal just doesn't offer them.

## What Changes

- **Genre dropdown** in the Narrow down modal: Discogs' 15 genres and "Any genre".
- **Style dropdown** in the Narrow down modal, with a search field and three sections: styles of the records already in this seller's crate, styles of the user's wantlist and collection, and every Discogs style.
- **A genre or a style, not both:** picking one hides the other's dropdown; "Any genre" / "Any style" brings it back.
- The genre or style picked is sent to Discogs as quoted phrases in the inventory search, with the search text, so they narrow the whole stock with one count. Like the other criteria, they're saved with the seller and reused by every refresh.
- No per-style or per-genre totals: the dropdowns cost no requests. The modal's count is still the only one.
- **Format** becomes a dropdown (Any, Vinyl, CD, Cassette) and a **Year** field is added, both searched for by Discogs, so every change costs one count and nothing waits for pages of listings.
- **BREAKING (saved data, fail soft):** Price and Condition are removed: Discogs can't search on them, and reading 100 pages of listings to filter them took minutes. File and Other leave the format choice (Discogs' search can't tell them apart).
- **BREAKING (saved data, fail soft):** the "Ships from" filter is removed from the modal and the criteria. Copies still carry and show their country. A seller saved with countries loads normally, and its next refresh no longer filters by country.
- The count line no longer reads "10,000 match" when the number is Discogs' 10,000 cap: it says that only the first 10,000 can be read.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `seller-crates`: the "Narrow down" requirement gains the genre and style criteria, loses the ships-from criterion, and words the 10,000 cap honestly.

## Impact

- **`crates/dig/src/discogs/seller.rs`:** `Criteria` gains `genre` and `style`, loses `ships_from`. A new `Criteria::discogs_query()` puts the quoted genre and style before the search text, and every count and inventory read uses it.
- **`crates/dig/src/discogs/` (new data):** the fixed list of Discogs genres and their styles.
- **`crates/dig/src/discogs/expand.rs`, `crates/dig/src/intake.rs`:** count and read with `discogs_query()`.
- **`crates/ui/src/app/sellers.rs`:** the two dropdowns, the style sections, the Ships from row removed, the cap wording; the count and scan keyed on the full Discogs query.
- **Tests:** `dig` unit tests (query composition, encoding, saved criteria with `ships_from` still load), `seller_tests.rs` headless (dropdowns, count request carries the quoted style, sections).
- **Docs:** README (Top Sellers, Tests), CHANGELOG.
- No change to playback, analysis or the extension.
