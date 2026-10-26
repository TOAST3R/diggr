## MODIFIED Requirements

### Requirement: Crate menu
While the crate sidebar isn't shown (the playlist narrower than 600 skin pixels), clicking the playlist title bar without dragging SHALL open a menu listing every crate by name, marking the shown crate and the playing crate, followed by New crate…, Rename crate… and Delete crate…. It SHALL mark the crate the current track comes from with ⏵ only while that track plays or is paused. It SHALL list the user's own crates first, then the user's Discogs crates under a Discogs heading (the wantlist crate, then the collection crate), then the label crates under a Labels heading, the seller crates under a Top sellers heading and the friend crates under a Friends heading, each group in the sidebar's order and each heading shown only when its group has crates. Label, seller and friend crates SHALL NOT be listed among the user's own crates. When the crates don't fit in two thirds of the app window's height, the list of crates SHALL scroll, with the shown crate scrolled into view when the menu opens; New crate…, Rename crate…, Delete crate… and the refresh items SHALL stay visible below it. While the sidebar is shown, clicking the title bar SHALL NOT open a menu: the sidebar lists, creates, renames and deletes crates. Choosing a crate SHALL show it. Dragging the title bar SHALL still move the window and SHALL NOT open the menu. Deleting a crate that has entries SHALL ask for confirmation first.

#### Scenario: Switch crate
- **WHEN** the playlist is 400 px wide and the user clicks the title bar and chooses "Keepers"
- **THEN** the playlist window shows the Keepers crate and the title bar reads KEEPERS

#### Scenario: No menu beside the sidebar
- **WHEN** the playlist is 700 px wide and the user clicks the title bar
- **THEN** no menu opens

#### Scenario: Drag still moves the window
- **WHEN** the user drags the playlist title bar
- **THEN** the window moves and no menu opens

#### Scenario: New crate
- **WHEN** the user chooses New crate… and enters "Gig 12 Oct"
- **THEN** an empty crate named "Gig 12 Oct" is created and shown

#### Scenario: Delete asks first
- **WHEN** the user chooses Delete crate… for a crate with 40 entries
- **THEN** the crate is deleted only after the user confirms

#### Scenario: Discogs crates last
- **WHEN** the crates are Playlist, Friday, "Wantlist: digger" and "Collection: digger"
- **THEN** the menu lists Playlist and Friday, then under the Discogs heading "Wantlist: digger" and "Collection: digger", in that order

#### Scenario: Grouped
- **WHEN** the crates are Playlist, "Label: Siesta Records", "Seller: decks.de" and "Friend: javimaxilo"
- **THEN** the menu lists Playlist, then "Label: Siesta Records" under Labels, "Seller: decks.de" under Top sellers and "Friend: javimaxilo" under Friends

#### Scenario: A long menu scrolls
- **WHEN** the user has 5 crates, 14 labels, 31 sellers and 9 friends, the app window is 700 px tall, and the shown crate is "Seller: hhv"
- **THEN** the menu is no taller than the window, its crate list scrolls with "Seller: hhv" in view, and New crate…, Rename crate… and Delete crate… are visible without scrolling

### Requirement: Crate sidebar
When the playlist is at least 600 skin pixels wide, a list of every crate SHALL be shown on the left of the playlist, with each crate's number of records (albums, and entries of no album; see `record-view`), whether it is grouped or flat, and a tooltip naming it. The shown crate SHALL be marked •. The crate the current track comes from SHALL be marked with the player's play sign while it plays and its pause sign while paused, and not marked when playback is stopped; its tooltip SHALL say so. The user's own Discogs crates (the wantlist crate once a token is set, and the collection crate the app makes from their collection, both still recognised after a rename) SHALL be pinned at the bottom of the sidebar under a DISCOGS heading, the wantlist crate above the collection crate, both drawn in the OWNED badge's colour (off-white in the default skin) with a record icon; they SHALL NOT fold. The other crates SHALL be listed from the top, in creation order, under a CRATES heading showing ⏵ or ⏷ and their number, followed by "+ New crate". Clicking the CRATES heading SHALL fold or unfold that list, independently of the Discogs groups, and the fold SHALL be remembered across restarts. Under DISCOGS, below the collection crate, a LABELS heading SHALL list the label crates (see `label-crates`) in the order they were followed, each with its name and number of records. Right-clicking a label crate SHALL offer its menu (see `label-crates`). Below LABELS, a TOP SELLERS heading SHALL list the seller crates (see `seller-crates`) in the list's order, each with its seller's name and number of records. A seller crate never dug SHALL be dimmed and show no count. Right-clicking the TOP SELLERS heading SHALL offer Add seller…. Below TOP SELLERS, a FRIENDS heading SHALL list the friend crates (see `friend-crates`) in the friends list's order, each with its name and number of records; a private friend's crate SHALL be dimmed with no count. FRIENDS SHALL NOT be shown without a token or with no friends, and its heading SHALL offer no menu.

