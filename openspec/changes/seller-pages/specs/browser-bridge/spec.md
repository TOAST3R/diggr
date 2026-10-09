## MODIFIED Requirements

### Requirement: Crates and status
A paired extension SHALL be able to read the list of crate names, with the shown and the playing crate marked, and the usernames of the app's Top Sellers, and the player's status: its name, what is playing (artist, title and crate), and the progress of sends that are still running.

#### Scenario: Crate list
- **WHEN** the extension asks for the crates
- **THEN** it receives every crate's name, with the shown crate and the playing crate marked, and the Top Sellers' usernames

#### Scenario: Status
- **WHEN** the extension asks for the status while a label is being expanded and a track plays
- **THEN** it receives the playing track's artist, title and crate, and the label's progress (for example 120 of 312 releases)
