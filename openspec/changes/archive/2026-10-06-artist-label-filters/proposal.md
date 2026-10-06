## Why

A collection or wantlist crate of 1,000+ records can now be narrowed by style and tempo, but a digger looks for records by who made them and who put them out just as often ("every Theo Parrish I own", "what I want from Lowtide Tapes"). The style filter's model (pick values, any of them shows, on top of the BPM range) fits artists and labels too. There is a catch: an artist or label list has hundreds of values, and the footer has no room for more controls at classic width. It already has no room for the STYLES button when the crate has tempos.

## What Changes

- **Artist filter and label filter** in the Discogs wantlist and collection crates, built like the style filter:
  - pick artists or labels in a searchable list with a record count per value, the most records first; a record shows when it has any picked value;
  - **artist** is the record's credited artist as Discogs shows it ("Nightcraft", "Various", "Theo Parrish & Marcellus Pittman"), matched exactly, not split into names and not the track's own credit;
  - **label** is the record's first label;
  - style, artist and label filters, and the BPM range, all apply: an entry shows when it passes every filter that is set;
  - each is remembered per crate; play order, previews, record rows, the title bar count and `P` follow what's shown, as for styles.
- **Footer buttons, and the ☰ menu always:**
  - after the BPM control, the footer shows the style chips and then ARTISTS and LABELS buttons when they all fit. Otherwise it shows STYLES, ARTISTS and LABELS buttons when those fit, and otherwise nothing. A button is lit while its filter is set and shows how many values are picked ("ARTISTS 2");
  - ☰ gets Filter by style…, Filter by artist… and Filter by label…, which open the same lists at any width, and Show all records, which clears all three. Show all styles goes, replaced by Show all records. This also fixes the classic-width case where the STYLES button had no room.
- **Record artist on entries:** entries from Discogs now carry the record's credited artist (not only the track's). New sends get it with the record's details. Crates saved before get it from the disk cache when shown, with no request, as albums and styles were filled. Record rows show it, so a compilation reads "Various – ‹album›" instead of its first track's artist.
- Out of scope: these filters in other crates, splitting joined credits, labels after the first, and filtering by year or format.

## Capabilities

### New Capabilities
- `record-filters`: the artist and label filters, the shared rules for combining the style, artist, label and BPM filters, the footer buttons and their layout rule, and the ☰ entries (Filter by style/artist/label…, Show all records).

### Modified Capabilities
- `style-filter`: the control gives way to the record-filters layout (chips, then the buttons, then only ☰). Show all styles is replaced by Show all records, and Filter by style… opens the list at any width.
- `album-entries`: Discogs entries also carry the record's credited artist, saved with the crate and filled for older crates from the cache without requests.
- `record-view`: a record row names the record's credited artist when known.

## Impact

- `crates/ui`:
  - `playlist.rs`: `Origin.artist` (saved, `serde(default)`); `styles_on` generalised to one selection per facet (style, artist, label), with `Shown` checking all three; per-facet value counts; saved selections;
  - `app.rs`: the footer layout (chips or buttons or none), the facet lists (one popup with search, counts and Clear, reused for all three), the ☰ entries, record rows using the record artist;
  - `app/digging.rs`: `origin()` and `backfill()` carry the record artist; older crates are backfilled when the record artist is missing.
- `crates/dig`: no change (`RecordInfo.artist` already carries the release's or master's credited artist).
- README (filters, ☰ entries, the test count), help panel.
