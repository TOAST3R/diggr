## MODIFIED Requirements

### Requirement: Crate menu
While the crate sidebar isn't shown (the playlist narrower than 600 skin pixels and not maximized), clicking the playlist title bar without dragging SHALL open a menu listing every crate by name, marking the shown crate and the playing crate, followed by New crate…, Rename crate… and Delete crate…. It SHALL mark the crate the current track comes from with ⏵ only while that track plays or is paused, and list the user's Discogs collection crate last, under a Discogs heading. While the sidebar is shown, clicking the title bar SHALL NOT open a menu: the sidebar lists, creates, renames and deletes crates. Choosing a crate SHALL show it. Dragging the title bar SHALL still move the window and SHALL NOT open the menu. Deleting a crate that has entries SHALL ask for confirmation first.

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


### Requirement: Crate sidebar
When the playlist is at least 600 skin pixels wide, or maximized, a list of every crate SHALL be shown on the left of the playlist, with each crate's number of entries and a tooltip naming it. The shown crate SHALL be marked •. The crate the current track comes from SHALL be marked with the player's play sign while it plays and its pause sign while paused, and not marked when playback is stopped; its tooltip SHALL say so. The user's own Discogs collection crate (the one the app makes from their collection, still recognised after a rename) SHALL be pinned at the bottom of the sidebar under a DISCOGS heading, drawn in the OWNED badge's amber with a record icon; the other crates are listed from the top in creation order, followed by "+ New crate". Narrower, it SHALL be hidden. There SHALL be no button or setting to hide it. Clicking a crate SHALL show it. Right-clicking a crate (or Control-clicking it on macOS, which SHALL NOT show it) SHALL offer Rename crate… and Delete crate… for that crate, with the same rules as the crate menu (deleting a crate with entries asks first). For the Playlist crate, which can't be renamed or deleted, the menu SHALL offer Clear crate instead and say why. "+ New crate" SHALL create and show a new crate, after asking its name as the crate menu does. A crate that can't be read SHALL be dimmed and can't be chosen. After a crate is clicked in the sidebar, the Delete key (or Backspace) SHALL delete that crate, with the same rules as Delete crate…, until the list is clicked or its cursor moves. The crate menu on the title bar SHALL keep working. Showing a crate from the sidebar SHALL NOT interrupt playback.

#### Scenario: Switch from the sidebar
- **WHEN** the playlist is maximized and the user clicks "Keepers" in the sidebar
- **THEN** the Keepers crate is shown, the sidebar marks it •, and the playing track plays on

#### Scenario: Too narrow
- **WHEN** the playlist is 400 px wide and not maximized
- **THEN** no sidebar is shown, and the crate menu still works

#### Scenario: Delete from the sidebar
- **WHEN** the user right-clicks "Friday" in the sidebar and chooses Delete crate…
- **THEN** Friday is deleted (after confirming, when it has entries), whichever crate is shown

#### Scenario: Control-click on a Mac
- **WHEN** the user Control-clicks "Friday" in the sidebar on macOS
- **THEN** Friday's menu opens with Rename crate… and Delete crate…, and the shown crate doesn't change

#### Scenario: The collection stands apart
- **WHEN** the crates are Playlist, Keepers and "Collection: digger" (the user's collection)
- **THEN** the sidebar lists Playlist, Keepers and + New crate from the top, and "Collection: digger" in amber with a record icon on its bottom row, under DISCOGS

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
- **WHEN** the playlist is widened from 400 to 700 px
- **THEN** the sidebar appears, with no button to hide it

#### Scenario: An old setting
- **WHEN** the app starts with a `settings.ron` that has `crate_sidebar: false` and the playlist maximized
- **THEN** the settings load, and the sidebar is shown
