## MODIFIED Requirements

### Requirement: Entry context menu
Right-clicking an entry (or Control-clicking it on macOS) SHALL open a menu with:
- Play, or Arm when the entry is waiting for its audio;
- Remove;
- Remove album (N tracks) and Select album, when the entry belongs to an album;
- Send to crate;
- for an entry from Discogs: Add to wantlist or Remove from wantlist (or "In collection", disabled, when the record is owned), Add to collection (or "In collection", disabled, when this pressing is owned), Retry wantlist or Retry add to collection after a failure, Pass or Undo pass, Open for-sale page, Open release on Discogs, and Copy Discogs link.

When the clicked entry is selected, Remove, Send to crate, Add to wantlist, Remove from wantlist and Add to collection SHALL act on the whole selection (the wantlist and collection items on its distinct releases, see `discogs-write`). Otherwise the selection SHALL first become the clicked entry. Remove album and Select album SHALL act on the clicked entry's album. All other items SHALL act on the clicked entry only.

#### Scenario: Remove a selection
- **WHEN** entries 3 to 6 are selected and the user right-clicks entry 4 and chooses Remove
- **THEN** entries 3 to 6 are removed

#### Scenario: Right-click outside the selection
- **WHEN** entries 3 to 6 are selected and the user right-clicks entry 10 and chooses Remove
- **THEN** only entry 10 is removed

#### Scenario: Open the release
- **WHEN** the user chooses Open release on Discogs on an entry from release 123456
- **THEN** the default browser opens https://www.discogs.com/release/123456

#### Scenario: Arm from the menu
- **WHEN** the user chooses Arm on an entry whose preview is downloading
- **THEN** the entry is armed exactly as by a double-click

#### Scenario: Album items
- **WHEN** the user right-clicks an entry of a release with 4 entries in the crate
- **THEN** the menu offers "Remove album (4 tracks)" and "Select album" after Remove

#### Scenario: Discogs items
- **WHEN** the user right-clicks an entry from release 123456, which is neither wanted nor owned
- **THEN** the menu offers "Add to wantlist (Y)" and "Add to collection", and no Keep item

#### Scenario: No Render show
- **WHEN** the user right-clicks any entry, in any crate
- **THEN** the menu offers no Render show item
