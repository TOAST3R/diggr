## MODIFIED Requirements

### Requirement: Empty crate hint
When the shown crate has no entries and is not a Discogs crate (wantlist or collection), its list SHALL show, centred in the list area under the column header:
- "PASTE A LINK HERE · CMD+V" (CTRL+V off macOS), then the store names **Discogs** and **Bandcamp**;
- "WITH THE BROWSER EXTENSION", then **Juno**, **Bleep**, **Beatport** and **Traxsource**, then **Set up the extension**;
- after a gap, "OR DROP FILES · PRESS H FOR HELP".

Each store name SHALL be drawn in the store's brand colour, with at least 3:1 contrast against the playlist background, and SHALL open the store's home page in the default browser when clicked. **Set up the extension** SHALL open the Browser dialog (Options ▸ Browser…). When the list area is too short for these lines, the hint SHALL show "PASTE A DISCOGS OR BANDCAMP LINK", "CMD+V · OR DROP FILES", a gap and "PRESS H FOR HELP" instead. It SHALL show in the normal and maximized layouts, SHALL NOT move, and SHALL go away as soon as the crate has an entry.

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

#### Scenario: Maximized
- **WHEN** the playlist is maximized and the shown crate is empty
- **THEN** the hint shows in the middle of the list

#### Scenario: Filled
- **WHEN** a Discogs link is pasted into the empty crate and its first entry arrives
- **THEN** the hint is gone

#### Scenario: Empty wantlist crate
- **WHEN** the wantlist crate is shown and empty
- **THEN** no hint is shown, since a paste can't fill it
