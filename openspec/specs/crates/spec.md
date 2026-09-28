# crates Specification

## Purpose
Lets the player hold several named playlists ("crates"), such as one per label being dug, one for the records you keep and a scratch list, and switch between them without interrupting playback.

## Requirements
### Requirement: Named crates
The player SHALL keep any number of named crates, each an ordered list of entries. The playlist window SHALL show one crate at a time, and its title bar SHALL show that crate's name in the skin's font, shortened to fit. Crate names SHALL be 1 to 40 characters long and unique, ignoring case.

#### Scenario: Name in the title bar
- **WHEN** the shown crate is named "Lowtide Tapes"
- **THEN** the playlist title bar reads LOWTIDE TAPES

#### Scenario: Duplicate name
- **WHEN** the user tries to create a crate named "keepers" while a crate named "Keepers" exists
- **THEN** the name is refused with a message and no crate is created

### Requirement: Crate menu
Clicking the playlist title bar without dragging SHALL open a menu listing every crate by name, marking the shown crate and the playing crate, followed by New crate…, Rename crate… and Delete crate…. Choosing a crate SHALL show it. Dragging the title bar SHALL still move the window and SHALL NOT open the menu. Deleting a crate that has entries SHALL ask for confirmation first.

#### Scenario: Switch crate
- **WHEN** the user clicks the title bar and chooses "Keepers"
- **THEN** the playlist window shows the Keepers crate and the title bar reads KEEPERS

#### Scenario: Drag still moves the window
- **WHEN** the user drags the playlist title bar
- **THEN** the window moves and no menu opens

#### Scenario: New crate
- **WHEN** the user chooses New crate… and enters "Gig 12 Oct"
- **THEN** an empty crate named "Gig 12 Oct" is created and shown

#### Scenario: Delete asks first
- **WHEN** the user chooses Delete crate… for a crate with 40 entries
- **THEN** the crate is deleted only after the user confirms

### Requirement: Playback follows its crate
Switching the shown crate SHALL NOT interrupt playback. The playing track SHALL continue, and next, previous, shuffle, repeat and pre-warm SHALL follow the crate the playing track came from (the playing crate) until the user starts a track in another crate, which then becomes the playing crate. Changing a crate that is not playing SHALL NOT affect playback. Deleting the playing crate SHALL stop playback.

#### Scenario: Look elsewhere while listening
- **WHEN** track 3 of crate A is playing and the user switches to crate B
- **THEN** track 3 keeps playing without a gap, and when it ends track 4 of crate A starts

#### Scenario: Start a track in another crate
- **WHEN** the user double-clicks an entry in crate B while crate A is playing
- **THEN** that entry plays, and next and previous now follow crate B

### Requirement: The Playlist crate
A crate named "Playlist" SHALL always exist. Files opened with Eject or on the command line SHALL replace its contents, and it SHALL then be shown and played. It SHALL be possible to clear it, but not to rename or delete it. Opening files SHALL NOT replace any other crate. Adding music (ADD, drag-and-drop, Cmd+O, M3U import) SHALL add to the shown crate.

#### Scenario: Opening files leaves other crates alone
- **WHEN** crate "Lowtide Tapes" is shown and the app is started with two files on the command line
- **THEN** the Playlist crate holds exactly those two files and plays the first one, and "Lowtide Tapes" is unchanged

#### Scenario: Playlist cannot be deleted
- **WHEN** the user opens the crate menu while the Playlist crate is shown
- **THEN** Rename crate… and Delete crate… are unavailable

### Requirement: Send entries to a crate
The entry right-click menu SHALL offer Send to crate, listing the other crates and New crate…. It SHALL copy the selected entries, in their order and with their metadata and origin, to the end of the chosen crate, skipping entries that crate already holds (the same file, or the same origin clip). The source crate SHALL be unchanged.

#### Scenario: Copy without duplicates
- **WHEN** three selected entries are sent to a crate that already holds one of them
- **THEN** the other two are appended to that crate in their order, and the source crate is unchanged

### Requirement: Crates persist
Every crate SHALL be saved within 2 s of a change, and when the app quits, with its name, its entries (with their metadata and origin) and its current entry. Writes SHALL be atomic. On launch, the crate that was shown when the app closed SHALL be shown again with its current entry. A crate file that cannot be read SHALL NOT prevent launch: it SHALL be left in place and reported once, and the other crates SHALL load.

#### Scenario: Restart
- **WHEN** the app is closed while crate "Lowtide Tapes" is shown with entry 7 current, then started again
- **THEN** "Lowtide Tapes" is shown with the same entries, order and metadata, and entry 7 is current

#### Scenario: Damaged crate file
- **WHEN** one crate file contains invalid data
- **THEN** the app launches, the other crates are available, and a message names the crate that could not be read

### Requirement: Migration from the single playlist
On the first launch with crates, if the config folder holds the single playlist file and no crates yet, its entries, order, metadata and current entry SHALL become the Playlist crate. The single playlist file SHALL be left unchanged.

#### Scenario: Upgrade
- **WHEN** the app starts with a saved 300-entry playlist and no crates
- **THEN** the Playlist crate holds the same 300 entries in the same order, with the same current entry, and the old playlist file is still present and unmodified

### Requirement: Crates do not slow launch
Only the list of crates and the shown crate SHALL be loaded at launch. Other crates SHALL be loaded when they are first shown, played or sent to. The window SHALL be interactive within 300 ms of launch with 50 saved crates of 500 entries each.

#### Scenario: Many crates
- **WHEN** the app is launched with 50 saved crates of 500 entries each
- **THEN** the window is interactive within 300 ms

