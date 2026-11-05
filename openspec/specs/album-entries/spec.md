# album-entries Specification

## Purpose
Every entry knows the album it belongs to (a Discogs release, or a local file's album tag), so a whole record can be seen, selected or removed in one move, and its cover shown on hover without slowing digging.

## Requirements

### Requirement: Album on entries
Each entry SHALL carry the name of the album it belongs to, when known:
- an entry from a Discogs release SHALL carry the release's title;
- an entry from a master release SHALL carry the master's title;
- a "listed" entry SHALL carry the listing's title for its record;
- a local file SHALL carry its album tag.

The album SHALL be saved with the crate and SHALL survive reordering, sending to another crate and restarting. File tags SHALL NOT overwrite the album of an entry that has an origin. For a Discogs entry saved without an album, showing its crate SHALL fill the album from the cached release or master data, and SHALL NOT make any request to do so.

#### Scenario: From a release
- **WHEN** release 123456, titled "Glasshouse EP", is expanded into three entries
- **THEN** all three carry the album "Glasshouse EP"

#### Scenario: Local file
- **WHEN** a file whose album tag is "Music Has the Right to Children" is added
- **THEN** its entry carries that album once its tags are read

#### Scenario: Older crate
- **WHEN** a crate saved before albums existed is shown, and its releases' data is in the disk cache
- **THEN** its entries show their albums, and no request is sent to Discogs

### Requirement: Album membership
Two entries of the shown crate SHALL belong to the same album when:
- both come from Discogs and have the same release, or have no release and the same master release; or
- both are local files with the same artist and album, compared without regard to case.

An entry with no release, no master release and no album tag SHALL belong to no album. Waiting and unavailable entries SHALL belong to their record's album.

#### Scenario: Same release
- **WHEN** a crate holds three entries from release 123456 and one from release 777
- **THEN** the three form one album and the fourth is not in it

#### Scenario: Same name, different artist
- **WHEN** a crate holds local files from "Greatest Hits" by Artist A and "Greatest Hits" by Artist B
- **THEN** they form two albums

### Requirement: Album on right-click
While an entry's context menu is open, every other entry of the same album in the shown crate SHALL be drawn with a tint, without changing the selection. The tint SHALL clear when the menu closes. When the entry belongs to an album, the menu SHALL offer:
- Remove album (N tracks), where N counts every entry of the album in the shown crate, shown or scrolled away. It removes all of them, and a playing entry among them SHALL keep playing to its end, as with Remove;
- Select album, which makes the selection exactly the album's entries and puts the cursor on the clicked entry.

Both items SHALL be disabled when the album has only the clicked entry, and absent when the entry belongs to no album.

#### Scenario: Tint
- **WHEN** entries 12, 13, 14 and 40 come from release 123456, entries 3 to 6 are selected, and the user right-clicks entry 13
- **THEN** entries 12, 14 and 40 are tinted while the menu is open, and the selection is only entry 13, as without albums

#### Scenario: Remove album
- **WHEN** the user right-clicks entry 13 and chooses Remove album (4 tracks)
- **THEN** entries 12, 13, 14 and 40 are removed, and the rest keep their order

#### Scenario: Playing entry in the album
- **WHEN** entry 12 plays and the user removes its album
- **THEN** entry 12 plays to its end without interruption, and the next track is the next remaining entry

#### Scenario: Select album
- **WHEN** the user right-clicks entry 13 and chooses Select album
- **THEN** entries 12, 13, 14 and 40 are selected, and the cursor is on entry 13

#### Scenario: Single
- **WHEN** the user right-clicks the only entry of its release
- **THEN** Remove album and Select album are shown disabled

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

### Requirement: Record artist on entries
Each entry from Discogs SHALL carry its record's credited artist, as Discogs shows it for the release (or the master, or the listing): "Various" for a compilation, joined names for a shared credit. Every entry of a record SHALL carry the same record artist, whatever its own track credit. The record artist SHALL be saved with the crate and SHALL survive reordering, sending to another crate and restarting. For a Discogs entry saved without a record artist, showing its crate SHALL fill it from the cached release or master data, and SHALL NOT make any request to do so. The entry's own artist (the track's credit) SHALL NOT change.

#### Scenario: Compilation
- **WHEN** a release credited to "Various", with tracks by Nightcraft and Lumen, is expanded
- **THEN** every entry carries the record artist "Various", and the entries' own artists stay Nightcraft and Lumen

#### Scenario: Older crate
- **WHEN** a crate saved before record artists existed is shown, and its releases' data is in the disk cache
- **THEN** its entries carry their record artists, and no request is sent to Discogs
