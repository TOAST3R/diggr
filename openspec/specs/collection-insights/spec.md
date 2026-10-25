# collection-insights Specification

## Purpose
Shows what the user's Discogs collection says about them, counted over its vinyl records: years, styles, a style × decade heatmap, top labels (with their totals on Discogs and Follow) and artists, value and rarity, and tempos, in a window opened from the collection crate, where every bar filters the collection.
## Requirements
### Requirement: Open Insights
Right-clicking the collection crate, in the sidebar or the title-bar crate menu, SHALL offer Open Insights. It SHALL open the Insights window, a window drawn in the skin's font and colours like the EQ and spectrogram windows, closed with its × or Escape. The window SHALL count vinyl records only: a record of the collection crate counts when "Vinyl" is among its formats, and once however many entries it has. Its title SHALL say how many it counts ("INSIGHTS · 1,081 vinyl records"). It SHALL follow the collection crate: when the crate changes (a refresh adds records, analysis adds tempos), its numbers SHALL be recomputed within one second, and recomputing SHALL NOT block a frame for more than 16 ms for a collection of 5,000 records.

#### Scenario: Open
- **WHEN** the user right-clicks the collection crate and chooses Open Insights
- **THEN** the Insights window opens, titled with the number of vinyl records

#### Scenario: Vinyl only
- **WHEN** the collection holds 1,116 records, of which 18 are CD only and 16 File only and one is "Vinyl, CD"
- **THEN** the window counts 1,082 records, the "Vinyl, CD" one among them

#### Scenario: Not the collection
- **WHEN** the user right-clicks the wantlist crate or a crate of their own
- **THEN** the menu offers no Open Insights

### Requirement: Charts
The window SHALL show, each from the counted records:
- **Years:** the number of records per release year, a bar per year from the earliest to the latest, grouped in decade bands labelled "90s", "00s"…; records with no year left out;
- **Styles:** the number of records per style, most first, the 15 largest;
- **Style × decade:** a heatmap of the 8 largest styles by decade, each cell's shade following its number of records, the number shown on hover;
- **Top labels** and **top artists:** the 10 with the most records, "Various" and "Unknown Artist" left out of artists;
- **Value:** the sum of the records' lowest prices for sale on Discogs (in the currency most of them use, the records priced in another left out and counted), the median, the five dearest records, and how many records have at most one copy for sale;
- **Tempo:** a histogram of the analysed tracks' BPM in 10-BPM buckets, saying how many tracks it counts.

Charts with nothing to show SHALL say so ("No tracks analysed yet") instead of drawing empty.

#### Scenario: Styles
- **WHEN** the counted records have Techno on 482, Drum n Bass on 209 and Breakbeat on 200
- **THEN** the Styles chart lists Techno, then Drum n Bass, then Breakbeat, with those numbers

#### Scenario: Value
- **WHEN** 1,057 counted records have a lowest price in EUR summing to €17,815.45
- **THEN** the Value part says about €17,815 for 1,057 priced records, and lists the five dearest

#### Scenario: No tempos
- **WHEN** no track of the collection has been analysed
- **THEN** the Tempo part says "No tracks analysed yet"

### Requirement: Bars are filters
Clicking a bar or row SHALL show the collection crate with the playlist's filters set to it, every other filter cleared:
- a year: the year filter set to that year;
- a decade band: the year filter set to every year of the decade that has records;
- a style: the style filter set to that style;
- a heatmap cell: the style filter to its style and the year filter to its decade's years;
- a label or an artist: the label or artist filter set to it;
- a dearest or rare record: no filter, that record selected and scrolled into view.

The window SHALL stay open. Hovering a bar or row SHALL show its number and say what a click does.

#### Scenario: Style bar
- **WHEN** the user clicks the Acid bar
- **THEN** the collection crate is shown with the style filter set to Acid only, and lists only Acid records

#### Scenario: Decade
- **WHEN** the user clicks the 90s band
- **THEN** the collection crate is shown with the year filter set to the 90s years that have records

#### Scenario: A dear record
- **WHEN** the user clicks The Sonics in the five dearest
- **THEN** the collection crate is shown unfiltered with that record selected and in view

### Requirement: Label loyalty
Each top label SHALL read "‹n› of ‹N› releases", N being the label's number of releases on Discogs, and offer Follow, which follows the label as Follow label does (see `label-crates`); a label already followed SHALL read "Following" instead. N SHALL cost at most one request per label to Discogs once its id is known, and the label's id SHALL come from a release of that label already in the cache, else from reading one such release. Totals SHALL be cached for 7 days. Until a total arrives the row SHALL read "‹n› of …"; without a token or offline it SHALL read "‹n› records".

#### Scenario: Totals
- **WHEN** the Insights window opens and Analogical Force has 35 counted records and 61 releases on Discogs
- **THEN** its row reads "35 of 61 releases" once the total arrives, after at most one request for it

#### Scenario: Follow
- **WHEN** the user clicks Follow on Analogical Force
- **THEN** its label crate is made and fills as when following a label, and the row reads "Following"

#### Scenario: Cached
- **WHEN** the window is opened again the next day
- **THEN** the totals show at once and no request is sent for them

