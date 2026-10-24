## MODIFIED Requirements

### Requirement: Narrow down
When a seller has more than 1,000 copies for sale, the Narrow down modal SHALL offer these criteria:
- a genre, chosen from a dropdown of Discogs' genres with "Any genre" first, showing every genre without scrolling;
- a style, chosen from a dropdown (see "Style dropdown");
- a format, chosen from a dropdown: Any format, Vinyl, CD or Cassette;
- a year, typed as four digits;
- search text;
- "newest N" (N from 50 to 1,000).

Discogs SHALL apply every criterion, so the modal SHALL never read pages of listings before Dig: each change SHALL cost at most one count request, sent at once for a dropdown and once typing pauses for the year or the search text.

A genre and a style SHALL be searched together. Changing the genre SHALL clear a style that isn't one of the new genre's styles. They SHALL be sent in the inventory search as the genre in double quotes, then the style in double quotes, then the format's name in lower case, then the year, then the search text, each part left out when empty (`"Rock" "Punk" vinyl 2026 ostgut`). Every count and every inventory read for the seller SHALL use that same search. The dropdowns SHALL send no request of their own and SHALL show no per-genre or per-style totals.

It SHALL show the number of copies matching the current criteria, as Discogs counts them. Dig SHALL be enabled only when 1 to 5,000 copies match. When more than 1,000 match, the button SHALL read "Dig the newest 1,000", the modal SHALL say that the rest come 1,000 at a time when asked, and the dig SHALL bring the newest 1,000 only (see "Add next 1,000"). When more than 5,000 match, the modal SHALL ask to narrow it down to 5,000 or fewer. When more than 10,000 copies match, the modal SHALL say that Discogs only shows the first 10,000 and suggest a genre or style, a format, a year, search text or newest N, and SHALL NOT present those 10,000 as the number that match. The criteria SHALL be saved with the seller and reused by every refresh. Right-clicking a dug seller crate SHALL offer Narrow down… to change them. A seller saved with criteria this modal no longer offers (format groups, a price range, a minimum condition, countries the copies ship from) SHALL load, with those criteria ignored.

#### Scenario: Search narrows
- **WHEN** decks.de has 40,728 copies and the user types "techno"
- **THEN** the count shows 8,389 and Dig stays disabled

#### Scenario: Style narrows the whole stock
- **WHEN** www.hhv.de has 159,648 copies and the user picks the style Deep House
- **THEN** one count is requested with the search `"Deep House"`, the count shows its answer (4,429), and Dig stays disabled

#### Scenario: Style and text together
- **WHEN** the user picks the style Deep House and types "vinyl"
- **THEN** the count and the dig use the search `"Deep House" vinyl`

#### Scenario: Genre and style together
- **WHEN** the user picks the genre Rock, the style Punk and the format Vinyl
- **THEN** the count and the dig use the search `"Rock" "Punk" vinyl`

#### Scenario: A style of another genre
- **WHEN** the style is Deep House and the user picks the genre Rock
- **THEN** the style becomes "Any style", and the count uses the search `"Rock"`

#### Scenario: Between 1,000 and 5,000
- **WHEN** www.hhv.de has 3,412 vinyl copies matching and the user picks Vinyl
- **THEN** the button reads "Dig the newest 1,000" and is enabled, and Dig brings the 1,000 newest copies only

#### Scenario: Newest
- **WHEN** the user picks newest 500 for decks.de
- **THEN** the count shows 500 and Dig is enabled

#### Scenario: Format and year
- **WHEN** www.hhv.de has 159,648 copies and the user picks the genre Electronic, the format Vinyl and types the year 2025
- **THEN** the count uses the search `"Electronic" vinyl 2025` and shows its answer (6,555), and no page of listings is read

#### Scenario: Nothing waits for listings
- **WHEN** the user changes any criterion in Narrow down
- **THEN** at most one count request is sent, and no "Reading listings" progress is shown

#### Scenario: Beyond 10,000
- **WHEN** the user double-clicks "Seller: www.hhv.de", which has 159,183 copies for sale
- **THEN** the modal says Discogs shows only the first 10,000, suggests a genre, a style, search text or newest N, and does not say "10,000 match"

