## ADDED Requirements

### Requirement: BPM filter bar
When the shown crate has at least two different known tempos, a filter bar SHALL show above the list, with:
- an ALL button;
- a BPM range slider with two handles, snapping to whole BPM, spanning the crate's lowest to highest known tempo, with the range shown as text in the skin's font ("124-140 BPM").

An entry SHALL be shown when its BPM is inside the range. An entry without a BPM SHALL be shown only while the range is the whole span. ALL SHALL set the range back to the whole span. The range SHALL be remembered per crate, across restarts, and widened back to the whole span when the crate's tempos no longer reach it. While the range is narrower than the whole span, the playlist title bar SHALL show the number of shown entries and of all entries ("LOWTIDE TAPES · 2/4"), and the bar SHALL say how many entries have no BPM ("+3 without BPM") when there are any: on the bar when the playlist is wide enough for it, and always in the slider's tooltip. When no entry matches, the list SHALL say so and offer ALL. Changing the range SHALL update the list within one frame (16 ms) for a crate of 1,000 entries.

#### Scenario: A range
- **WHEN** a crate holds tracks at 124, 128, 137 and 139 BPM, and the user drags the range to 130–140
- **THEN** only the 137 and 139 BPM tracks are shown, numbered 3 and 4, and the title bar reads "‹NAME› · 2/4"

#### Scenario: Unknown tempos
- **WHEN** a crate holds tracks at 124 and 139 BPM and three without a BPM, and the range is 130–140
- **THEN** only the 139 BPM track is shown, and the slider's tooltip says 3 entries without a BPM are hidden (the bar shows "+3 without BPM" too when the playlist is wide)

#### Scenario: ALL
- **WHEN** the user clicks ALL
- **THEN** every entry is shown again, and the title bar shows the name only

#### Scenario: Remembered
- **WHEN** a crate is filtered to 130–140 BPM and the app is restarted
- **THEN** showing that crate shows it filtered to 130–140 BPM

#### Scenario: Not enough tempos
- **WHEN** the shown crate has no known tempo, or only one
- **THEN** no filter bar is shown

### Requirement: What plays under a filter
Next, previous, shuffle, the pre-warmed next track and the previews downloaded ahead SHALL use only the entries the filter shows. The playing track SHALL finish even if the filter hides it, and SHALL be followed by the next shown entry after it in crate order. Changing the filter SHALL NOT interrupt playback and SHALL cause zero underruns. Keyboard navigation, the cursor, Select all and Invert selection SHALL work over the shown entries. Entries SHALL keep their crate numbers. Pressing P while the filter hides the playing entry SHALL turn the filter off and show the playing entry. Sorting and M3U export SHALL act on the whole crate.

#### Scenario: Next under a filter
- **WHEN** the range is 130–140, entry 12 (134 BPM) plays, entries 13 and 14 are at 124 BPM and entry 15 is at 138 BPM
- **THEN** entry 15 plays after entry 12

#### Scenario: Hidden while playing
- **WHEN** a range change hides the playing entry
- **THEN** it keeps playing without interruption, and the next shown entry after it plays next

#### Scenario: P on a hidden entry
- **WHEN** the filter hides the playing entry and the user presses P
- **THEN** the filter is turned off, and the list scrolls to the playing entry

#### Scenario: Previews follow the filter
- **WHEN** a dig crate is filtered to 130–140 BPM while entry 5 plays
- **THEN** the previews downloaded ahead are those of the next shown entries, not of hidden ones

#### Scenario: Export ignores the filter
- **WHEN** a filtered crate of 301 entries showing 42 is exported as M3U8
- **THEN** the file lists all 301 entries
