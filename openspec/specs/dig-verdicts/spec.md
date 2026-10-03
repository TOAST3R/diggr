# dig-verdicts Specification

## Purpose
One-key decisions while digging: keep a record (collected in a Keepers crate and on the user's Discogs wantlist), pass on it (it isn't offered again), or open its for-sale page on Discogs.
## Requirements
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

### Requirement: Open for-sale page
Opening the for-sale page SHALL open the entry's release on the Discogs marketplace, the page that lists its copies for sale, in the default browser. For an entry that isn't from Discogs, the main window SHALL say so.

#### Scenario: Buy page
- **WHEN** the user presses I while a track from release 123456 plays
- **THEN** the default browser opens https://www.discogs.com/sell/release/123456

### Requirement: Dig memory
Passed clips, wanted releases and wantlist changes still waiting SHALL be remembered in the config folder across restarts. Memory written before this change SHALL load: every release kept before SHALL be wanted.

#### Scenario: After a restart
- **WHEN** the app is restarted
- **THEN** wanted records still show ★, passed tracks are still dimmed, and waiting wantlist changes are still retried

#### Scenario: Kept before the upgrade
- **WHEN** the app starts with a memory file in which release 123456 was kept
- **THEN** release 123456 is wanted, and its entries show ★

