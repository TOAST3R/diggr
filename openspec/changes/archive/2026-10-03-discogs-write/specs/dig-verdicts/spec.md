## MODIFIED Requirements

### Requirement: Verdict keys
`Y` (add to wantlist, or remove from it), `N` (pass) and `I` (open the for-sale page) SHALL act on the playing or paused entry, and SHALL do nothing while playback is stopped. The entry right-click menu SHALL offer the same actions for the entry clicked: Add to wantlist or Remove from wantlist (see `discogs-write`), Pass or Undo pass, and Open for-sale page.

#### Scenario: Stopped
- **WHEN** playback is stopped and the user presses Y
- **THEN** nothing is added to the wantlist and nothing changes

#### Scenario: Add after listening
- **WHEN** the user right-clicks a track played earlier and chooses Add to wantlist
- **THEN** its record is added exactly as if Y had been pressed while it played

### Requirement: Pass
Passing on an entry SHALL remember its clip as passed and dim its row; when it is the playing entry, the next track SHALL start. Passed clips SHALL be left out of later sends while skip passed is on. Passing on an entry whose record is wanted SHALL do nothing, and the main window SHALL say that it is on the wantlist. Undo pass SHALL forget the pass.

#### Scenario: Pass and move on
- **WHEN** the user presses N while track 5 plays
- **THEN** track 5 is dimmed and remembered as passed, and track 6 starts

#### Scenario: Passed stays passed
- **WHEN** the user sends the same label again a month later, with skip passed on
- **THEN** the clip passed on in track 5 is not added

#### Scenario: Wanted is not passed
- **WHEN** the user presses N on a track whose record is on the wantlist
- **THEN** nothing is passed, and the main window says the record is on the wantlist

### Requirement: Dig memory
Passed clips, wanted releases and wantlist changes still waiting SHALL be remembered in the config folder across restarts. Memory written before this change SHALL load: every release kept before SHALL be wanted.

#### Scenario: After a restart
- **WHEN** the app is restarted
- **THEN** wanted records still show ✓, passed tracks are still dimmed, and waiting wantlist changes are still retried

#### Scenario: Kept before the upgrade
- **WHEN** the app starts with a memory file in which release 123456 was kept
- **THEN** release 123456 is wanted, and its entries show ✓

## REMOVED Requirements

### Requirement: Keep
**Reason**: Keep is replaced by Add to wantlist, which acts on whole records and keeps the wantlist crate in step with the Discogs wantlist. Owning a record now blocks it, with no "keep anyway" dialog. Removing from the wantlist no longer depends on who added the release.
**Migration**: See `discogs-write` (Add to wantlist, Never want what you own, The wantlist crate mirrors Discogs, Wantlist changes end). The Keepers crate becomes the wantlist crate, and kept releases become wanted.
