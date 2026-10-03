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

