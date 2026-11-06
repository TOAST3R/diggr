# Proposal

## Why

The playlist footer is 38 skin pixels that hold only a gear and a time readout, and the band's
controls are drawn in four different button styles in three places. Moving each control to the
place it belongs gives the list about three more rows and makes the window read as one system.
Diggers think in records, yet only Discogs, seller and label crates open grouped; every other
crate starts as a flat list of tracks.

## What Changes

- The ▤ (group by record) button moves from the playlist title bar into the mini player's LCD
  row, lit while the shown crate is grouped. Its tooltip loses the "(Shift+G)" it never had.
- **BREAKING**: the waveform is always shown. Its button, the `W` key and the seek bar that
  stood in for it go.
- The mini player's rows are realigned. The LCD row ends in ▤, EQ, spectrum and SYNC (SYNC
  beside the spectrum button). The button row is transport, a wider volume slider (108 px),
  shuffle and repeat, on one centre line with even gaps, ending where the LCD row ends.
- The Options gear moves from the playlist footer into the band's window corner, between
  minimize and close (`[_][⚙][×]`), drawn at the window buttons' size. The corner widens to hold
  it and the waveform gives up the difference.
- The "selected/total" time readout moves from the footer into the filter bar, right after the
  search field, in a sunken LCD box like the search's. As the bar narrows it shows the total
  only, then nothing.
- **BREAKING** (layout): the playlist footer is removed. A 12 px edge like the left one closes
  the frame under the last row, and the list takes the rest of the footer's height.
- Every crate that has never been toggled opens grouped by record, existing ones included
  (crates the user toggled keep their choice).
- While grouped, the column header still shows when the crate holds no album, so a crate of
  loose tracks (the default Playlist, singles) keeps its header and click-to-sort.
- Spec text that still places the BPM range and style chips in the footer is corrected.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `playlist`: the "Playlist footer" requirement is removed; "Title bar buttons" leaves the title
  bar without buttons; "Playlist placement and size" counts the frame with a bottom edge and no
  footer.
- `player-window`: the mini player's rows (▤ and SYNC in the LCD row, no waveform button, a
  wider volume); the Options menu's gear is in the window corner; the band's corner holds
  minimize, the gear and close; no `W` shortcut.
- `waveform-view`: the waveform is always shown; no toggle and no seek bar in its place.
- `record-view`: ▤ is in the mini player; crates never toggled are grouped; the column header
  shows for a grouped crate with no album.
- `filter-bar`: the bar holds the time readout after the search, and it gives way as it narrows.
- `classic-skin`: the LCD-colour list names the filter bar's time readout instead of the footer.

## Impact

- `crates/ui/src/app.rs`: `mini_player`, `window_buttons` (and `CORNER_W`), `filter_bar`,
  `playlist_section` (title bar and footer), `pl_header`.
- `crates/ui/src/layout.rs`: `playlist_chrome`, `playlist_def`.
- `crates/ui/src/crates.rs`: `CrateInfo::is_grouped`.
- `crates/ui/src/skin/generate.rs` and `assets/skin/default/`: new `tog_group` and gear sprites,
  `pl_bottom_h` 0, `pl_group`/`pl_opts`/`pl_info` layout entries dropped; regenerate with
  `cargo run -p ui --bin skin-gen`.
- Tests: headless tests for the footer, mini player buttons and grouping; layout tests; crates
  default-grouping tests.
- Docs: README (layout description, test count), CHANGELOG `## Unreleased`; the Purpose lines of
  `playlist-filters` and `style-filter`.