#### Scenario: Saved style is reused
- **WHEN** www.hhv.de was dug with the style Dub Techno, and the user chooses Refresh seller
- **THEN** the refresh counts and reads with the search `"Dub Techno"`

#### Scenario: Old saved countries
- **WHEN** a seller was saved with criteria that include ships-from countries
- **THEN** the seller list loads, the seller keeps its other criteria, and its next refresh does not filter by country, format group, price or condition

### Requirement: Refreshing
Right-clicking a dug seller crate, in the sidebar or the title-bar crate menu, SHALL offer Refresh seller. It SHALL refresh the crate whenever it was last dug. A refresh SHALL read the seller's listings again with the saved criteria, as many of the newest as the crate holds (all of them, or the 1,000s Add next brought), and then:
- bring in new copies, expanding their records if the crate doesn't hold them;
- mark copies no longer listed as SOLD, keeping them;
- update changed prices.

The main window SHALL then summarise ("decks.de: 14 new, 6 sold, 3 cheaper", or "decks.de: up to date"). If more than 1,000 copies now match a crate dug whole, or more than 5,000 a crate dug 1,000 at a time, the refresh SHALL change nothing and open the Narrow down modal with the saved criteria. While it runs, the item SHALL read "Refreshing…" and be disabled. A failed refresh SHALL change nothing and say why.

#### Scenario: New and sold
- **WHEN** since the last dig, a seller listed 2 new copies and sold 1, and the user chooses Refresh seller
- **THEN** the 2 copies come in, the sold one is marked SOLD and stays, and the main window says "‹seller›: 2 new, 1 sold"

#### Scenario: Grew past the limit
- **WHEN** a seller dug with no criteria at 950 copies now has 1,120, and the crate is refreshed
- **THEN** the crate is unchanged and the Narrow down modal opens

#### Scenario: A crate dug 1,000 at a time
- **WHEN** a crate holds the newest 2,000 of 3,412 matching copies and is refreshed
- **THEN** the newest 2,000 are read again, and Narrow down opens only if more than 5,000 now match

#### Scenario: Offline
- **WHEN** Discogs is unreachable and the user chooses Refresh seller
- **THEN** the crate is unchanged, and the main window says the refresh failed because Discogs is offline

## ADDED Requirements

### Requirement: Style dropdown
The style dropdown SHALL hold "Any style" and a search field, then up to three sections, in this order, each listing styles by name:
- **in this crate**: the styles of the records already in this seller's crate, shown only when it holds any;
- **yours**: the styles of the records in the user's wantlist and collection crates, shown only when they hold any;
- **all**: every Discogs style, from a fixed list shipped with the app.

While a genre is picked, every section SHALL list only that genre's styles.

The first two sections SHALL list styles by the number of records that have them, most first, then by name. Typing in the search field SHALL narrow every section to styles containing the text, ignoring case, within one frame. Picking a style SHALL select it and close the dropdown. "Any style" SHALL clear the style. Opening the dropdown and searching it SHALL send no request to Discogs.

#### Scenario: First dig of a big seller
- **WHEN** the user opens the style dropdown for www.hhv.de, never dug, and their collection has Deep House and Electro records
- **THEN** the dropdown shows "Any style", the "yours" section with Deep House and Electro, and the "all" section; there is no "in this crate" section

#### Scenario: Search the styles
- **WHEN** the user types "deep" in the style dropdown's search field
- **THEN** only styles containing "deep" are listed in each section, such as "Deep House" and "Deep Techno"

#### Scenario: Genre narrows the list
- **WHEN** the genre is Rock and the user types "punk" in the style dropdown's search field
- **THEN** Rock styles containing "punk" are listed, such as "Punk" and "Pop Punk", and no style of another genre is listed in any section

#### Scenario: Re-narrowing a dug seller
- **WHEN** the user chooses Narrow down… on "Seller: logon", whose crate holds records in Minimal and Dub Techno
- **THEN** the style dropdown's first section, "in this crate", lists Minimal and Dub Techno

