## MODIFIED Requirements

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

## ADDED Requirements

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
