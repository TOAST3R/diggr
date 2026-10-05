## MODIFIED Requirements

### Requirement: Acting on a record
In a grouped crate, a record row SHALL act on all of its entries:
- clicking a record row SHALL select all its entries. Shift-click and Cmd- or Ctrl-click SHALL extend or toggle the selection by whole records;
- right-clicking a record row SHALL open the entry menu with the record's entries selected, so that Remove, Send to crate and the Discogs items act on the whole record. Remove album and Select album SHALL NOT be offered there. In the wantlist crate and the collection crate, Remove SHALL NOT be offered either (see `discogs-write`);
- double-clicking a record row, or pressing Enter on it, SHALL play its first playable track, or arm it when none is playable yet;
- dragging a record row SHALL move all its entries together, between other records. Dropping it on a crate in the sidebar SHALL send them all, unless that crate takes no hand edits;
- dragging a track row SHALL reorder it only within its record, and no insertion line SHALL be drawn outside it.

#### Scenario: Remove a record
- **WHEN** the user right-clicks a record row of 3 tracks in a dig crate and chooses Remove
- **THEN** its 3 entries are removed

#### Scenario: No Remove in the collection crate
- **WHEN** the user right-clicks a record row in the collection crate
- **THEN** the menu offers no Remove

#### Scenario: Play a record
- **WHEN** the user double-clicks a record row whose first track is still downloading and whose second is playable
- **THEN** the second track plays

#### Scenario: Move a record
- **WHEN** the user drags the third record row above the first
- **THEN** all of that record's entries come first in the crate, in their order
