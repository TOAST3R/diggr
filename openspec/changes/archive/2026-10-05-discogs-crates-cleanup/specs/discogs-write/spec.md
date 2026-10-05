## MODIFIED Requirements

### Requirement: The wantlist crate mirrors Discogs
The wantlist crate SHALL hold the records on the user's wantlist. Without a token, it SHALL be a crate named "Wantlist" among the user's own crates. When a token for ‹user› is saved:
- it SHALL become "Wantlist: ‹user›". When a crate of that name already exists, the local crate's entries SHALL be merged into it, and the local crate removed once the merge is saved;
- it SHALL be filled with the user's Discogs wantlist, in the same way as sending the wantlist page;
- every record in it that isn't on the Discogs wantlist and isn't owned SHALL be added to the Discogs wantlist.

Once per session, when the account is first known (a dig, a wantlist change, or the wantlist crate shown, with a token), it SHALL take in records newly wanted on Discogs, and drop entries of records no longer on the Discogs wantlist that have no change waiting. With a token, records SHALL leave the wantlist crate only through Remove from wantlist, Add to collection, a sync or a refresh (see "Discogs crates take no hand edits"). Without a token, the "Wantlist" crate is one of the user's own crates: removing every entry of a record from it SHALL make it no longer wanted, without asking. Deleting the crate SHALL NOT change the Discogs wantlist.

#### Scenario: Connecting
- **WHEN** the user has a "Wantlist" crate with 5 records, 1 of them owned, and saves a token for "digger", whose Discogs wantlist has 20 records
- **THEN** the crate is named "Wantlist: digger", holds the 20 records and the 4 not owned, and those 4 are added to the Discogs wantlist

#### Scenario: Removed on discogs.com
- **WHEN** a record was removed from the wantlist on discogs.com, and the app is started again with a token
- **THEN** that record is no longer wanted, and its entries leave the crate

#### Scenario: Remove from wantlist
- **WHEN** the user chooses Remove from wantlist (Y) on a record row of release 123456 in the wantlist crate
- **THEN** its entries leave the crate at once, without a dialog, and release 123456 is removed from the Discogs wantlist

## ADDED Requirements

### Requirement: Discogs crates take no hand edits
With a Discogs token, the wantlist crate ("Wantlist: ‹user›") and the collection crate ("Collection: ‹user›") SHALL change only through Discogs-aware actions. In them:
- the entry menu SHALL NOT offer Remove, Remove album or Pass, on track rows or on record rows;
- the collection crate's entry menu SHALL NOT offer Add to wantlist, "In collection" or Add to collection;
- Delete and Backspace SHALL remove nothing, and the first time in a session the main window SHALL say that records leave the crate with Remove from wantlist or Remove from collection;
- entries SHALL NOT be added by hand: the crates SHALL NOT be listed under Send to crate, SHALL NOT take drops in the crate sidebar, and SHALL NOT take added files, addresses, pasted entries or a page sent to them. A refused drop or send SHALL say, in the main window, to use Add to wantlist or Add to collection.

Moving entries out of these crates (Send to crate, dragging a record onto another crate) and reordering them inside the crate SHALL still work. Add to wantlist, Add to collection, Remove from wantlist, Remove from collection, syncs, refreshes and connecting a token SHALL still change them.

#### Scenario: Track menu in the collection crate
- **WHEN** the user right-clicks a track row in the collection crate
- **THEN** the menu offers no Remove, Remove album, Pass, Add to wantlist or Add to collection

#### Scenario: Delete in the wantlist crate
- **WHEN** the user selects a record row in the wantlist crate and presses Delete
- **THEN** no entry leaves the crate, the Discogs wantlist is unchanged, and the main window says to use Remove from wantlist

#### Scenario: Drop on the collection crate
- **WHEN** the user drags a record from a dig crate onto the collection crate in the sidebar
- **THEN** nothing is added, and the main window says to use Add to collection

#### Scenario: Send to crate
- **WHEN** the user opens Send to crate on an entry of a dig crate
- **THEN** neither the wantlist crate nor the collection crate is listed

#### Scenario: Moving out
- **WHEN** the user sends a record of the wantlist crate to crate "Friday"
- **THEN** the record's entries are in "Friday" and still in the wantlist crate

### Requirement: Remove from collection
With a Discogs token, the collection crate's entry menu SHALL offer "Remove from collection…" on an entry or record row from a Discogs release, when the selection it acts on holds a single record. It SHALL NOT be offered for a selection of several records. Choosing it SHALL ask "Remove 1 copy of ‹artist – album› (‹catalog number›, ‹year›) from your Discogs collection?", saying that its notes and rating on Discogs are lost, and offer Remove (Enter) and Cancel (Esc). While the question is open, no other shortcut SHALL act. Remove SHALL take exactly one copy of the release, the most recently added one, out of the user's Discogs collection, with one request to find it and one to remove it. Cancel SHALL change nothing. When Discogs confirms the removal:
- the local collection SHALL drop that copy at once (see `discogs-collection`);
- when it was the last copy, the release's entries SHALL leave the collection crate, and SHALL lose OWNED in every crate unless another pressing of the same master release is owned;
- when copies remain, the entries SHALL stay, and their tooltip SHALL count the copies left.

When Discogs no longer has a copy of the release, the removal SHALL count as done, with no removal request. A removal that Discogs can't take SHALL be kept and retried as a wantlist change is (see "Wantlist changes end"), and after the last retry the entry SHALL show "collection removal failed" in its tooltip with a ⚑ in its row, and the menu SHALL offer Retry remove from collection. A removal still waiting SHALL survive restarts. It SHALL be sent by the background Discogs worker, within the existing rate limit, with zero underruns, and the menu and dialog SHALL respond within one frame.

#### Scenario: Sold it
- **WHEN** the user owns one copy of release 123456, chooses Remove from collection… on its record row in the collection crate, and confirms
- **THEN** release 123456 is no longer in the Discogs collection, its entries have left the collection crate, and no entry of release 123456 shows OWNED

#### Scenario: Two copies
- **WHEN** the user owns two copies of release 123456, added in 2021 and 2024, and removes one
- **THEN** the copy added in 2024 is removed, the copy added in 2021 stays, and the record stays in the collection crate, still OWNED

#### Scenario: Several records selected
- **WHEN** the user selects entries of two records in the collection crate and right-clicks one
- **THEN** the menu doesn't offer Remove from collection

#### Scenario: Cancel
- **WHEN** the user chooses Remove from collection… and presses Esc
- **THEN** no request is sent and nothing changes

#### Scenario: Already gone
- **WHEN** the user removed the copy on discogs.com, then removes it from the app before the next sync
- **THEN** no removal request is sent, and the record leaves the collection crate as if the removal had gone through

#### Scenario: Offline
- **WHEN** the user confirms a removal while Discogs is unreachable
- **THEN** the entry shows that the removal is pending, and the copy is removed within one minute of Discogs answering again
