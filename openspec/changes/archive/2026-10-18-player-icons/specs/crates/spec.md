## MODIFIED Requirements

### Requirement: The Playlist crate
A crate named "Playlist" SHALL always exist. Files given on the command line SHALL replace its contents, and it SHALL then be shown and played. It SHALL be possible to clear it, but not to rename or delete it. Opening files from inside the app SHALL NOT replace any crate. Adding music (ADD, drag-and-drop, Cmd+O, M3U import) SHALL add to the shown crate.

#### Scenario: Opening files leaves other crates alone
- **WHEN** crate "Lowtide Tapes" is shown and the app is started with two files on the command line
- **THEN** the Playlist crate holds exactly those two files and plays the first one, and "Lowtide Tapes" is unchanged

#### Scenario: Playlist cannot be deleted
- **WHEN** the user opens the crate menu while the Playlist crate is shown
- **THEN** Rename crate… and Delete crate… are unavailable

#### Scenario: Cmd+O adds, never replaces
- **WHEN** the Playlist crate holds three files and is shown, and the user opens two files with Cmd+O
- **THEN** the Playlist crate holds five files
