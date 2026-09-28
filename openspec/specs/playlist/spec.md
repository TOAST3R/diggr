# playlist Specification

## Purpose
TBD - created by archiving change classic-ui. Update Purpose after archive.
## Requirements
### Requirement: Playlist display
The playlist section SHALL list entries as "N. (catno) Artist: Title (T BPM)" with durations, highlight the current track, show total/selected duration, and scroll. "(catno) " SHALL appear only when the entry has a catalog number, "Artist: " only when it has an artist, and " (T BPM)" only when its tempo is known. An entry waiting for its audio SHALL be drawn dimmed, with a short status (for example "listed" or "downloading 40%") where its duration would be. An unavailable entry SHALL be drawn dimmed, with its reason (for example "no clip") where its duration would be. Only an entry whose file could not be opened or decoded SHALL be drawn in the error colour.

#### Scenario: Current highlighted
- **WHEN** track 4 is playing
- **THEN** entry 4 is drawn in the highlight color

#### Scenario: Discogs entry name
- **WHEN** entry 3 is "Nightcraft" / "Glasshouse" from catalog number LT-012, with a known tempo of 124 BPM
- **THEN** it reads "3. (LT-012) Nightcraft: Glasshouse (124 BPM)"

#### Scenario: Local file without tempo
- **WHEN** entry 5 is a local file tagged "Mira Sol" / "Coastline" that has never been analysed
- **THEN** it reads "5. Mira Sol: Coastline", with no catalog number and no BPM

#### Scenario: Waiting is not an error
- **WHEN** an entry's audio is 40% downloaded
- **THEN** the entry is drawn dimmed, with "downloading 40%" where its duration would be, and not in the error colour

#### Scenario: Unavailable stays visible
- **WHEN** an entry will never have audio because its record has no clip
- **THEN** it stays in the list, dimmed, with "no clip" where its duration would be

### Requirement: Adding and removing tracks
The system SHALL add tracks via file dialog, folder dialog (recursive, supported formats only), and drag-and-drop, and SHALL remove selected entries or clear the list.

#### Scenario: Drop a folder
- **WHEN** a folder with 200 audio files and some images is dropped
- **THEN** the 200 audio files are appended and the images are ignored

### Requirement: Lazy metadata
Entries SHALL appear immediately with file-name titles; tags and durations SHALL be filled by a low-priority background worker without affecting playback.

#### Scenario: Large add during playback
- **WHEN** 2,000 files are added while music plays
- **THEN** playback has no underruns and entries update progressively

### Requirement: Playback order
Double-clicking an entry SHALL play it; shuffle and repeat (off/all/one) SHALL determine the next track, and the Engine's pre-warm SHALL target the actual next track. Entries that are waiting for their audio or unavailable SHALL be skipped by next, previous, shuffle and repeat, and SHALL never be handed to the engine; an entry SHALL join the play order as soon as its audio arrives. Double-clicking an entry that is waiting for its audio SHALL arm it: the current track keeps playing, the main window says it is waiting for that entry, and the entry starts within 100 ms of its audio arriving. Starting another track SHALL cancel the arming.

#### Scenario: Shuffle pre-warm
- **WHEN** shuffle is on and the current track nears its end
- **THEN** the track chosen as next by shuffle is the one pre-warmed and played gaplessly

#### Scenario: Skip what is not ready
- **WHEN** track 3 ends while track 4 is still downloading and track 5 is playable
- **THEN** track 5 plays gaplessly after track 3, and track 4 is not marked as failed

#### Scenario: Arm a waiting entry
- **WHEN** the user double-clicks an entry whose audio is still downloading
- **THEN** the current track keeps playing, the main window says it is waiting for that entry, and the entry starts within 100 ms of its audio arriving

### Requirement: Reordering and selection
Entries SHALL support single, range (Shift), and toggle (Cmd/Ctrl) selection and drag-to-reorder.

#### Scenario: Reorder
- **WHEN** entry 7 is dragged above entry 2
- **THEN** it becomes entry 2 and numbering updates

### Requirement: Persistence and M3U
Each crate SHALL persist across restarts, and the shown crate SHALL import and export M3U/M3U8. An entry whose audio comes from a remote source SHALL be exported as that source's URL, whether or not its audio has arrived; an unavailable entry SHALL be left out of the export.

#### Scenario: Export and reimport
- **WHEN** a crate of local files is exported as M3U8 and imported into an empty crate
- **THEN** the same entries appear in the same order

#### Scenario: Export a remote entry
- **WHEN** a crate holding a local file and then an entry whose audio comes from https://www.youtube.com/watch?v=abcdefghijk is exported
- **THEN** the M3U8 lists the file's path and then that URL, and no downloaded audio file

### Requirement: Entry origin
An entry that has an origin SHALL keep it: the record the entry belongs to (source page, release, label, catalog number, year, side and clip). The origin SHALL survive reordering, sending to another crate, saving and restarting. An entry with an origin SHALL keep its own artist and title, in the playlist and in the main window's title line, even when its file's tags differ; it SHALL take only its duration from the file.

#### Scenario: Origin survives a copy and a restart
- **WHEN** an entry from release 123456 (catalog number LT-012, side A1) is sent to another crate and the app is restarted
- **THEN** the copy still has release 123456, LT-012 and A1

