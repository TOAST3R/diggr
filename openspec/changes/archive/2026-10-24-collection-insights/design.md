## Context

The collection crate (flagged `collection` in the crates index) holds every collection record as entries: one per track once a record is expanded, one waiting line before. Each entry's `Origin` carries `release`, `master`, `label` (the first label's name), `catno`, `year`, `styles` (comma-separated, genres when a record has none), `formats` ("Vinyl", "CD", "Vinyl, CD"…), `artist`, and `for_sale` (`count`, `lowest_cents`, `currency`, fetched with the record's details). Analysed tracks have `Entry.bpm`. A probe of the user's own crate (October 2026): 1,116 records, 8,605 entries, 1,081 with vinyl, styles on 8,196 entries, prices on 1,057 records, BPM on 349 tracks.

Pieces to reuse: `Playlist::counts(Facet)` (records per value, by album key), `Playlist::set_pick` / `clear_picks` (style, artist, label, year filters), `DiggrApp::dig_follow_label(page, id)` and `Crates::find_label(id)` (label crates), the spectrogram window's shape (a module of its own fed a struct each frame, returning actions), `format::thousands`.

## Goals / Non-Goals

**Goals:**
- A window of charts over the vinyl records, where every bar leads to those records.
- Numbers from data already stored; only label totals ask Discogs, once per label per week.
- No cost while closed; recomputing never stalls a frame.

**Non-Goals:**
- Growth over time (needs Discogs' `date_added`, not synced; the user declined).
- CD and digital releases in the counts.
- Exporting or sharing the charts.
- Insights for crates other than the collection.

## Decisions

**1. A record is its album key; vinyl means "Vinyl" among its formats.**
Counts are per record, not per track: group the crate's entries by `Entry::album_key()` (as `Playlist::counts` does), take each record's first entry with an origin as its data, and keep the record when `formats` contains "Vinyl". "Vinyl, CD" counts. A record's styles are its first entry's; its BPMs are all its tracks'.

**2. `insights::Stats`, a pure function of the playlist.**
`crates/ui/src/insights.rs`: `Stats::of(&Playlist) -> Stats` with `records`, `years: BTreeMap<u16, usize>`, `styles: Vec<(String, usize)>`, `heat: (styles × decades) counts`, `labels`, `artists`, `value: Value { currency, sum_cents, priced, other_currency, median_cents, dearest: Vec<(EntryId, String, u64)>, rare: usize }`, `bpm: Vec<(u16, usize)>` and `analysed`. Plain data, unit-tested with built playlists. Value uses the currency most records are priced in and counts the rest as "in another currency". Rare: `for_sale.count <= 1`.
*Alternative:* compute in the window each frame. Rejected: 8,605 entries every frame is wasted work; stats change only with the crate.

**3. Recompute on the crate's revision, off the frame.**
The app keeps `(collection rev, Stats)`; when the window is open and the collection crate's `rev()` changed, it recomputes. For a 5,000-record crate (40,000 entries) that is within a frame in the optimized build the app ships (measured by a test against 16 ms with `--release`); an unoptimized test build takes about 20 ms, so the test allows 100 ms there. It runs only when the collection changes while the window is open, so no worker thread is needed. The window draws from the cached `Stats` only.

**4. The window is a module like the spectrogram's.**
`insights::window(ctx, &Stats, &Labels, skin look) -> Vec<insights::Action>`, actions `Filter(Vec<(Facet, String)>)`, `ShowRecord(EntryId)`, `Follow(label name)`, `Close`. Drawn with egui painter calls in the skin's font and LCD colour: horizontal bars for styles, labels, artists; vertical bars for years with decade bands; a shaded grid for the heatmap (cell shade from count / max, LCD colour lerped toward the background); a small histogram for BPM. Each bar is an `interact` rect with a hover text. Laid out in two columns at the window's default size (about 640 × 560 points), one column when narrower.

**5. Clicks map to the existing filters.**
`Filter(picks)`: show the collection crate, `clear_picks(None)`, then `set_pick` each. A decade gives every year of it present in `Stats.years`; a heatmap cell gives its style plus those years. Year values are the strings `Facet::Year` uses ("1991"). `ShowRecord(id)`: show the crate, clear the picks, select the entry, `scroll_into_view`. Label picks use the label name as the label facet shows it (the first label), so they match.

**6. Label totals through the intake.**
New `Command::LabelTotals(Vec<(String, u64)>)`: each top label's name with one release of it from the crate. The intake, for each: reads that release's details (`expand::record`, normally a cache hit) and takes the id of the label whose name matches, then `GET /labels/{id}/releases?per_page=1` and reads `pagination.items`. Answers `Event::LabelTotal { name, id, total }` per label as it goes. Totals are cached in the Discogs cache (`Kind::Listing`, key the path) for 7 days, so a reopen costs nothing. Without a token the command isn't sent and rows read "‹n› records". The id also makes Follow possible: `Follow(name)` → `dig_follow_label(Page::new(PageKind::Label(id)), id)`; "Following" when `crates.find_label(id)` is some.
*Alternative:* a label search by name. Rejected: names aren't unique ("Mord" vs "Mord Records"); a release of that label in the user's own collection pins the right one.

**7. Opening.**
The collection crate's context menu (sidebar and title-bar crate menu) gains "Open Insights" after its Refresh item; it sets `insights_open`. Escape closes the window when it has focus (as the spectrogram window).

## Risks / Trade-offs

- [Label names in `Origin.label` hold several labels joined ("!K7 Records, !K7 Records")] → take the first, trimmed, the way the label facet does, so chart and filter agree.
- [A record priced in USD among EUR ones] → summed separately as "and ‹k› priced in other currencies"; no conversion.
- [Prices age: `for_sale` is from when the record's details were fetched] → the Value part says "at the last look" and shows the oldest fetch date.
- [Heatmap with many tiny styles] → only the 8 largest styles; the rest are in the Styles chart.
- [Waiting entries (records not yet expanded) have origin data but no BPM] → they count as records; BPM counts analysed tracks only.

## Open Questions

None: the user settled scope (no growth chart), opening (Open Insights on the collection crate) and label totals (one request).
