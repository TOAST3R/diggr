## ADDED Requirements

### Requirement: Owned records leave the wantlist
After every successful collection sync, every record on the user's Discogs wantlist that the collection owns, in that pressing or another pressing of the same master release, SHALL be removed from the Discogs wantlist, with one request per record. It SHALL also no longer be wanted, and its entries SHALL leave the wantlist crate. The main window SHALL say how many records were removed ("Removed 2 records you now own from your wantlist"). A sync that changes nothing SHALL remove nothing and say nothing.

#### Scenario: Bought on discogs.com
- **WHEN** the user bought release 123456 on discogs.com while it was on their wantlist, and the next sync finds it in the collection
- **THEN** release 123456 is removed from the Discogs wantlist and the wantlist crate, and the main window says "Removed 1 record you now own from your wantlist"

#### Scenario: Another pressing bought
- **WHEN** release 111 (master 900) is wanted and a sync finds release 222 of master 900 in the collection
- **THEN** release 111 is removed from the Discogs wantlist

#### Scenario: Nothing new
- **WHEN** a sync finds no owned record on the wantlist
- **THEN** no wantlist request is sent and no message is shown

### Requirement: Local copy follows the app's writes
When the app adds a release to the collection, the cached collection SHALL hold it at once, with its master release, catalog number and year taken from the cached release data, without a request. The cached count of copies SHALL include it, so that the next incremental sync stays incremental. The cache SHALL be saved atomically.

#### Scenario: After an add
- **WHEN** the app has added release 123456 to the collection, and the app is restarted before any sync
- **THEN** release 123456 is still marked OWNED

#### Scenario: Also added on discogs.com
- **WHEN** the user added one record on discogs.com and another from the app, before the next sync
- **THEN** the next sync notices the missing record and reads the whole collection again, and both are marked OWNED
