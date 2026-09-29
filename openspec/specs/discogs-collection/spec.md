# discogs-collection Specification

## Purpose
Knows the user's Discogs collection (through their token), with as few requests as possible, so records they already own are marked and never bought twice, and so the collection itself can become a crate.
## Requirements
### Requirement: Collection sync
With a Discogs token set, the app SHALL keep the user's collection cached on disk: the owned releases, and for each master release its owned pressings. It SHALL sync it only:
- when a crate holding Discogs entries is shown and the cache is missing or older than 7 days, and never before the window is interactive;
- when the user asks from OPT ▸ Discogs…

The first sync SHALL use at most one request per 100 records. Later syncs SHALL fetch only the records added since the last one, newest first, stopping at the first record already known, and SHALL fetch the whole collection again only when records were removed. No request SHALL be made to find a record's other pressings. Syncing SHALL wait behind any page expansion that is waiting. A sync that fails SHALL keep the previous cache. OPT ▸ Discogs… SHALL show how many records the collection holds and how old it is, or why it isn't available. Without a token, nothing SHALL be fetched and nothing marked.

#### Scenario: First sync
- **WHEN** a token for a user with 1,234 records is set
- **THEN** the collection is fetched with 13 requests, and OPT ▸ Discogs… shows "Collection: 1,234 records, updated just now"

#### Scenario: A week later
- **WHEN** a crate of Discogs entries is shown with a cache 8 days old, and 3 records were added to the collection since
- **THEN** the sync takes 1 request, and those 3 records are marked from then on

#### Scenario: A record sold
- **WHEN** a sync finds that the collection's total is lower than the cached count plus the new records
- **THEN** the whole collection is fetched again, and the sold record is no longer marked

#### Scenario: Nothing to mark
- **WHEN** the app is launched with a token and an old cache, and only local files are shown
- **THEN** no collection request is sent

#### Scenario: No token
- **WHEN** no token is set and a crate from Discogs is shown
- **THEN** no collection request is ever sent, no entry is marked owned, and the main window says once that a Discogs token (OPT ▸ Discogs…) would mark the records already owned

#### Scenario: Recent cache
- **WHEN** the app is launched with a collection cache 2 days old
- **THEN** no collection request is sent, and the cached collection marks the entries

#### Scenario: Offline
- **WHEN** a refresh fails because Discogs is unreachable
- **THEN** the cached collection stays in use, and the dialog says the refresh failed

### Requirement: Owned mark
An entry SHALL be marked owned when its release is in the user's collection, or when another release of the same master release is. An owned entry SHALL show a clearly visible "OWNED" badge at the start of its title, in the single-line format and in columns, without changing the row's colour. Its tooltip SHALL say whether this pressing or another pressing is owned, naming the owned pressing's catalog number and year. The mark SHALL follow the latest cached collection, in every crate, without re-saving crates.

#### Scenario: This pressing
- **WHEN** an entry from release 123456 is shown and release 123456 is in the collection
- **THEN** its row shows the OWNED badge, and its tooltip says "Owned: this pressing"

#### Scenario: Another pressing
- **WHEN** an entry from release 111 (master 900) is shown and the collection holds release 222 of master 900, catalog number AF001R, from 2019
- **THEN** its row shows the OWNED badge, and its tooltip says "Owned: another pressing (AF001R, 2019)"

#### Scenario: Not owned
- **WHEN** an entry's release and master are not in the collection
- **THEN** no badge is shown

### Requirement: Collection crate
When a Discogs token is saved and no crate named "Collection: ‹username›" exists, the app SHALL send the user's collection into a new crate of that name and show it, with the playlist opened. The collection SHALL also be sendable like any page, from its address (`/user/‹name›/collection`) or the browser extension. In a crate made from a collection, entries SHALL NOT show the OWNED badge. Like any send, its records' details SHALL be fetched nearest the playhead first, behind other work, and cached.

#### Scenario: First token
- **WHEN** the user saves a valid token for "digger" and has no "Collection: digger" crate
- **THEN** a crate "Collection: digger" is created, shown, and filled with the collection's records

#### Scenario: Token saved again
- **WHEN** the user saves a token again while "Collection: digger" exists
- **THEN** no other crate is created, and "Collection: digger" is left as it is

#### Scenario: No badge in it
- **WHEN** "Collection: digger" is shown and the collection is synced
- **THEN** none of its entries shows the OWNED badge

