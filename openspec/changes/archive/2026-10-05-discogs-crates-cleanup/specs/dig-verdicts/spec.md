## MODIFIED Requirements

### Requirement: Verdict keys
`Y` (add to wantlist, or remove from it), `N` (pass) and `I` (open the for-sale page) SHALL act on the playing or paused entry, and SHALL do nothing while playback is stopped. The entry right-click menu SHALL offer the same actions for the entry clicked: Add to wantlist or Remove from wantlist (see `discogs-write`), Pass or Undo pass, and Open for-sale page. In the wantlist crate and the collection crate, the menu SHALL NOT offer Pass or Undo pass.

#### Scenario: Stopped
- **WHEN** playback is stopped and the user presses Y
- **THEN** nothing is added to the wantlist and nothing changes

#### Scenario: Add after listening
- **WHEN** the user right-clicks a track played earlier and chooses Add to wantlist
- **THEN** its record is added exactly as if Y had been pressed while it played

#### Scenario: No Pass in the Discogs crates
- **WHEN** the user right-clicks an entry in the wantlist crate
- **THEN** the menu offers neither Pass nor Undo pass
