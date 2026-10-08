## MODIFIED Requirements

### Requirement: Owned mark
An entry SHALL be marked owned when its release is in the user's collection, or when another release of the same master release is. An owned entry SHALL show a clearly visible "OWNED" badge at the start of its title, in the single-line format and in columns, without changing the row's colour. The badge's colour SHALL differ from the skin's playlist text and LCD colours (off-white in the default skin). Its tooltip SHALL say whether this pressing or another pressing is owned, naming the owned pressing's catalog number and year. The mark SHALL follow the latest cached collection, in every crate, without re-saving crates.

#### Scenario: This pressing
- **WHEN** an entry from release 123456 is shown and release 123456 is in the collection
- **THEN** its row shows the OWNED badge, and its tooltip says "Owned: this pressing"

#### Scenario: Another pressing
- **WHEN** an entry from release 111 (master 900) is shown and the collection holds release 222 of master 900, catalog number AF001R, from 2019
- **THEN** its row shows the OWNED badge, and its tooltip says "Owned: another pressing (AF001R, 2019)"

#### Scenario: Not owned
- **WHEN** an entry's release and master are not in the collection
- **THEN** no badge is shown

#### Scenario: Badge stands out
- **WHEN** an owned entry is shown in the default skin
- **THEN** its OWNED badge is off-white, not the amber of the playlist text
