## MODIFIED Requirements

### Requirement: Keep
Keeping an entry SHALL:
- copy it to the Keepers crate, which is created when first needed;
- mark it kept (✓) wherever it appears;
- when it comes from a Discogs release and a token is set, add that release to the user's Discogs wantlist.

When the entry is owned (see `discogs-collection`) and a token is set, keeping it SHALL first ask, in a dialog that names the owned pressing, whether to keep it anyway. Keep anyway SHALL keep it as above, and Cancel SHALL change nothing. While the dialog is open, no other shortcut SHALL act.

Keeping an entry that is already kept SHALL undo the keep, without asking: the entry leaves the Keepers crate, and its release is removed from the wantlist only if this app added it there. When the wantlist can't be reached, the change SHALL be retried later, and the entry SHALL show "wantlist pending" until it succeeds. Without a token, the entry SHALL be kept in the Keepers crate, and the main window SHALL say that a token is needed for the wantlist.

#### Scenario: Keep a track
- **WHEN** the user presses Y while "Nightcraft - Glasshouse" from release 123456 plays
- **THEN** the track is added to the end of the Keepers crate with its origin, its row shows ✓, and release 123456 is on the user's Discogs wantlist

#### Scenario: Keep an owned record
- **WHEN** the user presses Y on a track whose record is owned in another pressing (AF001R, 2019)
- **THEN** a dialog says the record is already owned (AF001R, 2019); Cancel leaves the track un-kept and the wantlist unchanged, and Keep anyway keeps it and adds its release to the wantlist

#### Scenario: Undo
- **WHEN** the user presses Y again on that track
- **THEN** it leaves the Keepers crate, loses its ✓, and release 123456 is removed from the wantlist, because the app had added it

#### Scenario: Already on the wantlist
- **WHEN** the user keeps and then un-keeps a track whose release was on the wantlist before
- **THEN** the release stays on the wantlist

#### Scenario: Offline keep
- **WHEN** the user keeps a track while Discogs is unreachable
- **THEN** the track is in the Keepers crate with "wantlist pending", and its release is added to the wantlist once Discogs answers again
