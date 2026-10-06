## MODIFIED Requirements

### Requirement: Filters
Every send SHALL apply the skip passed filter, on by default: clips the user has passed on are left out. Its default can be changed in Options ▸ Discogs…, and each send can override it. Pasting SHALL use the default. A send SHALL keep records in every format: vinyl, digital files, CD, cassette and others.

#### Scenario: Every format
- **WHEN** a label with 10 vinyl releases and 5 CD-only releases is sent
- **THEN** all 15 releases add entries

#### Scenario: Skip passed
- **WHEN** a label is sent again after the user passed on 20 of its clips
- **THEN** those 20 clips are not added

## ADDED Requirements

### Requirement: Formats on entries
Each entry from Discogs SHALL carry its record's formats, grouped as Vinyl, File, CD, Cassette or Other:
- from the release's details when they are known;
- from the listing until then.

A format written with a count ("2x12\"", "3xLP", "17xFile") SHALL count as that format. Descriptions such as Album, Comp or Ltd, and containers such as Box Set or All Media, SHALL NOT count as formats. A record SHALL be vinyl when one of its formats is Vinyl. The formats SHALL be saved with the crate. For an entry saved without them, showing its crate SHALL fill them from the cached data, and SHALL NOT make any request to do so. An entry whose record has formats but no Vinyl SHALL show a dim mark naming its first format (FILE, CD, CASS or OTHER) where the OWNED badge goes.

#### Scenario: Triple LP
- **WHEN** a label listing gives AF060LP's format as "3x12\", Comp, Ltd, Tur"
- **THEN** AF060LP is vinyl, and its entries show no format mark

#### Scenario: Digital only
- **WHEN** a release whose only format is "5xFile, FLAC, EP" is expanded
- **THEN** its entries carry the format File and show the mark FILE

#### Scenario: Older crate
- **WHEN** a crate saved before formats existed is shown, and its releases' data is in the disk cache
- **THEN** its entries carry their formats, and no request is sent to Discogs

### Requirement: Vinyl first
In a crate filled from a page (any crate but the user's wantlist and collection crates), two releases SHALL be the same record when they have the same master release, or, when either has none, the same catalog number and title, ignoring case. When a record has a vinyl release in the crate:
- its tunes SHALL belong to the vinyl release: a non-vinyl release of the same record SHALL add no entry, and its listed placeholder SHALL leave the crate;
- when the non-vinyl release was expanded first, each of its entries whose clip the vinyl release also has SHALL become an entry of the vinyl release (its origin, catalog number, side, cover and for-sale data), keeping its place, its audio and, when it plays, its playback; its other entries SHALL leave the crate, except the entry that is playing, which SHALL leave once it stops;
- the expansion SHALL fetch a vinyl release before a non-vinyl release with the same catalog number and title, when both are waiting.

A record with no vinyl release SHALL stay in the crate with all its entries. Resolving twins SHALL NOT make any request beyond expanding the releases, and SHALL NOT interrupt playback. The wantlist and collection crates SHALL keep every release they hold.

#### Scenario: Vinyl and digital of one record
- **WHEN** a label lists AF060LP as "17xFile, FLAC" (release 33337220) and as "3x12\"" (release 33988281), both of master 3847755
- **THEN** the crate holds AF060LP's tracks once, as entries of release 33988281, and nothing of release 33337220

#### Scenario: Digital fetched first
- **WHEN** the focus makes release 33337220 (File) expand before 33988281 (Vinyl), and one of its tracks is playing
- **THEN** once 33988281 is expanded, the playing entry and the others it shares become entries of 33988281, the track plays on without a gap, and no entry of 33337220 is left once it stops

#### Scenario: Digital only
- **WHEN** a label lists a release only as "4xFile, MP3"
- **THEN** its tracks are in the crate, marked FILE

#### Scenario: Wantlist untouched
- **WHEN** the wantlist crate holds a FLAC release and the vinyl release of the same master
- **THEN** both keep their entries
