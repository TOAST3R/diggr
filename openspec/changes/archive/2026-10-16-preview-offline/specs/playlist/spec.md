## MODIFIED Requirements

### Requirement: Playlist display
The playlist section SHALL list entries as "N. (catno) Artist: Title · Album (T BPM)" with durations, highlight the current track, show total/selected duration, and scroll. "(catno) " SHALL appear only when the entry has a catalog number, and a catalog number of "none" (any case, as Discogs writes a release without one) SHALL count as none. "Artist: " SHALL appear only when it has an artist, " · Album" only when it has an album that differs from its title (ignoring case and surrounding spaces), and " (T BPM)" only when its tempo is known. When the row is too narrow, the album SHALL be cut before the title. An entry waiting for its audio SHALL be drawn dimmed, with an icon for its state where its duration would be: listed, queued, downloading (a bar showing the downloaded share), or needs yt-dlp. An unavailable entry SHALL be drawn dimmed, with an unavailable icon where its duration would be. Only an entry whose file could not be opened or decoded SHALL be drawn in the error colour. The state's wording (for example "downloading 40%" or "no clip") SHALL be shown in the entry's tooltip.

#### Scenario: Current highlighted
- **WHEN** track 4 is playing
- **THEN** entry 4 is drawn in the highlight color

#### Scenario: Album in the row
- **WHEN** an entry from "Glasshouse EP" is titled "Glasshouse", with catalog number LT-012, at 124 BPM
- **THEN** it is listed as "12. (LT-012) Nightcraft: Glasshouse · Glasshouse EP (124 BPM)"

#### Scenario: No catalog number from Discogs
- **WHEN** an entry by Ben Gibson titled "Vanished Between" comes from a release whose catalog number is "none"
- **THEN** it is listed as "Ben Gibson: Vanished Between", with no "(none) "

#### Scenario: Single
- **WHEN** an entry titled "Static" comes from a release titled "Static"
- **THEN** its row shows no album

#### Scenario: Narrow row
- **WHEN** a row is too narrow for its whole name
- **THEN** the album is cut before any of the title is

#### Scenario: Waiting is not an error
- **WHEN** an entry's audio is 40% downloaded
- **THEN** the entry is drawn dimmed, with a bar 40% full where its duration would be, not in the error colour, and its tooltip says "downloading 40%"

#### Scenario: Unavailable stays visible
- **WHEN** an entry will never have audio because its record has no clip
- **THEN** it stays in the list, dimmed, with the unavailable icon where its duration would be, and its tooltip says "no clip"

#### Scenario: Old crate text
- **WHEN** a crate saved by an older version holds an entry waiting with the text "listed"
- **THEN** it loads and shows the listed icon

### Requirement: Playback order
Double-clicking an entry SHALL play it; shuffle and repeat (off/all/one) SHALL determine the next track, and the Engine's pre-warm SHALL target the actual next track. Entries that are waiting for their audio or unavailable SHALL be skipped by next, previous, shuffle and repeat, and SHALL never be handed to the engine; an entry SHALL join the play order as soon as its audio arrives. Double-clicking an entry that is waiting for its audio SHALL arm it: the current track keeps playing, the main window says it is waiting for that entry, and the entry starts within 100 ms of its audio arriving. An armed entry SHALL come first among the previews to download or search, whichever crate it is in. Double-clicking an entry that is unavailable with "clip failed" or "not found", in any crate, SHALL retry it: it goes back to waiting (to be downloaded again, or searched again with its remembered "not found" forgotten), its retry starts before any other waiting preview, and it is armed. Double-clicking any other unavailable entry ("no clip") SHALL only say why it can't play. Starting another track SHALL cancel the arming.

#### Scenario: Shuffle pre-warm
- **WHEN** shuffle is on and the current track nears its end
- **THEN** the track chosen as next by shuffle is the one pre-warmed and played gaplessly

#### Scenario: Skip what is not ready
- **WHEN** track 3 ends while track 4 is still downloading and track 5 is playable
- **THEN** track 5 plays gaplessly after track 3, and track 4 is not marked as failed

#### Scenario: Arm a waiting entry
- **WHEN** the user double-clicks an entry whose audio is still downloading
- **THEN** the current track keeps playing, the main window says it is waiting for that entry, and the entry starts within 100 ms of its audio arriving

#### Scenario: Arm a track to search in another crate
- **WHEN** crate A is playing and the user double-clicks, in crate B, a track waiting to be searched
- **THEN** its search runs before the other waiting searches, and it starts once its preview arrives

#### Scenario: Retry a failed clip
- **WHEN** the user double-clicks an entry marked "clip failed" in a crate that isn't a label crate
- **THEN** it goes back to waiting, its clip downloads again before other waiting previews, and it starts playing once it arrives

#### Scenario: Retry a track not found
- **WHEN** the user double-clicks an entry marked "not found"
- **THEN** it is searched again, and plays if a usable video is found

#### Scenario: Nothing to retry
- **WHEN** the user double-clicks an entry marked "no clip"
- **THEN** the main window says it has no clip, and nothing is downloaded
