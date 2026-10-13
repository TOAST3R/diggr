## Why

Diggers narrow a pile of records by era as often as by style ("early-90s Detroit"), but no filter knows a record's year, though every Discogs entry already carries it. And the style, artist and label filters exist only in the wantlist and collection crates, so a friend's collection, a seller's stock or a big dig crate, often the larger piles, can't be narrowed the same way.

## What Changes

- **Year filter**: a YEARS button beside the other filter buttons opens a YEAR tab in the filter panel: a grid of every year from the crate's first to its last, five a row, each with its number of records. A click picks or unpicks a year; a Shift-click picks a run. Any picked year shows. A record's year is the one Discogs gives (the pressing's, or a master's original); a record without one is shown only while the filter is off. It is remembered per crate, combines with every other filter, and clears like them. (A range slider in the bar was tried first; picking years in a grid proved more usable.)
- **Record filters in every crate**: the style, artist and label filters (and the year filter) are offered in every crate holding entries with record data (friend, seller, label, dig and Bandcamp crates), not only the wantlist and collection crates. The existing rule still applies: a filter shows only when the crate has at least two values for it.
- **Styles fill in late**: outside the wantlist and collection crates, a record's styles arrive with its details, fetched nearest the playhead first. Until then the record has no style and is hidden while a style is picked, as a record with no style is today. The STYLE list grows as details arrive.

## Capabilities

### New Capabilities
- `year-filter`: the year filter, its grid in the filter panel, and how it's remembered.

### Modified Capabilities
- `record-filters`: artist and label filters are offered in every crate with record data; "Filters combine" includes the year filter; "Filter controls" gains the YEARS button and covers which crates offer which filters.
- `style-filter`: the style filter is offered in every crate with record data, not only the wantlist and collection crates.
- `filter-bar`: the panel gains a YEAR tab holding the grid; a row's Search ‹label› / ‹catno› items appear only where no label filter is offered.

## Impact

- `crates/ui/src/playlist.rs`: `Facet::Year`, its values served as text from a static table (no allocation while filtering); `SavedPlaylist` gains `years` (serde default, so old crate files load).
- `crates/ui/src/app.rs`: the YEAR tab's grid; `offered_facets` drops the `is_discogs` gate; the row menu keys on the offered filters.
- Specs: new `year-filter`; deltas to `record-filters`, `style-filter`, `filter-bar`. README's filter section and test count.
- No new dependency, no network request, nothing on the playback path.
