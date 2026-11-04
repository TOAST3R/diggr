## MODIFIED Requirements

### Requirement: Connect to Discogs
Add to wantlist without a Discogs token SHALL still mark the record wanted and add it to the wantlist crate. It SHALL then open a dialog saying, in substance, that if the user has a collection on Discogs they can keep it up to date from here, and that it only takes connecting their account. The dialog SHALL give the steps: open the Discogs developer settings, with a link that opens `https://www.discogs.com/settings/developers` in the default browser; generate a token and copy it; paste it in Options ▸ Discogs…. It SHALL offer Connect…, which opens Options ▸ Discogs… with the token field focused, Later, and "Don't show this again". After "Don't show this again", or while the Discogs setup step is Declined (see `welcome-setup`), Add to wantlist without a token SHALL show a one-line message in the main window instead. Add to collection without a token SHALL open the dialog every time, without that option, unless the Discogs setup step is Declined, in which case it SHALL show the one-line message. While the dialog is open, no other shortcut SHALL act.

#### Scenario: First time without a token
- **WHEN** the user presses Y on a Discogs track with no token set
- **THEN** the record is in the wantlist crate, and the dialog explains how to connect, with a link to the Discogs developer settings

#### Scenario: Not again
- **WHEN** the user ticked "Don't show this again" and presses Y on another track
- **THEN** the record is added to the wantlist crate, no dialog opens, and the main window says a Discogs token (Options ▸ Discogs…) would also add it to the Discogs wantlist

#### Scenario: No account
- **WHEN** the user declined the Discogs setup step and presses Y on a track
- **THEN** the record is added to the wantlist crate, no dialog opens, and the main window shows the one-line message

#### Scenario: Connect from the dialog
- **WHEN** the user clicks Connect…
- **THEN** Options ▸ Discogs… opens with the token field focused
