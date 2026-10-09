## ADDED Requirements

### Requirement: Label crates
The app SHALL keep one crate per followed label, named "Label: ‹name›" after the label's Discogs name, and SHALL remember which label each one follows, by the label's Discogs id. A label crate SHALL stay its label's crate after a rename, SHALL survive restarts, and SHALL be grouped by record until the user toggles it. Following a label that already has a crate SHALL NOT make a second one.

#### Scenario: Survives a restart
- **WHEN** the app follows Siesta Records and Hardline Sounds, and is restarted
- **THEN** LABELS lists both crates with their entries

#### Scenario: Renamed
- **WHEN** the user renames "Label: Siesta Records" to "Siesta"
- **THEN** it stays under LABELS, and Refresh label still refreshes Siesta Records

#### Scenario: Grouped by default
- **WHEN** a label crate is shown for the first time
- **THEN** it is grouped by record

### Requirement: Filled only from its label
A label crate SHALL take tracks only from its label's page. Pasting into it, dropping files or entries on it, and Send to crate into it SHALL be refused, and so SHALL Delete, Remove, Remove album, Remove selected and Clear crate; copying entries out to another crate SHALL still work, as from the Discogs crates. Each refusal SHALL say in the main window that label crates fill from their label and that Pass (N) dims a track. Pass (N), Undo pass, Y, I and the rest of the entry menu SHALL work as in any crate; a passed track SHALL stay in the crate, dimmed.

#### Scenario: Delete refused
- **WHEN** the user selects 3 entries in "Label: Siesta Records" and presses Delete
- **THEN** nothing is removed, and the main window says label crates fill from their label and N passes a track

#### Scenario: Paste refused
- **WHEN** "Label: Siesta Records" is shown and the user pastes an artist's Discogs address
- **THEN** nothing is added, and the main window says label crates fill from their label

#### Scenario: Pass dims
- **WHEN** the user presses N on track 4 of "Label: Siesta Records" while it plays
- **THEN** track 4 stays in the crate, dimmed and remembered as passed, and track 5 starts

### Requirement: Refresh label
Refresh label SHALL send the label's page again into its crate with skip passed on, so only tracks the crate doesn't hold, and that weren't passed, come in. Refreshing SHALL happen only when the user asks. When it finishes, the main window SHALL say how many records came in ("Label: Siesta Records: 4 new records"), or "Label: Siesta Records: up to date". While it runs the item SHALL read "Refreshing…" and be disabled. A failed refresh SHALL change nothing and say why.

#### Scenario: New records
- **WHEN** Siesta Records released 4 records since its crate was filled, and the user chooses Refresh label
- **THEN** their tracks come in, every other entry stays where it was, and the main window says "Label: Siesta Records: 4 new records"

#### Scenario: Nothing new
- **WHEN** the label has released nothing since the last refresh, and some of its tracks were passed
- **THEN** no entry is added or doubled, the passed tracks stay dimmed, and the main window says "Label: Siesta Records: up to date"

#### Scenario: Offline
- **WHEN** Discogs is unreachable and the user chooses Refresh label
- **THEN** the crate is unchanged, and the main window says the refresh failed because Discogs is offline

### Requirement: Label crate menu
Right-clicking a label crate, in the sidebar or the title-bar crate menu, SHALL offer exactly, in this order: Delete label…, Export to crate ▸, Refresh label, and Download all tracks. It SHALL NOT offer Rename crate… or Delete crate….

#### Scenario: The menu
- **WHEN** the user right-clicks "Label: Siesta Records" in the sidebar
- **THEN** the menu shows Delete label…, Export to crate ▸, Refresh label and Download all tracks, and nothing else

### Requirement: Delete label
Delete label… SHALL ask for confirmation, then stop following the label and delete its crate, as Delete crate does (playback from that crate stops). Passes SHALL be kept.

#### Scenario: Delete
- **WHEN** the user chooses Delete label… on "Label: Hardline Sounds" and confirms
- **THEN** the crate is deleted and Hardline Sounds is no longer followed, and a later follow of it leaves the tracks passed earlier out

### Requirement: Export to crate
Export to crate ▸ SHALL list the normal crates (not the Discogs, label or seller crates) and New crate…. Choosing a crate SHALL copy every entry of the label crate into it, in order, skipping entries it already holds, as Send to crate does. New crate… SHALL ask for a name, suggesting the label's name (for example "Siesta Records"), then create that crate with the copy and show it. The label SHALL stay followed and its crate SHALL be unchanged.

#### Scenario: Into a new crate
- **WHEN** the user chooses Export to crate ▸ New crate… on "Label: Siesta Records" with 23 entries and accepts "Siesta Records"
- **THEN** a crate "Siesta Records" with the same 23 entries is created and shown, can be edited freely, and "Label: Siesta Records" stays under LABELS with its 23 entries

