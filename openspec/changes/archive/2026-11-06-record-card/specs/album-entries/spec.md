# Spec Delta

## MODIFIED Requirements

### Requirement: Cover on hover
A record's cover SHALL be shown on its record row and in its record card (see `record-view` and `record-card`), when the record has an image. Entry tooltips SHALL NOT show a cover. The thumbnail (at most 150 pixels) SHALL be fetched by a background worker, only for the record whose card is open or for a record row in view in a grouped crate, and SHALL be kept on disk in the cache folder so it is fetched only once. Record rows in view SHALL be fetched from the top down, at most 40 waiting at a time, and the open card's record SHALL go first. A cover already on disk SHALL be loaded without waiting for the pace. While the cover isn't loaded, the row and the card SHALL keep the cover's space empty, and SHALL show the cover as soon as it arrives. Covers SHALL NOT be fetched from the Discogs API host, SHALL NOT use the API's request budget, and SHALL be fetched one at a time, at most 4 per second. A cover that couldn't be fetched SHALL NOT be tried again before the next launch. Fetching covers SHALL NOT affect playback.

#### Scenario: First hover
- **WHEN** the user opens the card of a record whose cover isn't cached
- **THEN** the card appears at once with an empty cover space, and the cover appears in it when it arrives

#### Scenario: Cached
- **WHEN** the user opens the card of a record whose cover was fetched in an earlier session
- **THEN** the cover is shown with no request to the network

#### Scenario: Sweeping the list
- **WHEN** in a flat crate, the pointer moves down 40 rows and stops on row 41
- **THEN** no cover is fetched and the tooltip shows none

#### Scenario: No image
- **WHEN** the record has no image on Discogs, or it is a local album
- **THEN** its row and its card show the record icon

#### Scenario: Playback unaffected
- **WHEN** covers are fetched while a track plays
- **THEN** playback has zero underruns

#### Scenario: Record rows in view
- **WHEN** a grouped crate is scrolled to show 12 record rows whose covers aren't cached
- **THEN** those 12 covers are fetched from the top row down, at most 4 per second, and records scrolled past without stopping are not fetched once out of view

#### Scenario: Hover goes first
- **WHEN** covers of rows in view are being fetched and the user opens the card of a record further down
- **THEN** that record's cover is fetched next
