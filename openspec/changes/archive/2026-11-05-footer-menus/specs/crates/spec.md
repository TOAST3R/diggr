## MODIFIED Requirements

### Requirement: The Playlist crate
A crate named "Playlist" SHALL always exist. Files given on the command line SHALL replace its contents, and it SHALL then be shown and played. It SHALL be possible to clear it, but not to rename or delete it. Opening files from inside the app SHALL NOT replace any crate. Adding music (the list menu's Add files… and Add folder…, the empty crate's Add files from your computer, drag-and-drop, Cmd+O, M3U import) SHALL add to the shown crate.

#### Scenario: Opening files leaves other crates alone
- **WHEN** crate "Lowtide Tapes" is shown and the app is started with two files on the command line
- **THEN** the Playlist crate holds exactly those two files and plays the first one, and "Lowtide Tapes" is unchanged

#### Scenario: Playlist cannot be deleted
- **WHEN** the user opens the crate menu while the Playlist crate is shown
- **THEN** Rename crate… and Delete crate… are unavailable

#### Scenario: Cmd+O adds, never replaces
- **WHEN** the Playlist crate holds three files and is shown, and the user opens two files with Cmd+O
- **THEN** the Playlist crate holds five files

## ADDED Requirements

### Requirement: Export a crate from its menu
Right-clicking any crate, in the sidebar or the title-bar crate menu, SHALL offer Export M3U…, for every kind of crate. It SHALL write that crate as M3U8 (see the playlist's "Persistence and M3U"), whether or not it is the shown crate, and SHALL NOT show it or change playback. The save dialog SHALL propose "‹crate name›.m3u8". A crate that can't be read SHALL offer it disabled.

#### Scenario: Export a crate that isn't shown
- **WHEN** "Lowtide Tapes" (8 entries) is shown and the user right-clicks "Friday" (5 entries) in the sidebar and chooses Export M3U…
- **THEN** the save dialog proposes "Friday.m3u8", the file lists Friday's 5 entries, and "Lowtide Tapes" stays shown

#### Scenario: The Playlist crate
- **WHEN** the user right-clicks the Playlist crate in the sidebar
- **THEN** the menu offers Clear crate and Export M3U…, and still says why it can't be renamed or deleted

#### Scenario: Playback goes on
- **WHEN** a track from "Friday" is playing and the user exports "Lowtide Tapes" from its menu
- **THEN** playback continues without a gap or underrun
