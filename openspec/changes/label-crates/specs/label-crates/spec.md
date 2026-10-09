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
A label crate SHALL take tracks only from its label's page. Pasting into it, dropping files or entries on it, and Send to crate into it SHALL be refused, and so SHALL Delete, dragging entries out, and Clear crate. Each refusal SHALL say in the main window that label crates fill from their label and that Pass (N) dims a track. Pass (N), Undo pass, Y, I and the rest of the entry menu SHALL work as in any crate; a passed track SHALL stay in the crate, dimmed.

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
Right-clicking a label crate, in the sidebar or the title-bar crate menu, SHALL offer Refresh label. It SHALL send the label's page again into its crate with skip passed on, so only tracks the crate doesn't hold, and that weren't passed, come in. Refreshing SHALL happen only when the user asks. When it finishes, the main window SHALL say how many records came in ("Label: Siesta Records: 4 new records"), or "Label: Siesta Records: up to date". While it runs the item SHALL read "Refreshing…" and be disabled. A failed refresh SHALL change nothing and say why.

#### Scenario: New records
- **WHEN** Siesta Records released 4 records since its crate was filled, and the user chooses Refresh label
- **THEN** their tracks come in, every other entry stays where it was, and the main window says "Label: Siesta Records: 4 new records"

#### Scenario: Nothing new
- **WHEN** the label has released nothing since the last refresh, and some of its tracks were passed
- **THEN** no entry is added or doubled, the passed tracks stay dimmed, and the main window says "Label: Siesta Records: up to date"

#### Scenario: Offline
- **WHEN** Discogs is unreachable and the user chooses Refresh label
- **THEN** the crate is unchanged, and the main window says the refresh failed because Discogs is offline

### Requirement: Create a crate from label
Right-clicking a label crate SHALL offer Create a crate from label. It SHALL create a normal crate named after the label (for example "Siesta Records", numbered when that name is taken) holding a copy of the label crate's entries in their order, passes included, and show it. The label SHALL stay followed and its crate SHALL be unchanged.

#### Scenario: A free crate
- **WHEN** the user chooses Create a crate from label on "Label: Siesta Records" with 23 entries
- **THEN** a crate "Siesta Records" with the same 23 entries is created and shown, can be edited freely, and "Label: Siesta Records" stays under LABELS with its 23 entries

### Requirement: Remove label
Right-clicking a label crate SHALL offer Remove label…. It SHALL ask for confirmation, then stop following the label and delete its crate, as Delete crate does (playback from that crate stops). Passes SHALL be kept.

#### Scenario: Remove
- **WHEN** the user chooses Remove label… on "Label: Hardline Sounds" and confirms
- **THEN** the crate is deleted and Hardline Sounds is no longer followed, and a later follow of it leaves the tracks passed earlier out

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
Following, filling, refreshing, copying and removing label crates SHALL run off the UI thread and the audio path, SHALL cause zero underruns, and SHALL NOT delay a start or a seek.

#### Scenario: Refresh while playing
- **WHEN** a label with 300 records is refreshed while a track plays
- **THEN** playback has zero underruns, and seeks stay under 50 ms
