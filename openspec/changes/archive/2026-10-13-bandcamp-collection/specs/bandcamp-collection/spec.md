## ADDED Requirements

### Requirement: Fan pages
The app SHALL accept a Bandcamp fan's collection page `https://bandcamp.com/‹user›` and wishlist page `https://bandcamp.com/‹user›/wishlist`, with or without `www.` and a trailing `/`, where ‹user› is 1 to 64 lowercase letters, digits, `_` or `-`. A query string or fragment SHALL be ignored. Bandcamp's own top-level pages (such as `/discover`, `/tag/…`, `/search`, `/about`, `/help`, `/login`, `/settings`) SHALL be refused as other unsupported addresses are. A page that has no fan data when read SHALL be refused with "Not a Bandcamp fan page", and nothing SHALL change. Fan pages SHALL be accepted by paste and from the browser extension, whatever the send mode.

#### Scenario: Collection page
- **WHEN** `https://bandcamp.com/toast3r?from=menubar` is pasted
- **THEN** it is recognised as the collection of fan "toast3r", and the import starts

#### Scenario: Wishlist page
- **WHEN** `https://bandcamp.com/toast3r/wishlist` is sent from the extension
- **THEN** it is recognised as the wishlist of fan "toast3r", and the import starts

#### Scenario: Bandcamp's own page
- **WHEN** `https://bandcamp.com/discover` is pasted
- **THEN** a message lists the supported kinds of page, and nothing is read

#### Scenario: Hostile address
- **WHEN** the address is `https://bandcamp.com.attacker.net/toast3r` or `https://bandcamp.com/toast3r;rm`
- **THEN** it is refused, and no request is sent

### Requirement: Reading a fan's lists
Importing a fan SHALL read both their collection and their wishlist, whichever page was sent. It SHALL read the fan page once for the fan id and the first items of each list, then ask Bandcamp's collection service for the rest, at most 100 items per request and at least 1 s apart, until it says there are no more. Each item SHALL be known by its Bandcamp page (album or track), artist, title, catalogue number (as `bandcamp-intake` takes it from the title) and kind: **physical** for a package (a record, cassette or CD bought on Bandcamp), **digital** otherwise. Each item's page SHALL then be read as a pasted album or track page is, at least 1 s apart. An item whose page can't be read SHALL be counted as unreadable with the reason, and the import SHALL go on. Reading SHALL run off the UI thread and the audio path. When the fan page or the collection service can't be read, no crate SHALL change and the main window SHALL say why.

#### Scenario: Full lists
- **WHEN** fan "toast3r" with 99 collection items and 103 wishlist items is imported
- **THEN** all 99 and 103 items are listed, with the collection service asked once per list beyond the page's first 20

#### Scenario: Physical purchase
- **WHEN** the collection holds the package "DJ Jackum - Context"
- **THEN** it is a physical item, and its tracks are read from `https://djjackum.bandcamp.com/album/context`

#### Scenario: Read while playing
- **WHEN** 200 items are read while a track plays
- **THEN** playback has zero underruns, and seeks stay under 50 ms

#### Scenario: Bandcamp unreachable
- **WHEN** the collection service doesn't answer
- **THEN** no crate changes, and the main window says the Bandcamp lists couldn't be read

### Requirement: Into the collection and wantlist crates
The collection's tracks SHALL be merged into the collection crate, and the wishlist's into the wantlist crate, by the merge rules of `bandcamp-intake` (a track that plays is skipped, a failed one is fixed, a missing one is added), so that no track is in a crate twice. A track the crate already holds from another source (a Discogs record) SHALL get the Bandcamp track too, keeping what it plays, so the list holds it and it can switch to Bandcamp. The shown crate SHALL NOT change. When no collection crate (or wantlist crate) exists, the import SHALL create "Collection: ‹user›" (or "Wantlist: ‹user›") with the fan's Bandcamp username and make it the collection crate (or wantlist crate), with or without a Discogs token. A Discogs token saved later SHALL use these crates and SHALL NOT create others.

#### Scenario: Bought on both
- **WHEN** the collection crate holds "Patricia - The Ooze" from Discogs release 1001 (AF070), and the Bandcamp collection holds the album AF070
- **THEN** no second "The Ooze" is added

#### Scenario: Linked, not doubled
- **WHEN** the wantlist crate holds "Nightcraft - Glasshouse" from Discogs release 1001 playing from YouTube, and the wished album LT-012 holds that track
- **THEN** the entry gets the Bandcamp track, still plays from YouTube, and no second "Glasshouse" is added

#### Scenario: No token
- **WHEN** no Discogs token is set, no collection or wantlist crate exists, and fan "toast3r" is imported
- **THEN** crates "Collection: toast3r" and "Wantlist: toast3r" are created under DISCOGS in the sidebar and filled, and nothing is sent to Discogs

#### Scenario: Token later
- **WHEN** the user then saves a Discogs token for "digger"
- **THEN** no "Collection: digger" crate is created, and "Collection: toast3r" is the collection crate

### Requirement: Two wantlists
An entry SHALL belong in the wantlist crate while its record is on the Discogs wantlist **or** its Bandcamp page is on the Bandcamp wishlist. Refresh wantlist SHALL remove an entry only when neither holds it. In the same way, Refresh collection SHALL remove an entry from the collection crate only when neither the Discogs collection nor the Bandcamp collection holds it.

#### Scenario: Off one list only
- **WHEN** "Nightcraft - Glasshouse" is on both lists, the user removes it from the Bandcamp wishlist, and chooses Refresh wantlist
- **THEN** it stays in the wantlist crate

#### Scenario: Off both lists
- **WHEN** an album was only on the Bandcamp wishlist and the user removed it there
- **THEN** after Refresh wantlist its entries leave the wantlist crate, and the main window counts it as gone

