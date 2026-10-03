## MODIFIED Requirements

### Requirement: Cover on hover
The tooltip of a Discogs entry SHALL show its record's cover at 96 points, beside the details, when the record has an image on Discogs. The cover SHALL be fetched by a background worker, only for an entry the pointer has rested on for 250 ms or for a record row in view in a grouped crate (see `record-view`), and SHALL be kept on disk in the cache folder so it is fetched only once. Record rows in view SHALL be fetched from the top down, at most 40 waiting at a time, and a rested hover SHALL go first. A cover already on disk SHALL be loaded without waiting for the pace. While the cover isn't loaded, the tooltip SHALL keep the cover's space empty, and SHALL show the cover as soon as it arrives. Covers SHALL NOT be fetched from the Discogs API host, SHALL NOT use the API's request budget, and SHALL be fetched one at a time, at most 4 per second. A cover that couldn't be fetched SHALL NOT be tried again before the next launch. Fetching covers SHALL NOT affect playback.

#### Scenario: First hover
- **WHEN** the pointer rests on an entry whose cover isn't cached
- **THEN** the tooltip appears at once with an empty cover space, and the cover appears in it when it arrives

#### Scenario: Cached
- **WHEN** the pointer rests on an entry whose cover was fetched in an earlier session
- **THEN** the cover is shown with no request to the network

#### Scenario: Sweeping the list
- **WHEN** in a flat crate, the pointer moves down 40 rows without resting, and stops on row 41
- **THEN** only row 41's cover is fetched

#### Scenario: No image
- **WHEN** the record has no image on Discogs, or the entry is a local file
- **THEN** the tooltip shows no cover space

#### Scenario: Playback unaffected
- **WHEN** covers are fetched while a track plays
- **THEN** playback has zero underruns

#### Scenario: Record rows in view
- **WHEN** a grouped crate is scrolled to show 12 record rows whose covers aren't cached
- **THEN** those 12 covers are fetched from the top row down, at most 4 per second, and records scrolled past without stopping are not fetched once out of view

#### Scenario: Hover goes first
- **WHEN** covers of rows in view are being fetched and the pointer rests on another entry for 250 ms
- **THEN** that entry's cover is fetched next
