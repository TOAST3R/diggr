# playlist Specification

## Purpose
TBD - created by archiving change classic-ui. Update Purpose after archive.
## Requirements
### Requirement: Playlist display
The playlist section SHALL list entries as "N. (catno) Artist: Title (T BPM)" with durations, highlight the current track, show total/selected duration, and scroll. "(catno) " SHALL appear only when the entry has a catalog number, "Artist: " only when it has an artist, and " (T BPM)" only when its tempo is known. An entry waiting for its audio SHALL be drawn dimmed, with an icon for its state where its duration would be: listed, queued, downloading (a bar showing the downloaded share), or needs yt-dlp. An unavailable entry SHALL be drawn dimmed, with an unavailable icon where its duration would be. Only an entry whose file could not be opened or decoded SHALL be drawn in the error colour. The state's wording (for example "downloading 40%" or "no clip") SHALL be shown in the entry's tooltip.

#### Scenario: Current highlighted
- **WHEN** track 4 is playing
- **THEN** entry 4 is drawn in the highlight color

#### Scenario: Waiting is not an error
- **WHEN** an entry's audio is 40% downloaded
- **THEN** the entry is drawn dimmed, with a bar 40% full where its duration would be, not in the error colour, and its tooltip says "downloading 40%"

#### Scenario: Unavailable stays visible
- **WHEN** an entry will never have audio because its record has no clip
- **THEN** it stays in the list, dimmed, with the unavailable icon where its duration would be, and its tooltip says "no clip"

#### Scenario: Old crate text
- **WHEN** a crate saved by an older version holds an entry waiting with the text "listed"
- **THEN** it loads and shows the listed icon

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

### Requirement: Entry tooltip
Hovering an entry SHALL show a tooltip with everything known about it:
- its full name, even when the row truncates it;
- its label, catalog number, side and year;
- its tempo and duration;
- its state and the reason;
- whether it is kept or passed, and whether a wantlist change is pending;
- its for-sale snapshot, with how long ago it was fetched.

Information that isn't known SHALL be left out. For a local file, the tooltip SHALL show its path. Building the tooltip SHALL NOT read files or make requests.

#### Scenario: Discogs entry
- **WHEN** the pointer rests on an entry from label Lowtide Tapes, catalog number LT-012, side A1, 1994, at 124 BPM, with 6 for sale from €9.00 fetched 3 hours ago
- **THEN** the tooltip shows all of these, including "fetched 3 h ago"

#### Scenario: Truncated name
- **WHEN** an entry's name is cut off in its row
- **THEN** the tooltip shows the whole name

#### Scenario: Local file
- **WHEN** the pointer rests on a local file entry
- **THEN** the tooltip shows its path, and no Discogs fields

### Requirement: Entry context menu
Right-clicking an entry (or Control-clicking it on macOS) SHALL open a menu with:
- Play, or Arm when the entry is waiting for its audio;
- Remove;
- Send to crate;
- Render show, when available;
- for an entry from Discogs: Keep or Undo keep, Pass or Undo pass, Open for-sale page, Open release on Discogs, and Copy Discogs link.

When the clicked entry is selected, Remove and Send to crate SHALL act on the whole selection. Otherwise the selection SHALL first become the clicked entry. All other items SHALL act on the clicked entry only.

#### Scenario: Remove a selection
- **WHEN** entries 3 to 6 are selected and the user right-clicks entry 4 and chooses Remove
- **THEN** entries 3 to 6 are removed

#### Scenario: Right-click outside the selection
- **WHEN** entries 3 to 6 are selected and the user right-clicks entry 10 and chooses Remove
- **THEN** only entry 10 is removed

#### Scenario: Open the release
- **WHEN** the user chooses Open release on Discogs on an entry from release 123456
- **THEN** the default browser opens https://www.discogs.com/release/123456

#### Scenario: Arm from the menu
- **WHEN** the user chooses Arm on an entry whose preview is downloading
- **THEN** the entry is armed exactly as by a double-click

### Requirement: Maximized playlist
A toggle SHALL maximize the playlist: the ⇔ button in the playlist's title bar, or Shift+P. While maximized:
- the window SHALL fill the screen's usable area (below the menu bar), as the operating system maximizes it;
- the player SHALL be a thin strip on the left, at most 27 skin pixels wide, showing only the play state, the elapsed time, previous, play or pause, next, and the ⇔ button;
- the playlist SHALL take the rest of the window, its width and rows following the window, with columns when it is wide enough;
- the EQ SHALL be hidden, and the keyboard SHALL go to the playlist.

Toggling again SHALL restore the previous window frame, layout, playlist width and rows exactly. When the playlist is hidden, the toggle SHALL show it first. The mode SHALL be remembered across launches, and a launch in this mode SHALL stay within the 300 ms launch target. Entering or leaving the mode SHALL NOT affect playback.

#### Scenario: Maximize
- **WHEN** the window is 1100 × 580 points at 2× on a 1440 × 900 display, and the user presses Shift+P
- **THEN** the window fills the area below the menu bar, the player is a thin strip on the left, and the playlist fills the rest, with columns

#### Scenario: Restore
- **WHEN** the user clicks ⇔ while maximized
- **THEN** the window returns to 1100 × 580 points at its previous position, with the player column and the playlist at their previous width and rows

#### Scenario: Strip controls
- **WHEN** the playlist is maximized and the user clicks next in the strip
- **THEN** the next track starts, exactly as with B

#### Scenario: Remembered
- **WHEN** the app is quit while maximized and launched again
- **THEN** its first frame is already maximized, within 300 ms of launch

### Requirement: Trackpad scrolling
Scrolling the playlist SHALL add up partial scroll steps, so that slow two-finger trackpad scrolling, and its momentum, move the list one row per row-height of scrolling. Scrolling SHALL work anywhere over the list, entries included. Leftover partial steps SHALL be dropped when the pointer leaves the list or scrolling pushes past its top or bottom.

#### Scenario: Slow scroll
- **WHEN** the user scrolls the playlist down by 3 points per frame for 10 frames with 13-point rows
- **THEN** the list moves down by 2 rows

#### Scenario: No jump later
- **WHEN** 10 points of scrolling are left over and the pointer leaves the list and comes back
- **THEN** the next 3 points of scrolling don't move the list