### Requirement: Add next 1,000
A seller crate dug with more than 1,000 matching copies SHALL end its list with a row "+ ADD NEXT 1,000 · ‹N› MORE" (N: the matching copies not in the crate yet, counting up to 5,000 in all), or "+ ADD THE LAST ‹N›" when 1,000 or fewer are left, as soon as the crate's listings are read: its records MAY still be expanding (a request each, many minutes for 1,000), and the row SHALL NOT wait for them. It SHALL be hidden only while a dig, an Add next or a refresh is still reading listing pages for the crate. A refresh SHALL be applied once its listing pages are read, not after its records are expanded. Nothing more SHALL be read until the user clicks the row. A click SHALL read only the next pages of listings after the ones the crate holds and bring their copies and records into the crate as they arrive, as a dig does; the crate's limit SHALL grow by 1,000, saved with the seller, and refreshes SHALL keep it. The row SHALL hold no audio: play order, the arrows and the previews SHALL skip it. When nothing is left, the row SHALL go.

#### Scenario: The next 1,000
- **WHEN** a crate holds the newest 1,000 of 3,412 matching copies and the user clicks "+ ADD NEXT 1,000 · 2,412 MORE"
- **THEN** pages 11 to 20 are read (and no earlier page), the crate holds 2,000 copies, and the row reads "+ ADD NEXT 1,000 · 1,412 MORE"

#### Scenario: Records still expanding
- **WHEN** the newest 1,000 of 3,412 copies are in, and 10 of their 961 records are expanded
- **THEN** the row "+ ADD NEXT 1,000 · 2,412 MORE" is shown

#### Scenario: Only on a click
- **WHEN** a dig brought the newest 1,000 of 3,412 copies and the user does nothing
- **THEN** no page after the tenth is read

#### Scenario: The last ones
- **WHEN** a crate holds 3,000 of 3,412 matching copies and the user clicks "+ ADD THE LAST 412"
- **THEN** the crate holds 3,412 copies and the row is gone

### Requirement: New search
Right-clicking a dug seller crate SHALL offer New search…. It SHALL ask first, saying the crate is cleaned (its records and their downloaded previews removed) and that nothing changes on Discogs, with "Clean and search" and Cancel. Cancel SHALL change nothing. Confirmed, it SHALL stop any dig, Add next or refresh running for the crate, remove every entry and copy from the crate, delete the downloaded previews of its entries unless another crate holds them, forget the seller's saved criteria, limit and last dig, and then count the seller's copies and open Dig / Narrow down as for a seller never dug.

#### Scenario: Cancel
- **WHEN** the user chooses New search… on "logon" and clicks Cancel
- **THEN** the crate, its copies and the seller's saved search are unchanged

#### Scenario: Clean and search
- **WHEN** the user chooses New search… on "logon", dug with the style Acid, and clicks "Clean and search"
- **THEN** the crate is empty, its previews are deleted from the cache, the seller has no saved search, and Dig / Narrow down opens with the seller's count

#### Scenario: A preview another crate holds
- **WHEN** a track of the seller crate was also sent to "Keepers", and the user confirms New search…
- **THEN** that track's preview stays in the cache

### Requirement: Dig progress
While a dig, Add next or refresh runs, the main window's status line SHALL say, per send: while its listing is read, the records it added so far ("www.hhv.de: reading the listing · 380 records added"); then how many of those records have their tracks, the tracks added so far, and the last record filled with its number of tracks ("www.hhv.de: tracks for 134 of 1,919 records · 1,204 tracks · +11 Impulse – One-Six-Four-One-Seven"). A record row whose tracklist isn't read yet SHALL say "tracks coming…" instead of a track count, blinking (shown, then faint, half a second each), and its count once its tracks arrive. When nothing is playing or selected in the shown crate, the first record in view SHALL have its tracks read first.

#### Scenario: Reading the listing
- **WHEN** a dig of www.hhv.de has read 4 pages adding 380 records
- **THEN** the status line says "www.hhv.de: reading the listing · 380 records added"

#### Scenario: Filling tracks
- **WHEN** 134 of 1,919 records have their tracks, 1,204 in all, and the last one filled was Impulse – One-Six-Four-One-Seven with 11
- **THEN** the status line says "www.hhv.de: tracks for 134 of 1,919 records · 1,204 tracks · +11 Impulse – One-Six-Four-One-Seven"

#### Scenario: A record not filled yet
- **WHEN** a record's tracklist hasn't been read
- **THEN** its row says "tracks coming…", and "11 tracks" once its 11 tracks arrive

