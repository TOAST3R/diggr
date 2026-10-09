# label-crates Specification

## Purpose
TBD - created by archiving change label-crates. Update Purpose after archive.
## Requirements
### Requirement: Label crates
The app SHALL keep one crate per followed label, named "Label: ‹name›". The name is the label's Discogs name, or, when only Bandcamp is followed, its Bandcamp name: taken from the page's title sent by the extension, else the `‹name›.bandcamp.com` part.

The crate SHALL remember what it follows: the label's Discogs id, its Bandcamp address (the `‹name›.bandcamp.com` part), or both. It SHALL:
- stay its label's crate after a rename;
- survive restarts;
- be grouped by record until the user toggles it.

Following a label that already has a crate SHALL NOT make a second one. Crates saved before this change, holding only a Discogs id, SHALL load unchanged.

#### Scenario: Survives a restart
- **WHEN** the app follows Siesta Records and Hardline Sounds, and is restarted
- **THEN** LABELS lists both crates with their entries

#### Scenario: Renamed
- **WHEN** the user renames "Label: Siesta Records" to "Siesta"
- **THEN** it stays under LABELS, and Refresh label still refreshes Siesta Records

#### Scenario: Grouped by default
- **WHEN** a label crate is shown for the first time
- **THEN** it is grouped by record

#### Scenario: Both sources survive a restart
- **WHEN** "Label: Analogical Force" follows Discogs label 123 and analogicalforce.bandcamp.com, and the app is restarted
- **THEN** it still follows both, and Refresh label reads both

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
Refresh label SHALL read every source the crate follows:
- **Discogs:** its label page again, with skip passed on;
- **Bandcamp:** only the albums not seen at the last refresh, merged as in `bandcamp-intake`.

Only tracks the crate doesn't hold, and that weren't passed, SHALL come in. Refreshing SHALL happen only when the user asks.

When it finishes, the main window SHALL say how many records came in from all sources ("Label: Siesta Records: 4 new records"), or "Label: Siesta Records: up to date". While it runs, the item SHALL read "Refreshing…" and be disabled. A failed refresh SHALL change nothing from the failed source and say why.

#### Scenario: New records
- **WHEN** Siesta Records released 4 records since its crate was filled, and the user chooses Refresh label
- **THEN** their tracks come in, every other entry stays where it was, and the main window says "Label: Siesta Records: 4 new records"

#### Scenario: Nothing new
- **WHEN** the label has released nothing since the last refresh, and some of its tracks were passed
- **THEN** no entry is added or doubled, the passed tracks stay dimmed, and the main window says "Label: Siesta Records: up to date"

#### Scenario: Offline
- **WHEN** Discogs is unreachable and the user chooses Refresh label
- **THEN** the crate is unchanged, and the main window says the refresh failed because Discogs is offline

#### Scenario: A new Bandcamp album
- **WHEN** "Label: Analogical Force" follows Discogs and Bandcamp, and the label put AF071 on Bandcamp only
- **THEN** Refresh label reads only AF071 from Bandcamp, adds its tracks, and counts it in "1 new record"

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
A label page sent from the browser extension SHALL follow that label, whether Discogs or Bandcamp. Its crate is created under LABELS and filled from the page, or, when the label is already followed, refreshed as Refresh label does. A Bandcamp label page SHALL first be matched with the followed labels as in "Matching a Bandcamp label".

It SHALL NOT change the shown crate, SHALL NOT start or interrupt playback, and SHALL NOT bring the app to the front, so several labels can be sent one after another from the browser. Each send SHALL be acknowledged in the main window. Sends of several labels SHALL fill their crates through the same queue as other sends.

#### Scenario: Several labels
- **WHEN** the user sends Siesta Records, Hardline Sounds and Lowtide Tapes from the browser, one after another, while a track plays
- **THEN** three crates appear under LABELS and fill, the track plays on, the shown crate doesn't change, and the browser stays in front

#### Scenario: Already followed
- **WHEN** Siesta Records is followed and its page is sent from the browser again
- **THEN** no new crate is made, and "Label: Siesta Records" is refreshed

#### Scenario: Paste is unchanged
- **WHEN** the user copies a label's Discogs or Bandcamp address and presses Cmd+V in the player window
- **THEN** the label's tracks are added to the crate on screen, as before, and nothing is followed

#### Scenario: A Bandcamp-only label
- **WHEN** the user sends `https://lowtidetapes.bandcamp.com/music` from the browser, and no followed label matches it
- **THEN** a crate "Label: Lowtide Tapes" appears under LABELS and fills from its Bandcamp albums

