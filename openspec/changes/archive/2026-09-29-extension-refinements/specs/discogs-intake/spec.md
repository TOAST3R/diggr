## MODIFIED Requirements

### Requirement: Supported pages
The app SHALL accept Discogs web addresses of a release, a master release, an artist, a label, a user's wantlist, a user list and a marketplace listing (`/shop/item/…` or `/sell/item/…`), with or without `www.`, a language prefix (such as `/de/`), the name part after the id, a query string or a fragment. A marketplace listing SHALL be treated as the release it sells, looked up with at most one request to Discogs (none when that listing was looked up before). Any other address SHALL be refused with a message listing the supported kinds of page, and nothing SHALL be added.

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

#### Scenario: Unsupported page
- **WHEN** the address is a Discogs forum thread
- **THEN** a message lists the supported kinds of page, and no crate changes
