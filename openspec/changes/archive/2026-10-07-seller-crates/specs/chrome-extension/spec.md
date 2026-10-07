## MODIFIED Requirements

### Requirement: Page button
On Discogs release, master release, artist, label, wantlist, collection, list and marketplace listing pages, the extension SHALL show a button next to the page's title. When the page's title can't be found, the button SHALL float in a corner of the page. The button's menu SHALL offer:
- Play in ‹App›;
- Enqueue in ‹App›;
- Send to crate, listing the app's crates and New crate…;
- the "skip passed" switch, which SHALL be remembered. There SHALL be no "vinyl only" switch: every send keeps records in every format.

On a seller's page (`/seller/‹name›/profile`, `/seller/‹name›`, or `/user/‹name›` with no further path), the button SHALL be shown the same way, and its menu SHALL offer only Add seller to ‹App›. Choosing it SHALL send the page to the app, which adds or refreshes that seller (see `seller-crates`).

New crate… SHALL open a field in the menu holding a suggested name, taken from the page's title, as editable text; Enter or Create SHALL send:
- for a release, master release or listing, "Artist - Title";
- for an artist, label or list, its name;
- for a wantlist, "Wantlist: ‹user›";
- for a collection, "Collection: ‹user›".

The suggestion SHALL leave out Discogs' name-variant asterisks and disambiguation numbers such as "(2)", SHALL use "-" between artist and title, and SHALL be at most 40 characters.

On other Discogs pages, no button SHALL be shown. The button SHALL follow Discogs' in-page navigation, appearing and disappearing as the address changes.

#### Scenario: Label page
- **WHEN** the user opens a Discogs label page while the app is running and paired, and chooses Enqueue in ‹App›
- **THEN** the label's tracks are added to the app's shown crate

#### Scenario: Send to a new crate
- **WHEN** the user chooses Send to crate, then New crate…, and enters "Friday"
- **THEN** a crate "Friday" is created in the app with the page's tracks

#### Scenario: Suggested name
- **WHEN** the user chooses New crate… on the release page titled "D'Arcangelo* – TimeLss | Releases | Discogs"
- **THEN** the field already holds "D'Arcangelo - TimeLss", and Enter sends it

#### Scenario: Listing page
- **WHEN** the user opens `https://www.discogs.com/shop/item/3923678974` and chooses Enqueue in ‹App›
- **THEN** the button is there, and the tracks of the release on sale are added to the shown crate

#### Scenario: Collection page
- **WHEN** the user opens `https://www.discogs.com/user/digger/collection` and chooses New crate…
- **THEN** the field holds "Collection: digger", and Enter creates that crate in the app with the collection's tracks, on screen

#### Scenario: Seller page
- **WHEN** the user opens `https://www.discogs.com/seller/decks.de/profile` while the app is running and paired
- **THEN** the button's menu offers only Add seller to ‹App›, and choosing it adds decks.de to the app's TOP SELLERS and shows its crate

#### Scenario: Not a supported page
- **WHEN** the user opens a Discogs forum thread
- **THEN** no button is shown

#### Scenario: No vinyl only switch
- **WHEN** the user opens the button's menu on a label page
- **THEN** it offers the skip passed switch and no vinyl only switch
