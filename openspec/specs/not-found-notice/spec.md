# not-found-notice Specification

## Purpose
Telling the user when a record has no preview anywhere: when a record counts as fully tried, the blocking No preview found notice (chained, once a session, with Retry and a search on the other store), "n/m previews" on partly found records, and a clicked track that isn't found handing over to the next track of its record.
## Requirements
### Requirement: Fully tried records
A record in a crate SHALL count as fully tried when every one of its entries, leaving out passed and "already in crate" entries, is playable or unavailable ("no clip", "clip failed" or "not found by search"), and no search, clip download, album-upload fallback or Bandcamp fallback is queued or running for any of them. A fully tried record with no playable entry SHALL count as not found; with some, as partly found. The check SHALL run only for the record an outcome touches and SHALL NOT run on the audio path.

#### Scenario: Bandcamp still pending
- **WHEN** all 4 tracks of a record are "not found by search" and a Bandcamp look for one of them is queued
- **THEN** the record is not yet fully tried

#### Scenario: Not found
- **WHEN** the last Bandcamp look for a record whose 4 tracks are "not found by search" finds nothing
- **THEN** the record counts as not found

### Requirement: No preview notice
When a record comes to count as not found, the app SHALL show a dialog titled "No preview found" listing it with its artist, title and year, and the sources tried (of YouTube search, the record's album upload, Bandcamp and store samples, those that were tried). For a record whose year is the current year or later, its line SHALL say "Probably not released yet: try again after its release"; otherwise "No preview anywhere yet". Each record SHALL have a Retry button; the dialog SHALL have Retry all and Close. Records that come to count as not found while it is open SHALL be added to its list; a second dialog SHALL NOT open. The dialog SHALL hold the keyboard until closed, SHALL close on Close, Esc or Enter, and SHALL NOT pause, stop or delay playback.

#### Scenario: One record
- **WHEN** the last track of "Unknown Artist – Forthcoming EP (2026)" is not found on any source during playback
- **THEN** the dialog shows it with "YouTube, Bandcamp" tried and "Probably not released yet: try again after its release", and the music keeps playing

#### Scenario: Chained
- **WHEN** the dialog lists one record and two more records come to count as not found
- **THEN** the same dialog lists three records

#### Scenario: Older record
- **WHEN** a 1998 record counts as not found
- **THEN** its line says "No preview anywhere yet"

#### Scenario: Keys held
- **WHEN** the dialog is open and the user presses N
- **THEN** nothing is passed; pressing Esc closes the dialog

### Requirement: Retry from the notice
Retry SHALL run again, for each unavailable entry of the record, its clip download ("clip failed") or its search ("not found by search"), and allow its Bandcamp fallback again; the record SHALL leave the dialog. Retry all SHALL do this for every listed record and close the dialog. A retried record SHALL be shown again only if it again counts as not found.

#### Scenario: Retry
- **WHEN** the user clicks Retry on a listed record of 3 "not found by search" tracks
- **THEN** its 3 entries show "to search", it leaves the list, and they are searched when they come near the playhead

### Requirement: Once per record
A record SHALL appear in the notice at most once per session unless retried. A record that already counts as not found when its crate loads SHALL NOT open the notice.

#### Scenario: Launch
- **WHEN** the app starts with a crate holding a record whose tracks were all "not found by search" last session
- **THEN** no dialog opens

### Requirement: Partly found records
In the grouped view, the dimmed second line of a partly found record's row SHALL end with "· ‹n›/‹m› previews", n being its playable entries and m its entries (its open track rows keep saying "Preview: not found by search" in their tooltips). The right-click menu of its record row and of its track rows SHALL offer Retry missing tracks, which retries them as Retry does. A partly found record SHALL NOT open the notice.

#### Scenario: Two of five
- **WHEN** 2 of a record's 5 tracks have a preview and the other 3 are fully tried and not found
- **THEN** its record row's second line ends with "· 2/5 previews" and no dialog opens

### Requirement: A clicked track not found hands over to its record's next
When a track the user started (armed: double-click, Enter or Play) turns out to have no preview (not found by search, or its clip failed), the next track of the same record in crate order that plays or may still play SHALL be armed in its place, so its preview is searched for or downloaded next, and SHALL start as soon as it can; the main window SHALL say "‹track›: no preview, trying the next track". When no such track is left, nothing SHALL be armed (the record then counts as fully tried, see No preview notice). Other records SHALL NOT be armed this way.

#### Scenario: Two tracks not found
- **WHEN** nothing plays and the user double-clicks the 4th of 6 tracks to search of a record, and the 4th and 5th have no usable upload while the 6th has one
- **THEN** the 5th and then the 6th are searched without another click, and the 6th plays

#### Scenario: Last track of the record
- **WHEN** the user double-clicks the last track of a record and it is not found
- **THEN** nothing is armed, and no track of another record starts

### Requirement: Search the other store by hand
For a record from Discogs (with a release or master) the app SHALL offer "Search on Bandcamp", and for a record from Bandcamp only "Search on Discogs", opening that store's search for the record's artist and title (see `buy-chooser`, Search addresses) in the default browser: as a link on its line in the No preview found notice, and as an item of the right-click menu of its tracks. Local files and store records SHALL NOT offer it.

#### Scenario: From the notice
- **WHEN** the notice lists "Various – Sixty (2025)" from Discogs, whose tracks are by The 89th Passenger, and the user clicks Search on Bandcamp
- **THEN** `https://bandcamp.com/search?q=The%2089th%20Passenger%20Sixty&item_type=a` opens, and the notice stays open

#### Scenario: A Bandcamp track
- **WHEN** the user right-clicks a track sent from a Moodymann album on Bandcamp
- **THEN** the menu offers Search on Discogs, opening Discogs' search for "Moodymann Silentintroduction"

