## MODIFIED Requirements

### Requirement: Supported pages
The app SHALL accept Discogs web addresses of a release, a master release, an artist, a label, a user's wantlist, a user's collection (`/user/‹name›/collection`), a user list and a marketplace listing (`/shop/item/…` or `/sell/item/…`), with or without `www.`, a language prefix (such as `/de/`), the name part after the id, a query string or a fragment. A marketplace listing SHALL be treated as the release it sells, looked up with at most one request to Discogs (none when that listing was looked up before). Any other address SHALL be refused with a message listing the supported kinds of page, and nothing SHALL be added.

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

#### Scenario: Unsupported page
- **WHEN** the address is a Discogs forum thread
- **THEN** a message lists the supported kinds of page, and no crate changes

### Requirement: Sending a page
A page SHALL be sent in one of three modes:
- Enqueue: its tracks are added to the end of the shown crate.
- Play: a new crate is named after the page (for example "Label: Lowtide Tapes"), shown, and its first track plays as soon as its preview is ready. When a crate with that name exists, it is reused.
- Crate: its tracks are added to the named crate, which is created if it doesn't exist. A crate created this way SHALL be shown; sending to a crate that already exists SHALL NOT change the shown crate.

Whenever a send shows a crate, the playlist SHALL be opened if it was hidden.

Pasting a Discogs address in the player window (Cmd+V, or Ctrl+V on Linux and Windows) SHALL send it with Enqueue; text that isn't a Discogs address SHALL be ignored. Each send SHALL be acknowledged in the main window with the page's name, and its progress SHALL be shown until it finishes (for example "Lowtide Tapes: 120 of 312 releases").

#### Scenario: Paste a label
- **WHEN** the user copies a label's Discogs address and presses Cmd+V in the player window
- **THEN** the label's tracks are added to the end of the shown crate, and the main window names the label

#### Scenario: Play as a new crate
- **WHEN** a label page is sent with Play
- **THEN** a crate "Label: Lowtide Tapes" is created and shown, and its first track plays as soon as its preview is ready

#### Scenario: Send to a new crate
- **WHEN** an artist page is sent to crate "Friday", which doesn't exist yet, while the playlist is hidden
- **THEN** a crate "Friday" is created with the artist's tracks, shown, and the playlist opens

#### Scenario: Send to an existing crate
- **WHEN** an artist page is sent to crate "Friday", which exists, while crate "Playlist" is shown
- **THEN** the tracks are added to "Friday", and "Playlist" stays on screen
