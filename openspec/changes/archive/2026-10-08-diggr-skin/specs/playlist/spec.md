## ADDED Requirements

### Requirement: Empty crate hint
When the shown crate has no entries and is not a Discogs crate (wantlist or collection), its list SHALL show, centred in the list area under the column header, "PASTE A DISCOGS LINK · CMD+V" over "OR DROP FILES", then after a blank line "PRESS H FOR HELP", in the playlist text colour, with CTRL+V instead of CMD+V off macOS. It SHALL show in the normal and maximized layouts, SHALL NOT move, and SHALL go away as soon as the crate has an entry.

#### Scenario: New crate
- **WHEN** the user creates the crate "asdf" on macOS and it is shown
- **THEN** its list shows "PASTE A DISCOGS LINK · CMD+V", "OR DROP FILES" and, after a gap, "PRESS H FOR HELP", centred

#### Scenario: Maximized
- **WHEN** the playlist is maximized and the shown crate is empty
- **THEN** the hint shows in the middle of the list

#### Scenario: Filled
- **WHEN** a Discogs link is pasted into the empty crate and its first entry arrives
- **THEN** the hint is gone

#### Scenario: Empty wantlist crate
- **WHEN** the wantlist crate is shown and empty
- **THEN** no hint is shown, since a paste can't fill it
