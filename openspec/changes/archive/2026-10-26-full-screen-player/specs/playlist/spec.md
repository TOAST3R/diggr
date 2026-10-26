## ADDED Requirements

### Requirement: Title bar buttons
The playlist's title bar SHALL hold, at its right end, the ▤ button that groups the shown crate by record (see `record-view`). It SHALL have no close button and no button that maximizes or restores the playlist.

#### Scenario: Title bar buttons
- **WHEN** the playlist is shown
- **THEN** its title bar shows ▤ at its right end, and no close or maximize button

## MODIFIED Requirements

### Requirement: Playlist placement and size
The playlist SHALL be drawn under the band (see `player-window` "Full-screen window") at the window's full width, its rows filling the height left, with columns when it is wide enough. Its width and rows SHALL follow the window; it SHALL have no resize handle and SHALL always be shown (there is no playlist toggle). Resizing the window SHALL NOT affect playback.

#### Scenario: Fills the window
- **WHEN** the window is 1100 × 600 skin pixels and SYNC is off
- **THEN** the playlist is 1100 skin pixels wide and its rows fill the 542 pixels under the 58-pixel band

#### Scenario: Follows the window
- **WHEN** the operating system restores the window to 800 × 500 skin pixels
- **THEN** the playlist is 800 skin pixels wide, with fewer rows, and the shown crate and playback are unchanged

#### Scenario: Always shown
- **WHEN** the app starts with a `settings.ron` holding `show_playlist: false`
- **THEN** the playlist is shown

### Requirement: Empty crate hint
When the shown crate has no entries and is not a Discogs crate (wantlist or collection), its list SHALL show, centred in the list area under the column header:
- "PASTE A LINK HERE · CMD+V" (CTRL+V off macOS), then the store names **Discogs** and **Bandcamp**;
- "WITH THE BROWSER EXTENSION", then **Juno**, **Bleep**, **Beatport** and **Traxsource**, then **Set up the extension**;
- after a gap, "OR DROP FILES · PRESS H FOR HELP".

Each store name SHALL be drawn in the store's brand colour, with at least 3:1 contrast against the playlist background, and SHALL open the store's home page in the default browser when clicked. **Set up the extension** SHALL open the Browser dialog (Options ▸ Browser…). When the list area is too short for these lines, the hint SHALL show "PASTE A DISCOGS OR BANDCAMP LINK", "CMD+V · OR DROP FILES", a gap and "PRESS H FOR HELP" instead. It SHALL NOT move, and SHALL go away as soon as the crate has an entry.

#### Scenario: New crate
- **WHEN** the user creates the crate "asdf" on macOS, the list is tall enough, and it is shown
- **THEN** its list shows "PASTE A LINK HERE · CMD+V" with Discogs and Bandcamp, "WITH THE BROWSER EXTENSION" with Juno, Bleep, Beatport, Traxsource and Set up the extension, and "OR DROP FILES · PRESS H FOR HELP", centred

#### Scenario: Store link
- **WHEN** the user clicks Bandcamp in the hint
- **THEN** `https://bandcamp.com` opens in the default browser and the crate stays shown

#### Scenario: Extension link
- **WHEN** the user clicks Set up the extension
- **THEN** the Browser dialog opens with its pairing code

#### Scenario: Short list
- **WHEN** the playlist shows only 4 rows and the shown crate is empty
- **THEN** the hint shows the four short lines

#### Scenario: Filled
- **WHEN** a Discogs link is pasted into the empty crate and its first entry arrives
- **THEN** the hint is gone

#### Scenario: Empty wantlist crate
- **WHEN** the wantlist crate is shown and empty
- **THEN** no hint is shown, since a paste can't fill it

## REMOVED Requirements

### Requirement: Maximized playlist
**Reason**: The full-screen layout is now the only one (see `player-window` "Full-screen window"), so there is nothing to maximize or restore.
**Migration**: The app opens full screen. An old `playlist_maximized` setting is ignored; ⇔ and Shift+P are gone.
