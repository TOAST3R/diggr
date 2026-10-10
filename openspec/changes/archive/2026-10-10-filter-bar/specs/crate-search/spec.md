## ADDED Requirements

### Requirement: Search matches words
The filter bar's search field SHALL narrow the shown crate to the entries matching its text. The text SHALL be split on whitespace into words; an entry SHALL match when every word appears, in any order, in at least one of its artist, title, record title, record artist, label or catalogue number. Matching SHALL ignore case and accents the way the skin font folds them ("Âme" and "ame" are the same). An empty search SHALL show every entry. A field an entry doesn't have SHALL match nothing.

#### Scenario: One word
- **WHEN** a seller crate holds records by Theo Parrish (3), Nightcraft (2) and others, and the user types "parrish"
- **THEN** only the 3 Theo Parrish records are shown

#### Scenario: Words in any field
- **WHEN** the user types "lowtide 012"
- **THEN** the record on Lowtide Tapes with catalogue number LT-012 is shown, and a Lowtide Tapes record numbered LT-007 is not

#### Scenario: Accents and case
- **WHEN** a crate holds a record by "Âme" and the user types "AME"
- **THEN** the Âme record is shown

#### Scenario: Local files
- **WHEN** a crate of local files is shown and the user types a word from a file's album tag
- **THEN** that file is shown

### Requirement: Search is live and combines with the filters
Every change to the search text SHALL update the list, the title bar's shown/all count, what plays next, shuffle, the pre-warmed track and the previews downloaded ahead within one frame (16 ms) for a crate of 5,000 entries, SHALL NOT interrupt playback, and SHALL cause zero underruns. An entry SHALL be shown when it matches the search and passes every filter that is set (see `record-filters` "Filters combine"). The playing entry SHALL keep playing if the search hides it.

#### Scenario: Typing
- **WHEN** a track plays in a grouped collection crate of 5,000 entries and the user types "deep" one letter at a time
- **THEN** after each letter the list shows the matching entries within 16 ms, and playback has zero underruns

#### Scenario: Next under a search
- **WHEN** the search is "parrish" and a Theo Parrish track ends, followed in crate order by a Nightcraft record and then a Theo Parrish record
- **THEN** the next Theo Parrish record's first track plays next

#### Scenario: With a BPM range
- **WHEN** the BPM range is 120–125 and the search is "parrish"
- **THEN** only Theo Parrish entries at 120 to 125 BPM are shown, and the title bar shows the shown and total counts

### Requirement: Matches are highlighted
In every shown row (entry rows and record rows), the characters matching a search word SHALL be drawn in the playlist's current-entry colour instead of the row's text colour. With an empty search no row SHALL be highlighted.

#### Scenario: Highlight
- **WHEN** the search is "parrish" and the row "THEO PARRISH: SUMMERTIME IS HERE" is shown
- **THEN** "PARRISH" in that row is drawn in the current-entry colour, and the rest in the row's colour

### Requirement: Search field keys
While the search field has the keyboard, typed characters SHALL go only into the field: no player, playlist or verdict shortcut SHALL act. Enter SHALL play the first shown entry; ↓ SHALL give the keyboard to the list, with the cursor on the first shown entry; Esc SHALL clear the search and give the keyboard back to the player. Cmd+F (Ctrl+F off macOS) SHALL give the field the keyboard from anywhere in the main window. The help (H) SHALL list Cmd+F.

#### Scenario: Shortcuts don't fire
- **WHEN** a track plays and the user types "xcvfni" in the search field
- **THEN** the field reads "XCVFNI", the track is still playing, fullscreen is off, no record is passed and no browser page is opened

#### Scenario: Enter
- **WHEN** the search is "parrish" and the user presses Enter
- **THEN** the first shown Theo Parrish entry plays

#### Scenario: Esc
- **WHEN** the search is "parrish", a label is picked, and the user presses Esc in the field
- **THEN** the search is empty, the label filter is still set, and X plays again

#### Scenario: Cmd+F
- **WHEN** the list has the keyboard and the user presses Cmd+F on macOS
- **THEN** the search field has the keyboard

### Requirement: Search is not remembered
The search SHALL belong to the shown crate only while it is shown: showing another crate SHALL clear it, and it SHALL NOT be saved across restarts. Pressing P while the search (or any filter) hides the playing entry SHALL clear the search together with the filters.

#### Scenario: Switch crates
- **WHEN** the search is "parrish" in "Seller: decks.de" and the user shows the collection crate
- **THEN** the search field is empty and the collection crate is not narrowed by it

#### Scenario: Restart
- **WHEN** the search is "parrish" and the app is restarted
- **THEN** the search field is empty

#### Scenario: P
- **WHEN** the search hides the playing entry and the user presses P
- **THEN** the search is cleared, every filter is off, and the list scrolls to the playing entry
