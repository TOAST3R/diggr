## MODIFIED Requirements

### Requirement: Supported pages
The app SHALL accept Discogs web addresses of a release, a master release, an artist, a label, a user's wantlist, a user's collection (`/user/‹name›/collection`), a user list, a marketplace listing (`/shop/item/…` or `/sell/item/…`) and a seller (`/seller/‹name›/profile`, `/seller/‹name›`, or `/user/‹name›` with no further path), with or without `www.`, a language prefix (such as `/de/`), the name part after the id, a query string or a fragment. A marketplace listing SHALL be treated as the release it sells, looked up with at most one request to Discogs (none when that listing was looked up before). A seller's page SHALL add or refresh that seller (see `seller-crates`) instead of sending tracks, whatever the send mode. Any other address SHALL be refused with a message listing the supported kinds of page, and nothing SHALL be added.

#### Scenario: Language prefix
- **WHEN** the address is `https://www.discogs.com/de/label/12345-Lowtide-Tapes`
- **THEN** it is recognised as label 12345

#### Scenario: No name part
- **WHEN** the address is `https://discogs.com/release/123456`
- **THEN** it is recognised as release 123456

#### Scenario: Marketplace listing
- **WHEN** the address is `https://www.discogs.com/shop/item/3923678974`, a listing of release 123456
- **THEN** release 123456 is sent, exactly as if its release page had been sent

#### Scenario: Listing gone
- **WHEN** the address names a listing that Discogs doesn't know
- **THEN** the main window says the page wasn't found, and no crate changes

#### Scenario: A collection
- **WHEN** the address is `https://www.discogs.com/user/digger/collection`
- **THEN** it is recognised as digger's collection, and sending it adds one entry per clip of every record in it, like a wantlist

#### Scenario: A seller
- **WHEN** the address is `https://www.discogs.com/seller/decks.de/profile?genre=Electronic`
- **THEN** it is recognised as seller decks.de, and sending it adds decks.de to TOP SELLERS without adding tracks to the shown crate

#### Scenario: A user page
- **WHEN** the address is `https://www.discogs.com/user/logon`
- **THEN** it is recognised as seller logon

#### Scenario: Unsupported page
- **WHEN** the address is a Discogs forum thread
- **THEN** a message lists the supported kinds of page, and no crate changes
