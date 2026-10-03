# discogs-write Specification

## Purpose
Changes the user's Discogs account from the player: records go on the wantlist while digging and into the collection once bought, nothing owned stays wanted, the wantlist and collection crates follow Discogs, and every change either goes through or says why, without ever adding a copy twice or touching playback.
## Requirements
### Requirement: Add to wantlist
Add to wantlist SHALL act on a record: the Discogs release of the entry, or its master's main release. It SHALL:
- mark the release wanted, so every entry of it, in every crate, shows ★;
- add the record's clips to the wantlist crate, skipping clips the crate already holds, with no request when the release's data is cached;
- with a Discogs token, add the release to the user's Discogs wantlist.

It SHALL be offered as "Add to wantlist (Y)" in the entry menu and by `Y` on the playing or paused entry. When the release is already wanted, the item SHALL read "Remove from wantlist (Y)". That item SHALL take the release off the wantlist crate and the Discogs wantlist, whoever added it there. Local files SHALL NOT offer either item. The entry SHALL be marked wanted, and the wantlist crate updated, within one frame of the action, without waiting for Discogs.

#### Scenario: Add a record
- **WHEN** the user presses Y while "Nightcraft - Glasshouse" from release 123456, a record with three clips, plays, with a token set
- **THEN** all three clips are in the wantlist crate, every entry of release 123456 shows ★, and release 123456 is on the user's Discogs wantlist

#### Scenario: Remove a record
- **WHEN** the user chooses Remove from wantlist on an entry of release 123456, which was already on the Discogs wantlist before the app was used
- **THEN** its entries leave the wantlist crate, lose their ★, and release 123456 is removed from the Discogs wantlist

#### Scenario: Local file
- **WHEN** the user right-clicks a local file
- **THEN** the menu offers neither Add to wantlist nor Add to collection

### Requirement: Never want what you own
Add to wantlist SHALL NOT be possible for a record the user owns, in this pressing or another pressing of the same master release. For an owned record, the menu SHALL show "In collection", disabled, with a tooltip naming the owned pressing, and `Y` SHALL only say that the record is already in the collection, naming the pressing. No dialog SHALL offer to add it anyway.

#### Scenario: Another pressing owned
- **WHEN** the user presses Y on a track of release 111 (master 900) and the collection holds release 222 of master 900, catalog number AF001R, from 2019
- **THEN** nothing is added to the wantlist crate or the Discogs wantlist, and the main window says the record is already in the collection (AF001R, 2019)

#### Scenario: Menu for an owned record
- **WHEN** the user right-clicks an entry whose release is in the collection
- **THEN** the menu shows "In collection" disabled, instead of Add to wantlist

### Requirement: Add to collection
With a Discogs token, the entry menu SHALL offer Add to collection for an entry from a Discogs release. It SHALL add one copy of the release to the user's Discogs collection, in the Uncategorized folder, so that it shows under All. When the release is already in the collection, the item SHALL read "In collection" and be disabled. When another pressing of its master is owned, the item SHALL name that pressing ("Add to collection (own AF001R, 2019)") and SHALL act without asking. When Discogs confirms the add:
- the local collection SHALL hold the release at once, so that its entries show the OWNED badge in every crate within one frame, with no sync;
- the release SHALL be removed from the Discogs wantlist when it is on it, whoever added it;
- its entries SHALL leave the wantlist crate, and its clips SHALL be added to the collection crate, skipping clips already there.

The add SHALL use exactly one request when the release isn't wanted, and two (add and wantlist removal) when it is.

#### Scenario: Bought it
- **WHEN** the user chooses Add to collection on an entry of release 123456, which is on the wantlist
- **THEN** release 123456 is in the Discogs collection's Uncategorized folder and off the Discogs wantlist; its entries show OWNED, have left the wantlist crate, and are in the collection crate

#### Scenario: Next sync after an add
- **WHEN** a collection sync runs after the user added one record from the app, and nothing else changed on Discogs
- **THEN** the sync takes 1 request and does not read the whole collection again

#### Scenario: Already owned
- **WHEN** the user right-clicks an entry whose release is in the collection
- **THEN** the menu shows "In collection" disabled

### Requirement: Never add a copy twice
An add to the collection that fails SHALL NOT be retried automatically. The entry SHALL show "collection add failed" in its tooltip with a ⚑ in its row, the main window SHALL show the error once, and the menu SHALL offer Retry add to collection. Before adding again, a retry SHALL ask Discogs whether the release is already in the collection. When it is, the retry SHALL handle it as a successful add without adding another copy.

