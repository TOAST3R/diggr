## MODIFIED Requirements

### Requirement: Crates and status
A paired extension SHALL be able to read the list of crate names, with the shown and the playing crate marked, and the player's status: its name, what is playing (artist, title and crate, and, for an entry from Discogs, its release id and side), and the progress of sends that are still running. A paused track SHALL count as playing; nothing SHALL be playing while playback is stopped.

#### Scenario: Crate list
- **WHEN** the extension asks for the crates
- **THEN** it receives every crate's name, with the shown crate and the playing crate marked

#### Scenario: Status
- **WHEN** the extension asks for the status while a label is being expanded and a track plays
- **THEN** it receives the playing track's artist, title and crate, and the label's progress (for example 120 of 312 releases)

#### Scenario: Status names the release and side
- **WHEN** the extension asks for the status while side B2 of release 5077187 plays
- **THEN** the playing track carries release 5077187 and position "B2"

#### Scenario: Not from Discogs
- **WHEN** a track sent from a Juno page plays
- **THEN** the playing track carries no release and an empty position

## ADDED Requirements

### Requirement: Play one track of a release
A send with mode Play of a Discogs release page SHALL accept one optional position (a side as the tracklist shows it, 1 to 8 characters, no control characters). Its tracks SHALL be added as any Play send's, and the entry of the target crate with that release and position (compared ignoring case and surrounding spaces) SHALL play as soon as it can, whether the send added it or the crate already held it. When the send ends and the crate holds no such entry, nothing SHALL play. A position on any other page, with any other mode, or breaking the rules above SHALL be refused with nothing added. The answer SHALL arrive within 100 ms, as for any send.

#### Scenario: A track of a new release
- **WHEN** a paired extension sends `https://www.discogs.com/release/5077187` with mode Play and position "A3", and the crate on screen doesn't hold that release
- **THEN** the release's 11 tracks are added to the end of the crate on screen, and A3 "Teardrop" plays as soon as its preview is ready, not A1

#### Scenario: A release already in the crate
- **WHEN** the same send is made again with position "B1"
- **THEN** nothing is added, and B1 plays at once if its preview is there, or as soon as it is

#### Scenario: Side not in the tracklist
- **WHEN** a Play send of release 5077187 names position "E9"
- **THEN** the release's tracks are added, and when the send ends nothing plays and playback doesn't change

#### Scenario: Position on a label page
- **WHEN** a paired extension sends a label page with a position
- **THEN** the send is refused with 422 and nothing is added

### Requirement: Stop
A paired extension SHALL be able to stop playback with `POST /v1/control` and the body `{"action":"stop"}`. It SHALL do what the player's Stop button does, and SHALL be answered within 100 ms without waiting for the UI. Any other action or field, or an unpaired request, SHALL be refused and change nothing.

#### Scenario: Stop from the page
- **WHEN** a paired extension sends `{"action":"stop"}` to `/v1/control` while a track plays
- **THEN** the answer arrives within 100 ms, playback stops, and the status then names nothing playing

#### Scenario: Unknown action
- **WHEN** a paired extension sends `{"action":"next"}`
- **THEN** it is refused with 422 and playback doesn't change
