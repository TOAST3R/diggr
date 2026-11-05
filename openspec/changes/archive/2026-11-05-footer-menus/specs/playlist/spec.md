## MODIFIED Requirements

### Requirement: Playlist footer
The playlist footer SHALL hold, from left to right: a gear button and the "selected/total" time readout, right-aligned in its LCD box beside the resize grip (only the total when both don't fit in the box). The footer SHALL hold no `+` button, no filter control (see `filter-bar`) and no `≡` button. Everything SHALL fit at the classic 275 skin pixel width. The gear SHALL open the Options menu (see the player window's Options menu), with the same items as the right-click.

No app setting SHALL be in the footer. The crate's own actions, adding music and M3U export included, SHALL be in the list menu (see "List menu").

#### Scenario: Three buttons
- **WHEN** the playlist is shown at 275 skin pixels wide
- **THEN** the footer shows only the gear and the time readout, and no `+`, `≡`, ADD, REM, SEL, MISC or OPT button

#### Scenario: No filters in the footer
- **WHEN** the collection crate, with tempos and styles, is shown 700 pixels wide
- **THEN** the footer shows the gear and the time readout only, and the BPM control and style chips are in the filter bar

#### Scenario: Options from the footer
- **WHEN** the user clicks the gear and chooses Discogs…
- **THEN** the Discogs dialog opens, as from the right-click Options menu

#### Scenario: Add
- **WHEN** the user right-clicks the list's empty space and chooses Add folder…
- **THEN** the folder dialog opens, as `+` ▸ Add folder… did

#### Scenario: M3U together
- **WHEN** the user right-clicks the list's empty space
- **THEN** the menu holds Export M3U…, which writes the shown crate as `+` ▸ Export M3U… did, and its Add files… opens M3U playlists as `+` ▸ Import M3U… did

### Requirement: Empty crate hint
When the shown crate has no entries and is not a Discogs crate (wantlist or collection), its list SHALL show, centred in the list area under the column header:
- "PASTE A LINK HERE · CMD+V" (CTRL+V off macOS), then the store names **Discogs** and **Bandcamp**;
- "WITH THE BROWSER EXTENSION", then **Juno**, **Bleep**, **Beatport** and **Traxsource**, then **Set up the extension**;
- after a gap, **Add files from your computer**, in crates the user can edit (not label crates), then "OR DROP FILES · PRESS H FOR HELP".

Each store name SHALL be drawn in the store's brand colour, with at least 3:1 contrast against the playlist background, and SHALL open the store's home page in the default browser when clicked. **Set up the extension** SHALL open the Browser dialog (Options ▸ Browser…). **Add files from your computer** SHALL open the file dialog as the list menu's Add files… does, adding to the shown crate. When the list area is too short for these lines, the hint SHALL show "PASTE A DISCOGS OR BANDCAMP LINK", "CMD+V · OR DROP FILES", a gap and "PRESS H FOR HELP" instead. It SHALL NOT move, and SHALL go away as soon as the crate has an entry.

#### Scenario: New crate
- **WHEN** the user creates the crate "asdf" on macOS, the list is tall enough, and it is shown
- **THEN** its list shows "PASTE A LINK HERE · CMD+V" with Discogs and Bandcamp, "WITH THE BROWSER EXTENSION" with Juno, Bleep, Beatport, Traxsource and Set up the extension, then Add files from your computer and "OR DROP FILES · PRESS H FOR HELP", centred

#### Scenario: Store link
- **WHEN** the user clicks Bandcamp in the hint
- **THEN** `https://bandcamp.com` opens in the default browser and the crate stays shown

#### Scenario: Extension link
- **WHEN** the user clicks Set up the extension
- **THEN** the Browser dialog opens with its pairing code

#### Scenario: Add files link
- **WHEN** the empty crate "asdf" is shown and the user clicks Add files from your computer, then picks two audio files
- **THEN** "asdf" holds those two files

#### Scenario: No add link in a label crate
- **WHEN** an empty label crate is shown and the list is tall enough
- **THEN** the hint shows no Add files from your computer link

#### Scenario: Short list
- **WHEN** the playlist shows only 4 rows and the shown crate is empty
- **THEN** the hint shows the four short lines

#### Scenario: Filled
- **WHEN** a Discogs link is pasted into the empty crate and its first entry arrives
- **THEN** the hint is gone

#### Scenario: Empty wantlist crate
- **WHEN** the wantlist crate is shown and empty
- **THEN** no hint is shown, since a paste can't fill it

### Requirement: List menu
The shown crate's actions SHALL be reachable from the playlist list with no footer button: a right-click (or Control-click on macOS) on a row SHALL open the row's menu ending with a "Crate ▸" submenu, and a right-click on the list's empty space (below the last row, or anywhere in an empty crate's list) SHALL open the same items directly. The items SHALL be:
- Select all, Select none, Invert selection;
- Remove selected and Clear crate, in crates the user can edit (not the Discogs wantlist, collection and label crates);
- Sort ▸ (every column's field), Group by record (a checkbox, see `record-view`);
- Add files… and Add folder…, in crates the user can edit;
- Export M3U…, in every crate;
- Open cart on discogs.com, in seller crates and in every crate while the Discogs cart holds a copy (see `discogs-cart`); Open cart on juno.co.uk and Open cart on bleep.com, in every crate while that basket holds an item (see `store-carts`).

Add files… SHALL open the file dialog, accepting audio files and M3U/M3U8 playlists, and add to the shown crate as ⌘O does. Add folder… SHALL open the folder dialog and add the folder's audio, scanned recursively. Export M3U… SHALL write the whole shown crate as M3U8 (see "Persistence and M3U"). A separator SHALL be drawn only between groups that show an item.

#### Scenario: Sort while grouped
- **WHEN** the collection crate is grouped, so no column header is shown, and the user right-clicks a record row and chooses Crate ▸ Sort ▸ BPM
- **THEN** the crate is sorted by BPM, as by clicking the BPM column header

#### Scenario: Empty space
- **WHEN** a crate of 3 entries is shown and the user right-clicks below the last row and chooses Invert selection
- **THEN** the selection is inverted, and no row's menu opened

#### Scenario: Group from the menu
- **WHEN** the user right-clicks a row and ticks Crate ▸ Group by record
- **THEN** the shown crate is grouped, as with ▤

#### Scenario: No empty band
- **WHEN** the collection crate is shown and the user right-clicks its empty space
- **THEN** the menu shows no Remove selected, Clear crate, Add files… or Add folder…, shows Export M3U…, and has no two separators in a row

#### Scenario: Add a folder
- **WHEN** the crate "asdf" is shown and the user right-clicks its empty space, chooses Add folder… and picks a folder holding 3 audio files in a subfolder
- **THEN** "asdf" holds those 3 files

#### Scenario: Import a playlist
- **WHEN** the user chooses Crate ▸ Add files… on a row and picks an M3U8 listing 2 files
- **THEN** both files are added to the shown crate

#### Scenario: Export from the list
- **WHEN** a crate of 12 entries, filtered to show 4, is shown and the user chooses Export M3U… from its empty space's menu
- **THEN** the save dialog proposes "‹crate name›.m3u8", and the file lists all 12 entries

#### Scenario: Open cart from a label crate
- **WHEN** a label crate is shown, the Discogs cart holds 2 copies and the Juno basket is empty
- **THEN** the list menu offers Open cart on discogs.com and not Open cart on juno.co.uk, and choosing it opens `https://www.discogs.com/sell/cart`

#### Scenario: Nothing in any cart
- **WHEN** a label crate is shown and every cart is empty
- **THEN** the list menu offers no Open cart item
