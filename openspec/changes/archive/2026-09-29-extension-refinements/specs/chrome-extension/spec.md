## MODIFIED Requirements

### Requirement: Page button
On Discogs release, master release, artist, label, wantlist, list and marketplace listing pages, the extension SHALL show a button next to the page's title. When the page's title can't be found, the button SHALL float in a corner of the page. The button's menu SHALL offer:
- Play in ‹App›;
- Enqueue in ‹App›;
- Send to crate, listing the app's crates and New crate…;
- the "vinyl only" and "skip passed" switches, which SHALL be remembered.

New crate… SHALL suggest a name taken from the page's title, which the user can edit:
- for a release, master release or listing, "Artist - Title";
- for an artist, label or list, its name;
- for a wantlist, "Wantlist: ‹user›".

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
- **THEN** the prompt already holds "D'Arcangelo - TimeLss"

#### Scenario: Listing page
- **WHEN** the user opens `https://www.discogs.com/shop/item/3923678974` and chooses Enqueue in ‹App›
- **THEN** the button is there, and the tracks of the release on sale are added to the shown crate

#### Scenario: Not a supported page
- **WHEN** the user opens a Discogs forum thread
- **THEN** no button is shown
