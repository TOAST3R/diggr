## Context

Every filter feeds one predicate, `Shown` (`crates/ui/src/playlist.rs`), and everything downstream (rows, play order, previews ahead, `k/N`, P, record rows) reads only that. The record filters (style, artist, label, format) are generic over `Facet`: picks per facet, OR within a facet and AND across facets, per-crate persistence, the "gone value stops filtering" rule, record counts, bar buttons, panel tabs, Clear and ×. Only `offered_facets()` (`app.rs`) restricted style, artist and label to `crates.is_discogs()`. The `artist-label-filters` design listed "these filters outside the wantlist and collection crates" as a non-goal, a scoping choice rather than a technical limit.

`Origin.year: Option<u16>` is already filled from listings and details, and the parser already drops Discogs' "0 = unknown".

## Goals / Non-Goals

**Goals:**
- Pick years in a grid, any of them, with a Shift-click for a run.
- Style, artist, label and year filters in every crate with record data, with no new per-crate rules.

**Non-Goals:**
- The original (master) year for releases: one request per record to learn it. The pressing's year is what's in hand.
- "Only this year" on a row, decade shortcuts.
- Telling "details not fetched yet" apart from "no style" in the STYLE list.

## Decisions

### Year is a facet
`Facet::Year` sits between Label and Format, so the tabs read STYLE · ARTIST · LABEL · YEAR · FORMAT. `Entry::values(Year)` yields the year as text, so picks, `counts()`, persistence (`SavedPlaylist.years`), the gone-value rule, the YEARS button and its count, Clear, × and P all come from the facet code with no new logic.

*Why:* a year picked from a grid is a set of values, exactly what a facet is. *Alternative 1, tried first:* a range slider in the bar, generalising the BPM range (`Range::{Bpm, Year}`). In use, picking years in a grid proved more usable, and the slider crowded the bar at classic width. It was removed again, leaving the BPM code as it was. *Alternative 2:* a separate `BTreeSet<u16>` filter. Rejected: it would copy what facets already do.

### Year text without allocation
`values()` borrows `&str` from the origin, and `Shown::shows` runs over every entry on each change, so it must not allocate. Years come from a static table of `"1000"`…`"2999"` (`LazyLock<Vec<String>>`, built once). Years outside it count as unknown.

*Alternative:* make `values()` return `Cow<str>`. Rejected: an allocation per entry per pass, and every caller changes.

### The grid
The YEAR tab draws an `egui::Grid` of five columns from the earliest year (rounded down to a multiple of 5, so decades line up) to the latest, each cell a selectable button "year\ncount". Empty years show "·" and are disabled. A click pushes `TogglePick(Year, y)`. A Shift-click pushes `TogglePick` for each year with records between the last clicked year (`year_anchor`) and this one that isn't picked yet, so it only adds.

### Facets: drop the gate
`offered_facets()` offers every facet with at least two values in the shown crate; a local file has no values, so a crate of local files offers none. A label crate therefore offers ARTIST but not LABEL. The row menu's "Only this …" items follow `offers(facet)` (read from the per-frame counts cache, `&self`), and "Search ‹label›/‹catno›" appears only where no label filter is offered.

### Styles that arrive late
Nothing new: a record without details has empty `styles`, so it is hidden while a style is picked, as a record with no style is today. `facet_counts` is cached on `(crate, rev)`, and filling details goes through `entries_mut()`, which bumps `rev`, so counts and the list follow as details land.

## Risks / Trade-offs

- [A crate spanning 1950–2024 makes a 15-row grid] → It scrolls (320 px), like the long lists.
- [Picking a style in a fresh 2,000-record seller crate hides most of it] → It shows the records known to match, and grows as details come. The count beside each style makes the partial state visible.
- [Pressing year splits a classic across eras (1993 original, 2016 reissue)] → Accepted and stated in the spec. Master entries carry the master's year.
- [Six filters on 5,000 entries within 16 ms] → `Shown` is still a single pass with cheap tests. The big-crate test covers it.

## Migration Plan

`years` is `#[serde(default, skip_serializing_if = "BTreeSet::is_empty")]`, so old crate files load unfiltered and files without picked years don't change. Style, artist and label picks were never saved for other crates, so nothing appears filtered unexpectedly. Rollback: revert. `SavedPlaylist` doesn't deny unknown fields, so the old build ignores saved `years`.

## Open Questions

- None.
