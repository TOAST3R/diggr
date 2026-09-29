## ADDED Requirements

### Requirement: Owned check
A paired extension SHALL be able to ask whether the record of a Discogs release, master release or marketplace item page is in the user's collection, sending only the page's address. The answer SHALL be one of:
- owned (this pressing);
- owned in another pressing (naming its catalog number and year);
- not owned;
- checking;
- no token (so the extension can say what's missing);
- unknown (no collection yet).

It SHALL be given from the cached collection without waiting for the UI or for Discogs, and SHALL cause no request to Discogs for a release or master release page. For a marketplace item whose release isn't known yet, the answer SHALL be "checking", and the item's release SHALL be looked up once, then remembered. An unpaired request SHALL be refused.

#### Scenario: Owned release
- **WHEN** a paired extension asks about https://www.discogs.com/release/123456 and release 123456 is in the collection
- **THEN** the answer is "owned", within 50 ms, and no request is sent to Discogs

#### Scenario: New marketplace item
- **WHEN** a paired extension asks about a marketplace item seen for the first time
- **THEN** the answer is "checking", one lookup is made, and asking again after it completes gives the owned answer without another request

#### Scenario: No token
- **WHEN** no Discogs token is set
- **THEN** the answer is "no token"
