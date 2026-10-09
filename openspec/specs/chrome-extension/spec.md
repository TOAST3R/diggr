# chrome-extension Specification

## Purpose
A thin Chrome extension that sends the Discogs page you're on, or a Discogs link, to the player in one click, and shows what the player is doing.
## Requirements
### Requirement: Page button
On Discogs release, master release, artist, label, wantlist, collection, list and marketplace listing pages, the extension SHALL show a button next to the page's title. When the page's title can't be found, the button SHALL float in a corner of the page. The button's menu SHALL offer:
- Play in ‹App›;
- Enqueue in ‹App›;
- Send to crate, listing the app's crates and New crate…;
- the "skip passed" switch, which SHALL be remembered. There SHALL be no "vinyl only" switch: every send keeps records in every format.

On a seller's page (`/seller/‹name›/profile`, `/seller/‹name›`, or `/user/‹name›` with no further path), the button SHALL be shown the same way, and its menu SHALL offer only Add seller to ‹App›. Choosing it SHALL send the page to the app, which adds or refreshes that seller (see `seller-crates`).

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
- **THEN** the button's menu offers only Add seller to ‹App›, and choosing it adds decks.de to the app's TOP SELLERS and shows its crate

#### Scenario: Not a supported page
- **WHEN** the user opens a Discogs forum thread
- **THEN** no button is shown

#### Scenario: No vinyl only switch
- **WHEN** the user opens the button's menu on an artist page
- **THEN** it offers the skip passed switch and no vinyl only switch

### Requirement: Links anywhere
Right-clicking a link to a supported Discogs page, on any website, SHALL offer Play in ‹App› and Enqueue in ‹App›, except a link to a label's page, which SHALL offer only ‹App›: Send label.

#### Scenario: Forum link
- **WHEN** the user right-clicks a Discogs release link in a forum post and chooses Enqueue in ‹App›
- **THEN** the release's tracks are added to the app's shown crate

#### Scenario: Label link
- **WHEN** the user right-clicks a Discogs label link in a forum post
- **THEN** the menu offers only ‹App›: Send label, and choosing it follows the label in the app

### Requirement: Feedback
After a send from a Discogs page, the page SHALL show, for 3 s, a confirmation naming the page and the crate. After a send from a link on another site, the extension's toolbar button SHALL show a mark for 3 s: success or failure. Clicking the toolbar button SHALL show whether the app is running and paired, what is playing, and sends in progress. When the app isn't running, the extension isn't paired, or the page isn't supported, the extension SHALL say so and how to fix it.

#### Scenario: Confirmation
- **WHEN** the user enqueues a label page
- **THEN** the page shows "Sent to ‹App›: Label: Lowtide Tapes → ‹crate›" for 3 s

#### Scenario: App not running
- **WHEN** the user chooses Enqueue in ‹App› while the app is closed
- **THEN** a message says that the app isn't running, and nothing else happens

#### Scenario: Not paired
- **WHEN** the user chooses an action before the extension is paired
- **THEN** the pairing screen opens and asks for the code shown in Options ▸ Browser…

### Requirement: Minimal access
The extension SHALL ask for access only to discogs.com, the subdomains of bandcamp.com and 127.0.0.1, and for its own storage, context menu and toolbar button. It SHALL send the app only the page's address, the chosen mode, the crate and the switches, and, for a Bandcamp page, its title. It SHALL contact no other server, SHALL read nothing from pages beyond their address and title, and SHALL contain no remote code.

#### Scenario: Permissions at install
- **WHEN** the extension is loaded in Chrome
- **THEN** the only site access it lists is discogs.com, *.bandcamp.com and 127.0.0.1

### Requirement: The app's name
Every label that names the player (for example "Play in ‹App›") SHALL use the name the app reports, so renaming the app renames them.

#### Scenario: Renamed app
- **WHEN** the app reports its name as "My Player"
- **THEN** the menus read "Play in My Player" and "Enqueue in My Player"

### Requirement: Owned note
On release, master release and marketplace item pages, when the app says the page's record is owned, the extension SHALL show under its button "✓ In your collection", or "✓ Another pressing in your collection (‹catalog number›, ‹year›)". When the answer is "checking", it SHALL ask once more after 3 s. When the app has no Discogs token, it SHALL show, dimmed, that a token in the app (Options ▸ Discogs…) is needed to see owned records. When the record isn't owned, or the answer is unknown, it SHALL show nothing. It SHALL send the app only the page's address for this.

#### Scenario: Owned on a shop page
- **WHEN** the user opens a marketplace item of a record they own
- **THEN** within a few seconds the button shows "✓ In your collection"

#### Scenario: No token in the app
- **WHEN** the user opens a release page while the app has no Discogs token
- **THEN** a dimmed line under the button says to add a Discogs token in the app to see owned records

#### Scenario: Not owned
- **WHEN** the user opens a release they don't own
- **THEN** no note is shown

### Requirement: Bandcamp page button
On a Bandcamp album or track page, the extension SHALL show its button next to the page's title (floating in a corner when the title can't be found). Its menu SHALL be the same as on a Discogs release: Play, Enqueue, Send to crate and skip passed. New crate… SHALL suggest "Artist - Title" from the page's title. On a Bandcamp label or artist page (`/` or `/music`), its menu SHALL offer only ‹App›: Send label. On other Bandcamp pages, no button SHALL be shown.

#### Scenario: Album page
- **WHEN** the user opens a Bandcamp album page while the app is running and paired
- **THEN** the button offers Play in ‹App›, Enqueue in ‹App› and Send to crate

#### Scenario: Label page
- **WHEN** the user opens `https://analogicalforce.bandcamp.com/music`
- **THEN** the button's menu offers only ‹App›: Send label, and choosing it follows the label in the app without changing its shown crate

#### Scenario: Merch page
- **WHEN** the user opens a Bandcamp merch page
- **THEN** no button is shown

### Requirement: Bandcamp links
Right-clicking a link to a Bandcamp album or track page, on any website, SHALL offer Play in ‹App› and Enqueue in ‹App›. A link to a Bandcamp label page (`/` or `/music`) SHALL offer only ‹App›: Send label.

#### Scenario: Album link in a forum
- **WHEN** the user right-clicks a Bandcamp album link and chooses Enqueue in ‹App›
- **THEN** its tracks are merged into the app's shown crate, and the toolbar button shows success for 3 s