### Requirement: Onto Discogs when Discogs has it
With a Discogs token, each item SHALL be looked up on Discogs once, by artist, title and catalogue number (when the item has one), and kept only among releases of its kind: format File for a digital collection item, any other format for a physical collection item and for every wishlist item (a wish doesn't say its format, and the wantlist is for records). The item's normalized artist and title SHALL equal the release's, and so SHALL the catalogue numbers when both have one. When exactly one release matches:
- for a collection item, when the Discogs collection doesn't hold that release, it SHALL be added to the Discogs collection;
- for a wishlist item, when that release isn't wanted, it SHALL be added to the Discogs wantlist;
- its tracks SHALL take over the item's Bandcamp entries in the crate (see `bandcamp-intake`).

When no release or more than one matches, nothing SHALL be written to Discogs. Nothing SHALL ever be removed from Discogs by an import or refresh. An item looked up once SHALL NOT be looked up again on later refreshes. Without a token, no Discogs request SHALL be made.

#### Scenario: Digital purchase on Discogs
- **WHEN** the Bandcamp collection holds a digital purchase of the album AF070, and Discogs has exactly one File release 2001 with catalogue number AF070 by that artist with that title, which the collection doesn't hold
- **THEN** release 2001 is added to the Discogs collection, and the vinyl release 1001 of the same master is not

#### Scenario: A wish is for the record
- **WHEN** the Bandcamp wishlist holds the album AF070, and Discogs has File release 2001 and vinyl release 1001 of it
- **THEN** release 1001 is added to the Discogs wantlist, and release 2001 is not

#### Scenario: Already owned
- **WHEN** the matching release is already in the Discogs collection
- **THEN** nothing is written, and the report counts it as already owned

#### Scenario: Two candidates
- **WHEN** a physical item matches a 2019 vinyl and a 2023 repress with the same catalogue number
- **THEN** nothing is written, and the report lists the item as not added (2 candidates)

#### Scenario: Bandcamp-only release
- **WHEN** Discogs has no release by that artist with that title
- **THEN** nothing is written, and the report lists the item as not added (not on Discogs)

### Requirement: Refreshing the Bandcamp lists
Once a fan has been imported, Refresh collection SHALL also read the fan's Bandcamp collection again, and Refresh wantlist the fan's Bandcamp wishlist; each SHALL read only its own list, and SHALL change only its own crate and its own Discogs list. Only items not seen before SHALL be read and looked up, and items no longer listed SHALL be dropped as the two-wantlists rule says. With no Discogs token, both refresh items SHALL still be offered, and SHALL read only Bandcamp. Sending either of the fan's pages again SHALL refresh both lists the same way.

#### Scenario: One new purchase
- **WHEN** the fan bought one album on Bandcamp since the import, and the user chooses Refresh collection
- **THEN** only that album's page is read and looked up on Discogs, and the main window says "Collection: 1 new"

#### Scenario: Only its own list
- **WHEN** the fan added an album to the Bandcamp wishlist and bought another since the import, and the user chooses Refresh wantlist
- **THEN** only the wishlist is read, the new wishlist album comes into the wantlist crate, and the bought album is neither read nor added to the Discogs collection

#### Scenario: No token
- **WHEN** no token is set and fan "toast3r" was imported
- **THEN** right-clicking "Collection: toast3r" offers Refresh collection, which reads only Bandcamp

### Requirement: Import progress and Stop
While an import or a refresh that reads Bandcamp runs, a "Bandcamp import" window SHALL show a progress bar and, in turn:
- "Reading your Bandcamp collection… ‹k› of ‹n› items" (or wishlist);
- "Reading albums… ‹k› of ‹n›";
- with a token, "Matching on Discogs… ‹k› of ‹n›".

A Stop button SHALL end it at any moment, after the current request. Items already read and merged SHALL stay, items not yet looked up on Discogs SHALL be looked up on the next refresh, and the main window SHALL say "Bandcamp import stopped". Stopping SHALL NOT affect playback.

#### Scenario: Reading albums
- **WHEN** 40 of 99 collection albums have been read
- **THEN** the window shows "Reading albums… 40 of 99" with the bar at 40 %

#### Scenario: Stop
- **WHEN** the user clicks Stop while matching item 30 of 99 on Discogs
- **THEN** no more requests are sent, the crates keep what was merged, and the next Refresh collection looks up the other 69

### Requirement: Import report
When an import or a refresh that read Bandcamp ends, or is stopped, a "Bandcamp import" window SHALL open and stay until closed. It SHALL show:
- for each crate, how many records were added and how many were already there;
- on Discogs, how many records were added to the collection and to the wantlist, how many were already owned or wanted, and how many weren't looked up (no token, or stopped);
- a foldable list of the items not added to Discogs, each with its artist, title, kind, why (not on Discogs, ‹n› candidates), and links that open its Bandcamp page and a Discogs search for it;
- a foldable list of the items that couldn't be read, with the reason;
- that the run was stopped, when it was.

Copy SHALL put the report on the clipboard as plain text. The last report SHALL be kept across restarts and reopen from the collection and wantlist crates' menus (Last Bandcamp import…).

#### Scenario: After the first import
- **WHEN** the import of 99 collection and 103 wishlist items ends with 41 added to the Discogs collection, 55 to the wantlist and 74 not found
- **THEN** the report shows those counts, and its "Not added to Discogs" list holds 74 items, each with Bandcamp and Discogs search links

#### Scenario: Copy
- **WHEN** the user clicks Copy
- **THEN** the clipboard holds the report as plain text, one item per line under its heading

#### Scenario: Reopen
- **WHEN** the app is restarted and the user chooses Last Bandcamp import… on the collection crate
- **THEN** the same report opens