LABELS, TOP SELLERS and FRIENDS SHALL form an accordion: at most one of them SHALL be open, showing its crates; the others SHALL show only their heading. Each heading SHALL show ⏷ when open or ⏵ when closed, and its number of crates. Clicking a closed heading SHALL open it and close the open one; clicking the open heading SHALL close it, leaving all three closed. Which group is open SHALL be remembered across restarts; after upgrading from a version with separate folds, all three SHALL start closed. While TOP SELLERS or FRIENDS is closed and a crate in it is being dug or refreshed, its heading SHALL end with " …" and its tooltip SHALL say so.

Every heading, "+ New crate", and the pinned wantlist and collection crates SHALL always be visible. When the CRATES list or the open Discogs group holds more crates than fit, it SHALL scroll by row with the mouse wheel or trackpad while the pointer is over it, and SHALL show a thin scrollbar that can be dragged; the room left by the always-visible rows SHALL be shared between the two lists, each getting at least half when it needs it and the rest going to the other. When the sidebar is too short to give each open list at least one row, the whole sidebar SHALL scroll as one list in the same order. No crate SHALL be left out of the sidebar at any playlist height.

Whenever the shown crate changes, by any means (the sidebar, the title-bar crate menu, the browser extension, a new crate, a friend promoted to the user's crates, a deleted crate), its group SHALL open (unfolding CRATES, or opening its Discogs group) and its list SHALL scroll as little as needed to bring it into view. Scrolling by the user SHALL NOT be undone until the shown crate changes again.

Double-clicking a seller crate SHALL dig it (see `seller-crates`), and right-clicking it SHALL offer Refresh seller (once dug), Narrow down… (once dug), Rename crate… and Remove seller…. Narrower, the sidebar SHALL be hidden. There SHALL be no button or setting to hide it. Clicking a crate SHALL show it. Right-clicking a crate (or Control-clicking it on macOS, which SHALL NOT show it) SHALL offer Rename crate… and Delete crate… for that crate, with the same rules as the crate menu (deleting a crate with entries asks first). For the Playlist crate, which can't be renamed or deleted, the menu SHALL offer Clear crate instead and say why. "+ New crate" SHALL create and show a new crate, after asking its name as the crate menu does. A crate that can't be read SHALL be dimmed and can't be chosen. After a crate is clicked in the sidebar, the Delete key (or Backspace) SHALL delete that crate, with the same rules as Delete crate…, until the list is clicked or its cursor moves. The crate menu on the title bar SHALL keep working. Showing a crate from the sidebar SHALL NOT interrupt playback. Scrolling or folding the sidebar SHALL NOT change the shown crate or interrupt playback.

#### Scenario: Switch from the sidebar
- **WHEN** the playlist is 1000 px wide and the user clicks "Keepers" in the sidebar
- **THEN** the Keepers crate is shown, the sidebar marks it •, and the playing track plays on

#### Scenario: Too narrow
- **WHEN** the window is restored so the playlist is 400 px wide
- **THEN** no sidebar is shown, and the crate menu still works

#### Scenario: Delete from the sidebar
- **WHEN** the user right-clicks "Friday" in the sidebar and chooses Delete crate…
- **THEN** Friday is deleted (after confirming, when it has entries), whichever crate is shown

#### Scenario: Control-click on a Mac
- **WHEN** the user Control-clicks "Friday" in the sidebar on macOS
- **THEN** Friday's menu opens with Rename crate… and Delete crate…, and the shown crate doesn't change

#### Scenario: The Discogs crates stand apart
- **WHEN** the crates are Playlist, Friday, "Wantlist: digger" (the user's wantlist) and "Collection: digger" (the user's collection)
- **THEN** the sidebar lists "⏷ CRATES (2)", Playlist, Friday and + New crate from the top, and under DISCOGS at the bottom "Wantlist: digger" then "Collection: digger", both in the OWNED badge's colour with a record icon

#### Scenario: Wantlist before connecting
- **WHEN** no token is set and the crates are Playlist and Wantlist
- **THEN** Wantlist is listed among the user's crates under CRATES, not under DISCOGS

#### Scenario: Playing mark follows playback
- **WHEN** a track from Keepers plays, is paused, and is stopped
- **THEN** Keepers shows the play sign, then the pause sign, then no mark

#### Scenario: Playlist clears
- **WHEN** the user right-clicks Playlist in the sidebar
- **THEN** the menu offers Clear crate and says Playlist can't be renamed or deleted; Clear crate empties it

#### Scenario: Delete key
- **WHEN** the user clicks "Friday" in the sidebar and presses Delete (or Backspace)
- **THEN** Friday is deleted, after confirming when it has entries; after a click or a cursor move in the list, Delete removes the selected entries instead, and the Playlist crate is never deleted this way

#### Scenario: Always there when it fits
- **WHEN** the window is widened so the playlist goes from 400 to 700 px
- **THEN** the sidebar appears, with no button to hide it

#### Scenario: An old setting
- **WHEN** the app starts with a `settings.ron` that has `crate_sidebar: false` and `labels_folded: true`
- **THEN** the settings load, the sidebar is shown, and LABELS, TOP SELLERS and FRIENDS are all closed

#### Scenario: Top Sellers
- **WHEN** TOP SELLERS is open and the list holds decks.de (dug, 420 records) and logon (never dug)
- **THEN** under DISCOGS, below the collection crate, "⏷ TOP SELLERS (2)" lists "Seller: decks.de" with 420 and "Seller: logon" dimmed with no count

#### Scenario: Opening one group closes the others
- **WHEN** LABELS is open and the user clicks the TOP SELLERS heading
- **THEN** TOP SELLERS shows its crates, LABELS reads "⏵ LABELS (‹n›)" with its crates hidden, and FRIENDS stays closed

#### Scenario: Closing the open group
- **WHEN** FRIENDS is open and the user clicks the FRIENDS heading, then restarts the app
- **THEN** LABELS, TOP SELLERS and FRIENDS all read ⏵ and show only their headings

#### Scenario: Remembered
- **WHEN** the user opens TOP SELLERS and restarts the app
- **THEN** TOP SELLERS is open, and LABELS and FRIENDS are closed

#### Scenario: Crates fold on their own
- **WHEN** TOP SELLERS is open and the user clicks the CRATES heading
- **THEN** CRATES reads "⏵ CRATES (‹n›)" with its crates hidden and "+ New crate" still shown, and TOP SELLERS stays open

#### Scenario: Many sellers scroll
- **WHEN** the sidebar has room for 20 rows, CRATES holds 3 crates, and TOP SELLERS is open with 31 sellers
- **THEN** all 3 crates are shown, TOP SELLERS shows as many sellers as the remaining rows allow with a scrollbar, every heading is visible, and scrolling over the sellers reaches the 31st

#### Scenario: Many crates and many sellers share the room
- **WHEN** CRATES holds 40 crates and TOP SELLERS is open with 31 sellers, and neither fits in half of the free rows
- **THEN** each list gets half of the free rows (the CRATES list one more when they are odd) and scrolls on its own

#### Scenario: Nothing is left out
- **WHEN** FRIENDS is open with 9 friends and the playlist is made as short as it goes
- **THEN** every crate, the wantlist and the collection can still be reached by scrolling the sidebar

#### Scenario: Revealed from elsewhere
- **WHEN** LABELS is open, and the browser extension adds the seller "hhv", whose crate becomes the shown crate and is the 25th of TOP SELLERS
- **THEN** TOP SELLERS opens, LABELS closes, and the list scrolls so "Seller: hhv" is visible and marked •

#### Scenario: The user's scroll stays
- **WHEN** the shown crate is "Seller: decks.de" and the user scrolls TOP SELLERS away from it
- **THEN** the list stays where the user left it until another crate is shown

#### Scenario: Busy while closed
- **WHEN** TOP SELLERS is closed and "Seller: decks.de" is being dug
- **THEN** the heading reads "⏵ TOP SELLERS (‹n›) …" and its tooltip says a seller is being dug; when the dig ends the " …" goes away

#### Scenario: Seller menu
- **WHEN** the user right-clicks "Seller: decks.de", which has been dug
- **THEN** the menu offers Refresh seller, Narrow down…, Rename crate… and Remove seller…

#### Scenario: Labels between the collection and the sellers
- **WHEN** the user follows Siesta Records, has the seller decks.de, and LABELS is open
- **THEN** under DISCOGS the sidebar lists the wantlist and collection crates, then LABELS with "Label: Siesta Records", then the TOP SELLERS heading, closed

#### Scenario: Records, not tracks
- **WHEN** the collection crate holds 6,322 tracks from 924 records and is shown flat
- **THEN** the sidebar shows 924 for it, and its tooltip reads "Collection: 301 (924 records, 6322 entries)"

#### Scenario: Friends under the sellers
- **WHEN** the user has the seller decks.de and the friends javimaxilo and Waxport (private), and FRIENDS is open
- **THEN** under DISCOGS the sidebar lists the TOP SELLERS heading, closed, then FRIENDS with "Friend: javimaxilo" and a dimmed "Friend: Waxport"

## REMOVED Requirements

### Requirement: Crates panel
**Reason**: It lived in the classic player column, in the EQ section's place, which is removed; at full-screen width the crate sidebar is always shown and does the same.
**Migration**: Use the crate sidebar (or, in a window restored narrower than 600 skin pixels, the title bar's crate menu). An old `show_crates` setting is ignored.
