## MODIFIED Requirements

### Requirement: Local copy follows the app's writes
When the app adds a release to the collection, the cached collection SHALL hold it at once, with its master release, catalog number and year taken from the cached release data, without a request. The cached count of copies SHALL include it, so that the next incremental sync stays incremental. When the app removes a copy from the collection, the cached collection SHALL drop that copy's instance and lower the count of copies by one at once, and SHALL drop the release when no copy of it is left, without a request. The cache SHALL be saved atomically.

#### Scenario: After an add
- **WHEN** the app has added release 123456 to the collection, and the app is restarted before any sync
- **THEN** release 123456 is still marked OWNED

#### Scenario: Also added on discogs.com
- **WHEN** the user added one record on discogs.com and another from the app, before the next sync
- **THEN** the next sync notices the missing record and reads the whole collection again, and both are marked OWNED

#### Scenario: After a removal
- **WHEN** the app has removed the only copy of release 123456, and a sync runs with nothing else changed on Discogs
- **THEN** release 123456 is not marked OWNED, and the sync takes 1 request and doesn't read the whole collection again

#### Scenario: Other pressing still owned
- **WHEN** the app removes release 111 (master 900) and the collection still holds release 222 of master 900
- **THEN** entries of release 111 stay marked OWNED, naming release 222's pressing
