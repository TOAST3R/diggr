## Purpose

One-key decisions while digging: keep a record (collected in a Keepers crate and on the user's Discogs wantlist), pass on it (it isn't offered again), or open its for-sale page on Discogs.

## ADDED Requirements

### Requirement: Verdict keys
`Y` (keep), `N` (pass) and `I` (open the for-sale page) SHALL act on the playing or paused entry, and SHALL do nothing while playback is stopped. The entry right-click menu SHALL offer the same actions for the entry clicked: Keep or Undo keep, Pass or Undo pass, and Open for-sale page.

#### Scenario: Stopped
- **WHEN** playback is stopped and the user presses Y
- **THEN** nothing is kept and nothing changes

#### Scenario: Keep after listening
- **WHEN** the user right-clicks a track played earlier and chooses Keep
- **THEN** it is kept exactly as if Y had been pressed while it played

### Requirement: Keep
Keeping an entry SHALL:
- copy it to the Keepers crate, which is created when first needed;
- mark it kept (✓) wherever it appears;
- when it comes from a Discogs release and a token is set, add that release to the user's Discogs wantlist.

Keeping an entry that is already kept SHALL undo the keep: the entry leaves the Keepers crate, and its release is removed from the wantlist only if this app added it there. When the wantlist can't be reached, the change SHALL be retried later, and the entry SHALL show "wantlist pending" until it succeeds. Without a token, the entry SHALL be kept in the Keepers crate, and the main window SHALL say that a token is needed for the wantlist.

#### Scenario: Keep a track
- **WHEN** the user presses Y while "Nightcraft - Glasshouse" from release 123456 plays
- **THEN** the track is added to the end of the Keepers crate with its origin, its row shows ✓, and release 123456 is on the user's Discogs wantlist

#### Scenario: Undo
- **WHEN** the user presses Y again on that track
- **THEN** it leaves the Keepers crate, loses its ✓, and release 123456 is removed from the wantlist, because the app had added it

#### Scenario: Already on the wantlist
- **WHEN** the user keeps and then un-keeps a track whose release was on the wantlist before
- **THEN** the release stays on the wantlist

#### Scenario: Offline keep
- **WHEN** the user keeps a track while Discogs is unreachable
- **THEN** the track is in the Keepers crate with "wantlist pending", and its release is added to the wantlist once Discogs answers again

### Requirement: Pass
Passing on an entry SHALL remember its clip as passed and dim its row; when it is the playing entry, the next track SHALL start. Passed clips SHALL be left out of later sends while skip passed is on. Passing on a kept entry SHALL do nothing, and the main window SHALL say that it is kept. Undo pass SHALL forget the pass.

#### Scenario: Pass and move on
- **WHEN** the user presses N while track 5 plays
- **THEN** track 5 is dimmed and remembered as passed, and track 6 starts

#### Scenario: Passed stays passed
- **WHEN** the user sends the same label again a month later, with skip passed on
- **THEN** the clip passed on in track 5 is not added

### Requirement: Open for-sale page
Opening the for-sale page SHALL open the entry's release on the Discogs marketplace, the page that lists its copies for sale, in the default browser. For an entry that isn't from Discogs, the main window SHALL say so.

#### Scenario: Buy page
- **WHEN** the user presses I while a track from release 123456 plays
- **THEN** the default browser opens https://www.discogs.com/sell/release/123456

### Requirement: Dig memory
Kept and passed clips, and the releases this app added to the wantlist, SHALL be remembered in the config folder across restarts.

#### Scenario: After a restart
- **WHEN** the app is restarted
- **THEN** kept tracks still show ✓, passed tracks are still dimmed, and un-keeping still removes only releases the app added to the wantlist
