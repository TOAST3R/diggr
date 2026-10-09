## MODIFIED Requirements

### Requirement: Page button
On Discogs release, master release, artist, label, wantlist, collection, list and marketplace listing pages, the extension SHALL show a button next to the page's title. When the page's title can't be found, the button SHALL float in a corner of the page. The button's menu SHALL offer:
- Play in ‹App›;
- Enqueue in ‹App›;
- Send to crate, listing the app's crates and New crate…;
- the "skip passed" switch, which SHALL be remembered. There SHALL be no "vinyl only" switch: every send keeps records in every format.

On a seller's page (`/seller/‹name›` and any page under it, `/sell/list` with a `seller=‹name›` parameter, or `/user/‹name›` with no further path; see `seller-crates`), the button SHALL be shown the same way, and its menu SHALL offer only ‹App›: Add seller, or ‹App›: In Top Sellers · Refresh when the seller is already in the app's Top Sellers. Choosing it SHALL send the page to the app, which adds or refreshes that seller.

On a label's page, the button's menu SHALL offer only ‹App›: Send label. Choosing it SHALL send the page to the app, which follows that label, or refreshes it when it is already followed (see `label-crates`).

New crate… SHALL open a field in the menu holding a suggested name, taken from the page's title, as editable text; Enter or Create SHALL send:
- for a release, master release or listing, "Artist - Title";
- for an artist, label or list, its name;
- for a wantlist, "Wantlist: ‹user›";
- for a collection, "Collection: ‹user›".

The suggestion SHALL leave out Discogs' name-variant asterisks and disambiguation numbers such as "(2)", SHALL use "-" between artist and title, and SHALL be at most 40 characters.

On other Discogs pages, no button SHALL be shown. The button SHALL follow Discogs' in-page navigation, appearing and disappearing as the address changes.

#### Scenario: Label page
- **WHEN** the user opens a Discogs label page while the app is running and paired
- **THEN** the button's menu offers only ‹App›: Send label, and choosing it follows the label in the app without changing its shown crate

#### Scenario: Followed label
- **WHEN** the user opens the page of a label the app already follows and chooses ‹App›: Send label
- **THEN** the label's crate is refreshed, and no second crate is made

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
- **THEN** the button's menu offers only ‹App›: Add seller, and choosing it adds decks.de to the app's TOP SELLERS and shows its crate

#### Scenario: Seller's catalogue
- **WHEN** the user opens `https://www.discogs.com/sell/list?seller=decks.de&style=Techno` or `https://www.discogs.com/seller/decks.de/mywants`
- **THEN** the button is shown, and its menu offers only ‹App›: Add seller

#### Scenario: Seller already followed
- **WHEN** the user opens a page of decks.de, which is in the app's TOP SELLERS
- **THEN** the menu offers only ‹App›: In Top Sellers · Refresh, and choosing it refreshes the seller's crate

#### Scenario: Not a supported page
- **WHEN** the user opens a Discogs forum thread
- **THEN** no button is shown

#### Scenario: No vinyl only switch
- **WHEN** the user opens the button's menu on an artist page
- **THEN** it offers the skip passed switch and no vinyl only switch

### Requirement: Links anywhere
Right-clicking a link to a supported Discogs page, on any website, SHALL offer Play in ‹App› and Enqueue in ‹App›, except a link to a label's page, which SHALL offer only ‹App›: Send label, and a link to a seller's page, which SHALL offer only ‹App›: Add seller.

#### Scenario: Forum link
- **WHEN** the user right-clicks a Discogs release link in a forum post and chooses Enqueue in ‹App›
- **THEN** the release's tracks are added to the app's shown crate

#### Scenario: Label link
- **WHEN** the user right-clicks a Discogs label link in a forum post
- **THEN** the menu offers only ‹App›: Send label, and choosing it follows the label in the app

#### Scenario: Seller link
- **WHEN** the user right-clicks a link to `https://www.discogs.com/seller/decks.de/profile` on a forum
- **THEN** the menu offers only ‹App›: Add seller, and choosing it adds decks.de to the app's TOP SELLERS
