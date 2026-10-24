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

It SHALL show the number of copies matching the current criteria, as Discogs counts them. Dig SHALL be enabled only when 1 to 1,000 copies match. When more than 10,000 copies match, the modal SHALL say that Discogs only shows the first 10,000 and suggest a genre or style, a format, a year, search text or newest N, and SHALL NOT present those 10,000 as the number that match. The criteria SHALL be saved with the seller and reused by every refresh. Right-clicking a dug seller crate SHALL offer Narrow down… to change them. A seller saved with criteria this modal no longer offers (format groups, a price range, a minimum condition, countries the copies ship from) SHALL load, with those criteria ignored.

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
