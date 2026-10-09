## MODIFIED Requirements

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

## ADDED Requirements

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
