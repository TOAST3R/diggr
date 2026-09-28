## MODIFIED Requirements

### Requirement: Playlist display
The playlist section SHALL list entries as "N. (catno) Artist: Title (T BPM)" with durations, highlight the current track, show total/selected duration, and scroll. "(catno) " SHALL appear only when the entry has a catalog number, "Artist: " only when it has an artist, and " (T BPM)" only when its tempo is known. An entry waiting for its audio SHALL be drawn dimmed, with a short status (for example "listed" or "downloading 40%") where its duration would be. An unavailable entry SHALL be drawn dimmed, with its reason (for example "no clip") where its duration would be. Only an entry whose file could not be opened or decoded SHALL be drawn in the error colour.

#### Scenario: Current highlighted
- **WHEN** track 4 is playing
- **THEN** entry 4 is drawn in the highlight color

#### Scenario: Discogs entry name
- **WHEN** entry 3 is "Nightcraft" / "Glasshouse" from catalog number LT-012, with a known tempo of 124 BPM
- **THEN** it reads "3. (LT-012) Nightcraft: Glasshouse (124 BPM)"

#### Scenario: Local file without tempo
- **WHEN** entry 5 is a local file tagged "Mira Sol" / "Coastline" that has never been analysed
- **THEN** it reads "5. Mira Sol: Coastline", with no catalog number and no BPM

#### Scenario: Waiting is not an error
- **WHEN** an entry's audio is 40% downloaded
- **THEN** the entry is drawn dimmed, with "downloading 40%" where its duration would be, and not in the error colour

#### Scenario: Unavailable stays visible
- **WHEN** an entry will never have audio because its record has no clip
- **THEN** it stays in the list, dimmed, with "no clip" where its duration would be

## ADDED Requirements

### Requirement: Entry tempo
Each entry SHALL carry its tempo once any analysis of its audio is known: from preview preparation, from the playing track's analysis, or from the score cache. The tempo SHALL be the tempo of the score's longest steady-tempo stretch, doubled or halved until it lies within 88–176 BPM, and rounded to a whole number. The tempo SHALL be saved with the crate. The app SHALL NOT download or analyse audio only to learn a tempo, and SHALL NOT take a tempo from file tags.

#### Scenario: Prepared preview
- **WHEN** the next entry's preview is prepared and its analysed tempo is 124.3 BPM
- **THEN** the entry shows "(124 BPM)" within one UI frame of preparation finishing

#### Scenario: Half time folded
- **WHEN** a track is analysed at 87 BPM
- **THEN** its entry shows "(174 BPM)"

#### Scenario: Double time folded
- **WHEN** a track is analysed at 280 BPM
- **THEN** its entry shows "(140 BPM)"

#### Scenario: Survives a restart
- **WHEN** an entry showing "(128 BPM)" is in a crate and the app is restarted
- **THEN** the entry shows "(128 BPM)" in the first frame, without analysis running

#### Scenario: Nothing fetched for a BPM
- **WHEN** a 312-release label crate is shown while entry 1 plays
- **THEN** only previews within the download-ahead horizon are downloaded, and entries outside it show no BPM