#### Scenario: Timed out but done
- **WHEN** an add times out after Discogs has stored it, and the user chooses Retry add to collection
- **THEN** the app finds the release in the collection, adds no second copy, and handles the add as successful

#### Scenario: Offline add
- **WHEN** the user chooses Add to collection while Discogs is unreachable
- **THEN** nothing is retried by itself, the main window says the add failed, and the entry's menu offers Retry add to collection

### Requirement: Acting on a selection
When the right-clicked entry is part of the selection, Add to wantlist, Remove from wantlist and Add to collection SHALL act on the distinct releases of the selected entries, with one request per release. The item SHALL count records ("Add 3 records to wantlist"). Records the action doesn't apply to (owned for Add to wantlist, already wanted or already owned, or with no release) SHALL be skipped, and the main window SHALL say how many were done and how many were skipped and why. `Y` SHALL act only on the playing or paused entry.

#### Scenario: Selection across records
- **WHEN** 8 selected entries come from 4 releases, one of them owned, and the user chooses Add to wantlist
- **THEN** 3 releases are added with 3 requests, and the main window says "Added 3 records to your wantlist; 1 skipped (already in your collection)"

#### Scenario: Same record twice
- **WHEN** 3 selected entries all come from release 123456 and the user chooses Add to collection
- **THEN** exactly one copy of release 123456 is added

### Requirement: The wantlist crate mirrors Discogs
The wantlist crate SHALL hold the records on the user's wantlist. Without a token, it SHALL be a crate named "Wantlist" among the user's own crates. When a token for ‹user› is saved:
- it SHALL become "Wantlist: ‹user›". When a crate of that name already exists, the local crate's entries SHALL be merged into it, and the local crate removed once the merge is saved;
- it SHALL be filled with the user's Discogs wantlist, in the same way as sending the wantlist page;
- every record in it that isn't on the Discogs wantlist and isn't owned SHALL be added to the Discogs wantlist.

Once per session, when the account is first known (a dig, a wantlist change, or the wantlist crate shown, with a token), it SHALL take in records newly wanted on Discogs, and drop entries of records no longer on the Discogs wantlist that have no change waiting. When an action on it would leave no entry of a wanted record in the crate (Remove, Remove album or the Delete key), the app SHALL first ask "Remove N records from your Discogs wantlist?". Remove SHALL take them off the Discogs wantlist, and Cancel SHALL change nothing. Removing only some clips of a record, or deleting the crate, SHALL NOT change the Discogs wantlist. Without a token, removing every entry of a record from the wantlist crate SHALL make it no longer wanted, without asking.

#### Scenario: Connecting
- **WHEN** the user has a "Wantlist" crate with 5 records, 1 of them owned, and saves a token for "digger", whose Discogs wantlist has 20 records
- **THEN** the crate is named "Wantlist: digger", holds the 20 records and the 4 not owned, and those 4 are added to the Discogs wantlist

#### Scenario: Removed on discogs.com
- **WHEN** a record was removed from the wantlist on discogs.com, and the app is started again with a token
- **THEN** that record is no longer wanted, and its entries leave the crate

#### Scenario: Remove a whole record
- **WHEN** the user selects all 3 entries of release 123456 in the wantlist crate and presses Delete
- **THEN** a dialog asks "Remove 1 record from your Discogs wantlist?"; Remove takes it off the crate and the Discogs wantlist, and Cancel leaves both unchanged

#### Scenario: Remove one clip
- **WHEN** the user removes 1 of the 3 entries of release 123456 from the wantlist crate
- **THEN** no dialog opens, and release 123456 stays on the Discogs wantlist

### Requirement: Refresh from Discogs
With a Discogs token, right-clicking the wantlist crate or the collection crate in the crate sidebar, and the title-bar crate menu, SHALL offer Refresh wantlist and Refresh collection. Each SHALL make the crate match Discogs, in one direction only (Discogs is never changed by it):
- Refresh collection SHALL sync the collection (incrementally, as a weekly sync does). Records in the Discogs collection that the crate doesn't hold SHALL then be fetched into it, and the entries of records no longer in the collection SHALL leave it;
- Refresh wantlist SHALL read the Discogs wantlist again, even if it was read this session. Records newly wanted SHALL be fetched into the wantlist crate, and records no longer on it, with no change waiting, SHALL leave it.

