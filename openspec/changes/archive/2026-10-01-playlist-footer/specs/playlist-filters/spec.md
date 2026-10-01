## RENAMED Requirements

- FROM: `### Requirement: BPM filter bar`
- TO: `### Requirement: BPM filter control`

## MODIFIED Requirements

### Requirement: BPM filter control
When the shown crate has at least two different known tempos, the playlist footer SHALL show a BPM filter control between its buttons and its time readout, with:
- the label "BPM", when the footer has room for it;
- a range slider with two handles, snapping to whole BPM, spanning the crate's lowest to highest known tempo, 40 skin pixels wide at any playlist width;
- the range as text in the skin's font ("124-139");
- a × button, shown only while a range is set, that clears it.

The control SHALL stay clear of the time readout's box.

The control SHALL NOT take a row of the list. An entry SHALL be shown when its BPM is inside the range. An entry without a BPM SHALL be shown only while the range is the whole span. Clearing the range (×, a double-click on the slider, or ≡ ▸ Show all tempos) SHALL set it back to the whole span. The range SHALL be remembered per crate, across restarts, and widened back to the whole span when the crate's tempos no longer reach it. While the range is narrower than the whole span, the playlist title bar SHALL show the number of shown entries and of all entries ("LOWTIDE TAPES · 2/4"), and the slider's tooltip SHALL say how many entries without a BPM are hidden when there are any. When no entry matches, the list SHALL say so and how to clear the range. Changing the range SHALL update the list within one frame (16 ms) for a crate of 1,000 entries.

#### Scenario: A range
- **WHEN** a crate holds tracks at 124, 128, 137 and 139 BPM, and the user drags the range to 130–140
- **THEN** only the 137 and 139 BPM tracks are shown, numbered 3 and 4, and the title bar reads "‹NAME› · 2/4"

#### Scenario: No row taken
- **WHEN** a crate with known tempos is shown
- **THEN** the BPM control is in the footer, and the list's first row is the first entry (or the column header)

#### Scenario: Unknown tempos
- **WHEN** a crate holds tracks at 124 and 139 BPM and three without a BPM, and the range is 130–140
- **THEN** only the 139 BPM track is shown, and the slider's tooltip says 3 entries without a BPM are hidden

#### Scenario: Clear
- **WHEN** the user clicks × after the range text
- **THEN** every entry is shown again, × disappears, and the title bar shows the name only

#### Scenario: Classic width
- **WHEN** the playlist is 275 skin pixels wide and the crate's time readout is "0:00/70:12:33+"
- **THEN** the slider is 40 px wide, the range text and × follow it, and none of it overlaps the time's box

#### Scenario: Wide playlist
- **WHEN** the playlist is 700 skin pixels wide
- **THEN** the slider is still 40 px wide, with "BPM" in front of it

#### Scenario: Remembered
- **WHEN** a crate is filtered to 130–140 BPM and the app is restarted
- **THEN** showing that crate shows it filtered to 130–140 BPM

#### Scenario: Not enough tempos
- **WHEN** the shown crate has no known tempo, or only one
- **THEN** the footer shows no BPM control
