## MODIFIED Requirements

### Requirement: Download ahead
The app SHALL keep previews ready for the playing entry and the next 3 entries in its crate's play order. When playback is stopped, it SHALL do the same for the shown crate's current entry (its first entry when it has none) and the 3 after it. An armed entry SHALL be downloaded first. No other preview SHALL be downloaded, except those of a label crate the user asked to download in full (see `label-crates`), which come after all of these. At most 2 downloads SHALL run at once, audio only, and each entry being downloaded SHALL show its progress (for example "downloading 40%").

#### Scenario: Three ahead
- **WHEN** entry 3 of a 50-entry crate is playing
- **THEN** the previews of entries 4, 5 and 6 are ready or downloading, and no preview after entry 6 is downloaded

#### Scenario: Armed first
- **WHEN** the user double-clicks entry 30 while the previews of entries 4 to 6 are downloading
- **THEN** entry 30's preview starts downloading as soon as a download slot is free, before any other waiting preview

#### Scenario: Play from nothing
- **WHEN** a label is sent with Play while nothing is playing
- **THEN** the previews of the new crate's first 4 entries are downloaded, and the first entry plays as soon as its preview is ready

### Requirement: Preview cache
Previews SHALL be stored in `previews/` in the cache folder. They SHALL use at most the preview cache size, which is 2 GB by default and set in Options ▸ Discogs…. When a new preview doesn't fit, the previews played least recently SHALL be deleted first. A preview downloaded for Download all tracks SHALL never cause a deletion: downloading pauses instead (see `label-crates`). The previews of the playing entry, the armed entry and the next 3 entries SHALL never be deleted. A deleted preview SHALL be downloaded again when it is needed. When the downloaded file is identical, its cached analysis and waveform SHALL be reused.

#### Scenario: Over the limit
- **WHEN** a new preview would take the cache over its limit
- **THEN** the least recently played previews are deleted until it fits, and the playing, armed and next 3 previews are kept

#### Scenario: Downloaded again
- **WHEN** a preview that was deleted is downloaded again as an identical file
- **THEN** it starts with its whole waveform and sections, without being analyzed again
