# Spec Delta

## MODIFIED Requirements

### Requirement: Download ahead
The app SHALL keep previews ready for the playing entry and the next 3 entries in its crate's play order. When playback is stopped, it SHALL do the same for the shown crate's current entry (its first entry when it has none) and the 3 after it. An armed entry SHALL be downloaded first. While a record card is open, its record's previews SHALL come right after the armed and playing entries' and before the next 3 (see `record-card`). No other preview SHALL be downloaded, except those of a label crate the user asked to download in full (see `label-crates`), which come after all of these. At most 2 downloads SHALL run at once, audio only, and each entry being downloaded SHALL show its progress (for example "downloading 40%").

#### Scenario: Three ahead
- **WHEN** entry 3 of a 50-entry crate is playing
- **THEN** the previews of entries 4, 5 and 6 are ready or downloading, and no preview after entry 6 is downloaded

#### Scenario: Armed first
- **WHEN** the user double-clicks entry 30 while the previews of entries 4 to 6 are downloading
- **THEN** entry 30's preview starts downloading as soon as a download slot is free, before any other waiting preview

#### Scenario: Play from nothing
- **WHEN** a label is sent with Play while nothing is playing
- **THEN** the previews of the new crate's first 4 entries are downloaded, and the first entry plays as soon as its preview is ready

#### Scenario: An open record card
- **WHEN** entry 3 plays and the user opens the card of a record of 4 tracks further down
- **THEN** entry 3's preview and the record's 4 come before entries 4 to 6, two at a time
