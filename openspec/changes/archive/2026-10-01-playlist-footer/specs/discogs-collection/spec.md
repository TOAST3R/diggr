## MODIFIED Requirements

### Requirement: Collection sync
With a Discogs token set, the app SHALL keep the user's collection cached on disk: the owned releases, and for each master release its owned pressings. It SHALL sync it only:
- when a crate holding Discogs entries is shown and the cache is missing or older than 7 days, and never before the window is interactive;
- when the user asks from Options ▸ Discogs…

The first sync SHALL use at most one request per 100 records. Later syncs SHALL fetch only the records added since the last one, newest first, stopping at the first record already known, and SHALL fetch the whole collection again only when records were removed. No request SHALL be made to find a record's other pressings. Syncing SHALL wait behind any page expansion that is waiting. A sync that fails SHALL keep the previous cache. Options ▸ Discogs… SHALL show how many records the collection holds and how old it is, or why it isn't available. Without a token, nothing SHALL be fetched and nothing marked.

#### Scenario: First sync
- **WHEN** a token for a user with 1,234 records is set
- **THEN** the collection is fetched with 13 requests, and Options ▸ Discogs… shows "Collection: 1,234 records, updated just now"

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
- **THEN** no collection request is ever sent, no entry is marked owned, and the main window says once that a Discogs token (Options ▸ Discogs…) would mark the records already owned

#### Scenario: Recent cache
- **WHEN** the app is launched with a collection cache 2 days old
- **THEN** no collection request is sent, and the cached collection marks the entries

#### Scenario: Offline
- **WHEN** a refresh fails because Discogs is unreachable
- **THEN** the cached collection stays in use, and the dialog says the refresh failed