Records in both SHALL be left as they are. When the refresh is done, the main window SHALL say what changed ("Collection: 3 new, 1 gone", "Wantlist up to date"). While a refresh runs, its item SHALL read "Refreshing…" and be disabled. A refresh that fails SHALL change nothing and say why. Without a token, the items SHALL NOT be offered.

#### Scenario: Bought and sold on discogs.com
- **WHEN** the collection crate holds release 1001, the Discogs collection now holds 1003 instead, and the user chooses Refresh collection
- **THEN** release 1003's entries come into the crate, release 1001's leave it, and the main window says "Collection: 1 new, 1 gone"

#### Scenario: Removed from the wantlist elsewhere
- **WHEN** releases 1002 and 1006 were removed from the wantlist on discogs.com during the session, and the user chooses Refresh wantlist
- **THEN** their entries leave the wantlist crate, they are no longer wanted, and the main window says "Wantlist: 2 gone"

#### Scenario: Narrow playlist
- **WHEN** the playlist is 400 points wide and the user clicks its title bar
- **THEN** the crate menu offers Refresh wantlist and Refresh collection under its Discogs crates

#### Scenario: Offline
- **WHEN** Discogs is unreachable and the user chooses Refresh collection
- **THEN** the crate is unchanged, and the main window says the refresh failed because Discogs is offline

### Requirement: Connect to Discogs
Add to wantlist without a Discogs token SHALL still mark the record wanted and add it to the wantlist crate. It SHALL then open a dialog saying, in substance, that if the user has a collection on Discogs they can keep it up to date from here, and that it only takes connecting their account. The dialog SHALL give the steps: open the Discogs developer settings, with a link that opens `https://www.discogs.com/settings/developers` in the default browser; generate a token and copy it; paste it in Options ▸ Discogs…. It SHALL offer Connect…, which opens Options ▸ Discogs… with the token field focused, Later, and "Don't show this again". After "Don't show this again", Add to wantlist without a token SHALL show a one-line message in the main window instead. Add to collection without a token SHALL open the dialog every time, without that option. While the dialog is open, no other shortcut SHALL act.

#### Scenario: First time without a token
- **WHEN** the user presses Y on a Discogs track with no token set
- **THEN** the record is in the wantlist crate, and the dialog explains how to connect, with a link to the Discogs developer settings

#### Scenario: Not again
- **WHEN** the user ticked "Don't show this again" and presses Y on another track
- **THEN** the record is added to the wantlist crate, no dialog opens, and the main window says a Discogs token (Options ▸ Discogs…) would also add it to the Discogs wantlist

#### Scenario: Connect from the dialog
- **WHEN** the user clicks Connect…
- **THEN** Options ▸ Discogs… opens with the token field focused

### Requirement: Wantlist changes end
A wantlist change that Discogs can't take SHALL be kept and retried:
- while Discogs is unreachable, it SHALL wait without limit and be sent when Discogs answers again;
- after a server error, it SHALL be retried 1, 2, 5, 15 and 60 minutes later. When the last retry fails too, the entry SHALL show "wantlist failed" in its tooltip with a ⚑ in its row, the main window SHALL show the error once, and the menu SHALL offer Retry wantlist, which tries again at once and restarts the schedule.

While a change waits, the entry SHALL show "wantlist change pending". Waiting changes SHALL survive restarts. An add and a removal of the same release that are both waiting SHALL cancel out. A rejected token or an unknown release SHALL drop the change with a message.

#### Scenario: Offline for an hour
- **WHEN** the user adds a record to the wantlist while Discogs is unreachable for an hour
- **THEN** the entry shows "wantlist change pending", and the release is added within one minute of Discogs answering again

#### Scenario: Server keeps failing
- **WHEN** Discogs answers every attempt with a server error
- **THEN** the change is tried 6 times in about 83 minutes, then the entry shows "wantlist failed", the main window shows the error, and no further request is sent until the user chooses Retry wantlist

### Requirement: Writes never touch playback
Wantlist and collection changes SHALL be sent by the background Discogs worker, within the existing rate limit, and never before the window is interactive. Playback SHALL have zero underruns while they run, and every menu item SHALL update the UI within one frame without waiting for Discogs.

#### Scenario: Adding while playing
- **WHEN** the user adds 20 selected records to the wantlist while a track plays
- **THEN** playback has zero underruns, and no 60-second window holds more than 60 requests to Discogs