### Requirement: Labels never touch playback
Following, filling, refreshing, exporting, downloading and deleting label crates SHALL run off the UI thread and the audio path, SHALL cause zero underruns, and SHALL NOT delay a start or a seek.

#### Scenario: Refresh while playing
- **WHEN** a label with 300 records is refreshed while a track plays
- **THEN** playback has zero underruns, and seeks stay under 50 ms

### Requirement: Download paused by YouTube
While YouTube is limiting requests (see `preview-fetch`), Download all tracks' window SHALL show, instead of the tracks downloading, "Paused: YouTube is limiting requests. Trying again in ‹N› min.", with a Try now button that tries at once. If that try is limited too, the wait SHALL go on as if it had been the scheduled try. The window SHALL come back if it was hidden when the pause starts.

#### Scenario: Paused and shown
- **WHEN** Download all tracks runs with its window hidden and YouTube starts limiting
- **THEN** the window shows again, says it is paused for YouTube and in how many minutes it tries again, and offers Try now and Stop

#### Scenario: Try now
- **WHEN** the user clicks Try now and YouTube answers normally
- **THEN** the pause line goes, and downloading carries on

### Requirement: Retry failed tracks
A label crate's tracks marked "clip failed" or "not found" SHALL be retryable. Download all tracks' window SHALL offer Retry failed (N) while N of the crate's tracks are retryable, and the label crate's menu SHALL offer Retry failed tracks (N) when it isn't downloading and N is above 0. Retrying SHALL put those tracks back to waiting: "clip failed" ones to be downloaded again, "not found" ones to be searched again, with their remembered "not found" forgotten. It SHALL then run them as part of Download all tracks, starting it if it isn't running. "No clip" and "already in crate" tracks SHALL NOT be retried. When everything is done, the summary SHALL count the tracks still without a preview.

#### Scenario: Retry from the menu
- **WHEN** "Label: Analogical Force" has 30 tracks "not found" and 4 "clip failed", and the user chooses Retry failed tracks (34)
- **THEN** the 30 are searched again, the 4 are downloaded again, Download all tracks' window opens with them, and the tracks that now get a preview play like any other

#### Scenario: Not everything is retryable
- **WHEN** the same crate also has 5 tracks "already in crate" and 2 with "no clip"
- **THEN** those 7 stay as they are, and the count reads 34

#### Scenario: Retry from the window
- **WHEN** Download all tracks' window shows 39 without a preview, 34 of them retryable
- **THEN** it offers Retry failed (34), and choosing it puts them back into the running download

### Requirement: Matching a Bandcamp label
A Bandcamp label sent while it isn't followed SHALL be matched by name with the followed labels. Names are compared:
- lowercase;
- without punctuation or spaces;
- without the words "records", "recordings", "music", "label", "rec" or "ltd" at the end, however many.

The outcome:
- **One label with the same name:** the Bandcamp label SHALL be added to that label's crate, which then follows both, and its albums merged in.
- **A close name** (one contains the other, or they differ by at most 2 letters): a dialog SHALL ask "Merge into Label: ‹name›?" with Merge and Separate.
  - Separate SHALL make a crate of its own.
  - The answer SHALL be remembered, so the dialog doesn't come back for that pair.
  - Several pending questions SHALL be asked one at a time, without holding up other sends.
- **No match:** a crate of its own SHALL be made.

The same check SHALL run when a Discogs label is followed while a Bandcamp-only label crate exists.

#### Scenario: Same name
- **WHEN** "Label: Analogical Force" follows its Discogs label, and analogicalforce.bandcamp.com ("Analogical Force") is sent from the browser
- **THEN** no new crate is made, the crate follows both, and the main window says "Merged into Label: Analogical Force"

#### Scenario: Trailing words
- **WHEN** "Label: Siesta" is followed, and the Bandcamp label "Siesta Records Ltd" is sent
- **THEN** after the trailing words are dropped the names are equal, and the label merges without a question

#### Scenario: Ask once
- **WHEN** "Label: Lowtide" is followed, the Bandcamp label "Lowtide Tapes" is sent, and the user chooses Separate
- **THEN** a crate "Label: Lowtide Tapes" is made, and sending it again refreshes that crate without asking

#### Scenario: Discogs after Bandcamp
- **WHEN** "Label: Lowtide Tapes" follows only Bandcamp, and the Discogs label Lowtide Tapes is sent from the browser
- **THEN** the crate follows both, and the Discogs records are merged in without doubling tracks it already holds

