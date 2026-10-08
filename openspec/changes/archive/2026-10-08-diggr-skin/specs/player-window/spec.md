## ADDED Requirements

### Requirement: Verdict flash
When a verdict changes at least one record, the title line SHALL show, for 1.5 s and without scrolling, centred: "WANTED" after adding to the wantlist, "UNWANTED" after removing from it, "PASS" after a pass and "OWNED" after adding to the collection. When more than one record changed, the count SHALL follow ("WANTED 3"). A verdict that changes nothing (Y on an owned record, N on a wanted one, a key while stopped) SHALL show no flash. A newer flash SHALL replace an older one, and when the flash ends the normal title line SHALL come back.

#### Scenario: Want
- **WHEN** the user presses Y while a record not on the wantlist plays
- **THEN** the title line shows "WANTED" for 1.5 s, then the track line again

#### Scenario: Pass and next track
- **WHEN** the user presses N while track 5 plays
- **THEN** the title line shows "PASS" for 1.5 s while track 6 starts, then shows track 6's line

#### Scenario: Several records
- **WHEN** the user adds 3 selected records to the wantlist from the right-click menu
- **THEN** the title line shows "WANTED 3"

#### Scenario: Nothing changed
- **WHEN** the user presses N on a track whose record is on the wantlist
- **THEN** no flash is shown