#### Scenario: Into an existing crate
- **WHEN** the user exports the same label to "Friday", which already holds 3 of its tracks
- **THEN** the other 20 are added to the end of "Friday", and "Friday" stays off screen if it wasn't shown

### Requirement: Download all tracks
Download all tracks SHALL download the previews of every entry of the label crate that isn't downloaded yet, searching first for those that need it, in the background: after the playing entry and the 3 ahead, within the same 2 download slots, in the crate's order. Downloads for this SHALL never make the cache delete other previews. Only one label SHALL download at a time; starting another replaces it.

A window SHALL open when it starts, without blocking the rest of the app, showing the label, a progress bar ("120 of 300 tracks"), the tracks downloading at that moment with their percentage, how many have no preview, and a Stop button that ends it at any moment, keeping what is downloaded. Closing the window SHALL only hide it; while it runs, the label crate's menu SHALL read "Downloading (N of M)…" instead of Download all tracks, and choosing it SHALL show the window again.

Once the preview cache reaches its size, no more of the label's previews SHALL start (the ones already downloading finish, so the cache can end up to one preview past it). The window SHALL then show again, if hidden, saying it is paused because the preview cache (2 GB) is full, with Raise cache to 4 GB (twice the size), Stop, and a link to Options › Discogs…. Raising the size, there or in Options › Discogs…, SHALL save it and resume. When everything is downloaded, the window SHALL close and the main window SHALL say "Label: Siesta Records: all 300 tracks downloaded", counting the entries that can't get a preview ("no clip", failed).

#### Scenario: All of a label
- **WHEN** the user chooses Download all tracks on "Label: Siesta Records" with 40 tracks while another crate plays
- **THEN** the playing crate's previews still come first, the label's previews download 2 at a time behind them, and the main window then says "Label: Siesta Records: all 40 tracks downloaded"

#### Scenario: Progress and Stop
- **WHEN** the download is at 40 of 300 tracks and the user clicks Stop in its window
- **THEN** no more of the label's previews start, the 40 downloaded stay in the cache, the window closes, and the main window says it stopped at 40 of 300

#### Scenario: Hidden and shown again
- **WHEN** the user closes the window while it downloads, then right-clicks the label crate
- **THEN** the download goes on, the menu reads "Downloading (N of M)…", and choosing it shows the window

#### Scenario: Cache full
- **WHEN** the label's 300 tracks don't fit in the 2 GB cache, and 180 are downloaded
- **THEN** downloading pauses, no other preview is deleted, and the window shows that the cache is full with Raise cache to 4 GB and Stop

#### Scenario: Raise and go on
- **WHEN** the user chooses Raise cache to 4 GB
- **THEN** the cache size in Options › Discogs… is 4 GB, and the remaining 120 previews download

### Requirement: Move to Labels
Right-clicking a normal crate, not empty, whose entries were all sent from the same label's page SHALL offer Move to Labels. It SHALL make that crate the label's crate, under LABELS, keeping its name and entries. When the label already has a crate, the item SHALL NOT be offered.

#### Scenario: An existing label crate
- **WHEN** the crate "Label: Siesta Records" was made by sending the Siesta Records page before this change, and the user chooses Move to Labels
- **THEN** it moves under LABELS with its 23 entries, and from then on takes no hand edits and offers Refresh label

#### Scenario: Mixed crate
- **WHEN** a crate holds tracks sent from two labels' pages
- **THEN** its menu doesn't offer Move to Labels

### Requirement: Label pages from the browser
A label page sent from the browser extension SHALL follow that label: its crate is created under LABELS and filled from the page, or, when the label is already followed, refreshed as Refresh label does. It SHALL NOT change the shown crate, SHALL NOT start or interrupt playback, and SHALL NOT bring the app to the front, so several labels can be sent one after another from the browser. Each send SHALL be acknowledged in the main window. Sends of several labels SHALL fill their crates through the same queue as other sends.

#### Scenario: Several labels
- **WHEN** the user sends Siesta Records, Hardline Sounds and Lowtide Tapes from the browser, one after another, while a track plays
- **THEN** three crates appear under LABELS and fill, the track plays on, the shown crate doesn't change, and the browser stays in front

#### Scenario: Already followed
- **WHEN** Siesta Records is followed and its page is sent from the browser again
- **THEN** no new crate is made, and "Label: Siesta Records" is refreshed

#### Scenario: Paste is unchanged
- **WHEN** the user copies a label's Discogs address and presses Cmd+V in the player window
- **THEN** the label's tracks are added to the crate on screen, as before, and nothing is followed

### Requirement: Labels never touch playback
Following, filling, refreshing, exporting, downloading and deleting label crates SHALL run off the UI thread and the audio path, SHALL cause zero underruns, and SHALL NOT delay a start or a seek.

#### Scenario: Refresh while playing
- **WHEN** a label with 300 records is refreshed while a track plays
- **THEN** playback has zero underruns, and seeks stay under 50 ms
