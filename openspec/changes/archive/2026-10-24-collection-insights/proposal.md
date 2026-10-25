## Why

A digger's collection says a lot about them, and Diggr already holds all of it: 1,116 records in the collection crate, each with its styles, label, artist, year, formats and lowest price on Discogs, plus a BPM for every analysed track. Nothing shows it. Seeing it ("482 Techno records, a 90s core, 35 Analogical Force") helps you know your own taste, and a chart you can click can take you straight to those records or to a label worth following.

## What Changes

- **Insights window**, opened by right-clicking the collection crate → **Open Insights**, drawn in the skin's style like the EQ and spectrogram windows. It counts **vinyl records only**: a release with no vinyl among its formats (CD-only, File-only) is left out.
- **Charts:**
  - records per year, a bar per year, with decade bands;
  - records per style (most first);
  - a style × decade heatmap (how the taste moved);
  - top labels and top artists;
  - value and rarity: the sum of lowest prices, the median, the five dearest records, and the records with at most one copy for sale;
  - a BPM histogram of the analysed tracks.
- **Every bar is a filter:** clicking a year, a decade, a style, a heatmap cell, a label or an artist shows the collection crate with that filter set (the playlist's style, label, artist and year filters); clicking a dearest or rare record shows it in the crate.
- **Label loyalty:** each top label reads "you own ‹n› of ‹N› releases", N being the label's total on Discogs (one request per label, cached), with **Follow** to make its label crate.
- No change to how the collection is synced: no date added, no growth-over-time chart.

## Capabilities

### New Capabilities

- `collection-insights`: the Insights window, its charts, its filters and label loyalty.

### Modified Capabilities

None.

## Impact

- **`crates/ui`:** a new `insights` module (the numbers, computed from the collection crate, and the window); the collection crate's right-click menu gains Open Insights; clicks map to `Playlist::set_pick` and the existing label following (`dig_follow_label`).
- **`crates/dig`:** an intake command that gives the top labels' ids and release totals: the id from a release of that label in the cache (else that release is read), the total from `/labels/{id}/releases?per_page=1`; cached for a week.
- **Tests:** the numbers (vinyl only, counts, decades, value), the label totals, and headless tests for opening the window and a bar setting a filter.
- **Docs:** README (Insights), CHANGELOG.
- No change to playback, analysis or the collection sync.
