# playlist Specification

## Purpose
TBD - created by archiving change classic-ui. Update Purpose after archive.
## Requirements
### Requirement: Playlist display
The playlist section SHALL list entries as "N. Artist - Title" with durations, highlight the current track, show total/selected duration, and scroll.

#### Scenario: Current highlighted
- **WHEN** track 4 is playing
- **THEN** entry 4 is drawn in the highlight color

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
Double-clicking an entry SHALL play it; shuffle and repeat (off/all/one) SHALL determine the next track, and the Engine's pre-warm SHALL target the actual next track.

#### Scenario: Shuffle pre-warm
- **WHEN** shuffle is on and the current track nears its end
- **THEN** the track chosen as next by shuffle is the one pre-warmed and played gaplessly

### Requirement: Reordering and selection
Entries SHALL support single, range (Shift), and toggle (Cmd/Ctrl) selection and drag-to-reorder.

#### Scenario: Reorder
- **WHEN** entry 7 is dragged above entry 2
- **THEN** it becomes entry 2 and numbering updates

### Requirement: Persistence and M3U
The playlist SHALL persist across restarts and SHALL import/export M3U/M3U8.

#### Scenario: Export and reimport
- **WHEN** a playlist is exported as M3U8 and imported into an empty playlist
- **THEN** the same entries appear in the same order

