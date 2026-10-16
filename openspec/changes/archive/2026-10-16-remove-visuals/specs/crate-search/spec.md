## MODIFIED Requirements

### Requirement: Search field keys
While the search field has the keyboard, typed characters SHALL go only into the field: no player, playlist or verdict shortcut SHALL act. Enter SHALL play the first shown entry; ↓ SHALL give the keyboard to the list, with the cursor on the first shown entry; Esc SHALL clear the search and give the keyboard back to the player. Cmd+F (Ctrl+F off macOS) SHALL give the field the keyboard from anywhere in the main window. The help (H) SHALL list Cmd+F.

#### Scenario: Shortcuts don't fire
- **WHEN** a track plays and the user types "xcvfni" in the search field
- **THEN** the field reads "XCVFNI", the track is still playing, no record is passed and no browser page is opened

#### Scenario: Enter
- **WHEN** the search is "parrish" and the user presses Enter
- **THEN** the first shown Theo Parrish entry plays

#### Scenario: Esc
- **WHEN** the search is "parrish", a label is picked, and the user presses Esc in the field
- **THEN** the search is empty, the label filter is still set, and X plays again

#### Scenario: Cmd+F
- **WHEN** the list has the keyboard and the user presses Cmd+F on macOS
- **THEN** the search field has the keyboard
