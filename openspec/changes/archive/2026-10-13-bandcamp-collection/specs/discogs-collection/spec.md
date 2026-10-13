## MODIFIED Requirements

### Requirement: Owned mark
An entry SHALL be marked owned when its release is in the user's collection, or when another physical release of the same master release is. A digital release (format File) in the collection SHALL mark only itself: it SHALL NOT make another release of its master owned. An owned entry SHALL show a clearly visible "OWNED" badge at the start of its title, in the single-line format and in columns, without changing the row's colour. The badge's colour SHALL differ from the skin's playlist text and LCD colours (off-white in the default skin). Its tooltip SHALL say whether this pressing or another pressing is owned, naming the owned pressing's catalog number and year. The mark SHALL follow the latest cached collection, in every crate, without re-saving crates. A cached collection from before this rule SHALL be read whole again on its next sync, so each release's format is known.

#### Scenario: This pressing
- **WHEN** an entry from release 123456 is shown and release 123456 is in the collection
- **THEN** its row shows the OWNED badge, and its tooltip says "Owned: this pressing"

#### Scenario: Another pressing
- **WHEN** an entry from release 111 (master 900) is shown and the collection holds release 222 of master 900, catalog number AF001R, from 2019
- **THEN** its row shows the OWNED badge, and its tooltip says "Owned: another pressing (AF001R, 2019)"

#### Scenario: Digital only
- **WHEN** an entry from vinyl release 111 (master 900) is shown and the collection holds only release 333 of master 900, in format File
- **THEN** no badge is shown

#### Scenario: Digital release of an owned vinyl
- **WHEN** an entry from File release 333 (master 900) is shown and the collection holds vinyl release 222 of master 900
- **THEN** its row shows the OWNED badge, naming release 222's pressing

#### Scenario: Not owned
- **WHEN** an entry's release and master are not in the collection
- **THEN** no badge is shown

#### Scenario: Badge stands out
- **WHEN** an owned entry is shown in the default skin
- **THEN** its OWNED badge is off-white, not the amber of the playlist text

### Requirement: Collection crate
When a Discogs token is saved and no collection crate exists (none made from an earlier token, and none made by a Bandcamp import), the app SHALL send the user's collection into a new crate named "Collection: ‹username›" and show it, with the playlist opened. When a collection crate exists, the Discogs collection SHALL be sent into it instead. The collection SHALL also be sendable like any page, from its address (`/user/‹name›/collection`) or the browser extension. In a crate made from a collection, entries SHALL NOT show the OWNED badge. Like any send, its records' details SHALL be fetched nearest the playhead first, behind other work, and cached.

#### Scenario: First token
- **WHEN** the user saves a valid token for "digger" and has no collection crate
- **THEN** a crate "Collection: digger" is created, shown, and filled with the collection's records

#### Scenario: Token saved again
- **WHEN** the user saves a token again while "Collection: digger" exists
- **THEN** no other crate is created, and "Collection: digger" is left as it is

#### Scenario: After a Bandcamp import
- **WHEN** the user saves a valid token for "digger" while "Collection: toast3r" was made by a Bandcamp import
- **THEN** no other crate is created, and the Discogs collection is merged into "Collection: toast3r"

#### Scenario: No badge in it
- **WHEN** "Collection: digger" is shown and the collection is synced
- **THEN** none of its entries shows the OWNED badge

### Requirement: Owned records leave the wantlist
After every successful collection sync, every record on the user's Discogs wantlist that the collection owns, in that pressing or another physical pressing of the same master release, SHALL be removed from the Discogs wantlist, with one request per record. A digital release (format File) in the collection SHALL remove only itself from the wantlist, never another release of its master. A removed record SHALL also no longer be wanted, and its entries SHALL leave the wantlist crate unless the Bandcamp wishlist still holds them. The main window SHALL say how many records were removed ("Removed 2 records you now own from your wantlist"). A sync that changes nothing SHALL remove nothing and say nothing.

#### Scenario: Bought on discogs.com
- **WHEN** the user bought release 123456 on discogs.com while it was on their wantlist, and the next sync finds it in the collection
- **THEN** release 123456 is removed from the Discogs wantlist and the wantlist crate, and the main window says "Removed 1 record you now own from your wantlist"

#### Scenario: Another pressing bought
- **WHEN** release 111 (master 900) is wanted and a sync finds release 222 of master 900 in the collection
- **THEN** release 111 is removed from the Discogs wantlist

#### Scenario: Files bought, vinyl still wanted
- **WHEN** vinyl release 111 (master 900) is wanted and a sync finds only File release 333 of master 900 in the collection
- **THEN** release 111 stays on the Discogs wantlist and in the wantlist crate

#### Scenario: Nothing new
- **WHEN** a sync finds no owned record on the wantlist
- **THEN** no wantlist request is sent and no message is shown