#### Scenario: Tags don't overwrite the record
- **WHEN** the audio of an entry shown as "Nightcraft - Glasshouse" arrives in a file whose tags say "Unknown - glasshouse (vinyl rip)"
- **THEN** the playlist and the title line still show "Nightcraft - Glasshouse", and the entry takes only its duration from the file

### Requirement: Entry tempo
Each entry SHALL carry its tempo once any analysis of its audio is known: from preview preparation, from the playing track's analysis, or from the score cache. The tempo SHALL be the tempo of the score's longest steady-tempo stretch, doubled or halved until it lies within 88–176 BPM, and rounded to a whole number. The tempo SHALL be saved with the crate. The app SHALL NOT download or analyse audio only to learn a tempo, and SHALL NOT take a tempo from file tags.

#### Scenario: Prepared preview
- **WHEN** the next entry's preview is prepared and its analysed tempo is 124.3 BPM
- **THEN** the entry shows "(124 BPM)" within one UI frame of preparation finishing

#### Scenario: Half time folded
- **WHEN** a track is analysed at 87 BPM
- **THEN** its entry shows "(174 BPM)"

#### Scenario: Double time folded
- **WHEN** a track is analysed at 280 BPM
- **THEN** its entry shows "(140 BPM)"

#### Scenario: Survives a restart
- **WHEN** an entry showing "(128 BPM)" is in a crate and the app is restarted
- **THEN** the entry shows "(128 BPM)" in the first frame, without analysis running

#### Scenario: Nothing fetched for a BPM
- **WHEN** a 312-release label crate is shown while entry 1 plays
- **THEN** only previews within the download-ahead horizon are downloaded, and entries outside it show no BPM

### Requirement: Playlist placement and size
The playlist SHALL be drawn to the right of the player column (main, waveform and EQ sections) in the same window. Its corner handle SHALL resize it freely in width, from the skin's playlist width (275 points at 1×) upwards, and in whole rows in height, never shorter than the player column. Its width and rows SHALL be remembered across launches. Hiding the playlist SHALL shrink the window to the player column. A restored width SHALL be reduced if the window wouldn't fit on its monitor. Resizing SHALL NOT affect playback.

#### Scenario: Widen
- **WHEN** the user drags the playlist's corner handle 300 points to the right
- **THEN** the playlist is 300 points wider, long names show in full where they fit, and the player column keeps its size

#### Scenario: Remembered size
- **WHEN** the playlist is resized to 700 × 30 rows and the app is restarted
- **THEN** it opens at 700 points wide with 30 rows

#### Scenario: Hidden playlist
- **WHEN** the user hides the playlist
- **THEN** the window is exactly as wide as the player column

### Requirement: Keyboard navigation
While the playlist has focus it SHALL show a keyboard cursor, drawn as an outline distinct from the selection highlight. The keys SHALL work as follows:
- ↑ and ↓ SHALL move the cursor by one entry and select only that entry;
- Shift+↑ and Shift+↓ SHALL extend the selection;
- PgUp and PgDn SHALL move by one visible page;
- Home and End SHALL move to the first and last entries;
- Enter SHALL play the cursor's entry (or arm it when it is waiting for audio).

The list SHALL scroll so the cursor stays visible. When there is no cursor yet, the first ↑ or ↓ SHALL put it on the playing entry, or on the first entry when nothing in the shown crate plays. The cursor SHALL follow its entry when entries are added, replaced or reordered around it.

#### Scenario: Walk down
- **WHEN** the playlist has focus, the cursor is on entry 9 of 40 with 10 visible rows ending at entry 10, and the user presses ↓ twice
- **THEN** the cursor and the selection are on entry 11, and the list has scrolled by one row so entry 11 is visible

#### Scenario: First press lands on the playing entry
- **WHEN** entry 23 plays, the playlist has just been focused with no cursor, and the user presses ↓
- **THEN** the cursor is on entry 23 and the list shows it

#### Scenario: Extend
- **WHEN** the cursor is on entry 5 and the user presses Shift+↓ three times
- **THEN** entries 5 to 8 are selected and the cursor is on entry 8

#### Scenario: Stable under dig expansion
- **WHEN** the cursor is on entry 30 and a "listed" entry above it is replaced by three clip entries
- **THEN** the cursor stays on the same entry, now numbered 32

### Requirement: Show the playing entry
Pressing P SHALL show the playing crate, scroll it so the playing entry is visible, and put the cursor on it. When the playing entry changes and the previous playing entry was visible, the list SHALL scroll by the least amount that shows the new one; when it wasn't visible, the list SHALL NOT scroll.

#### Scenario: P
- **WHEN** entry 180 plays while the list shows entries 1 to 20
- **THEN** pressing P scrolls to show entry 180 and puts the cursor on it

#### Scenario: Follow
- **WHEN** entry 20 is visible in the last row and playing, and the next track starts
- **THEN** the list scrolls by one row so entry 21 is visible

#### Scenario: Don't steal the scroll
- **WHEN** the user has scrolled to entries 200 to 220 while entry 5 plays, and the next track starts
- **THEN** the list stays at entries 200 to 220

