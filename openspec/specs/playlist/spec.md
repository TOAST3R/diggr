# playlist Specification

## Purpose
TBD - created by archiving change classic-ui. Update Purpose after archive.
## Requirements
### Requirement: Playlist display
The playlist section SHALL list entries as "N. Artist - Title" with durations, highlight the current track, show total/selected duration, and scroll. An entry waiting for its audio SHALL be drawn dimmed, with a short status (for example "listed" or "downloading 40%") where its duration would be. An unavailable entry SHALL be drawn dimmed, with its reason (for example "no clip") where its duration would be. Only an entry whose file could not be opened or decoded SHALL be drawn in the error colour.

#### Scenario: Current highlighted
- **WHEN** track 4 is playing
- **THEN** entry 4 is drawn in the highlight color

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

