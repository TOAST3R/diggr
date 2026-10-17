## MODIFIED Requirements

### Requirement: Entry tempo
Each entry SHALL carry its tempo once any analysis of its audio is known: from preview preparation, from the playing track's analysis, or from the score cache. The tempo SHALL be the tempo of the score's longest steady-tempo stretch, doubled or halved until it lies within 88–176 BPM, and rounded to a whole number. The tempo SHALL be saved with the crate. Until its audio is analysed, an entry from Beatport or Traxsource SHALL carry the BPM its store gives, and analysis SHALL replace it. The app SHALL NOT download or analyse audio only to learn a tempo, and SHALL NOT take a tempo from file tags.

#### Scenario: Prepared preview
- **WHEN** the next entry's preview is prepared and its analysed tempo is 124.3 BPM
- **THEN** the entry shows "(124 BPM)" within one UI frame of preparation finishing

#### Scenario: Half time folded
- **WHEN** a track is analysed at 87 BPM
- **THEN** its entry shows "(174 BPM)"

#### Scenario: Double time folded
- **WHEN** a track is analysed at 280 BPM
- **THEN** its entry shows "(140 BPM)"

#### Scenario: Store BPM first
- **WHEN** a Beatport track listed at 126 BPM is added, and later its preview is analysed at 125.7 BPM
- **THEN** its entry shows "(126 BPM)" at once, and still "(126 BPM)" after analysis, now from the analyser

#### Scenario: Survives a restart
- **WHEN** an entry showing "(128 BPM)" is in a crate and the app is restarted
- **THEN** the entry shows "(128 BPM)" in the first frame, without analysis running

#### Scenario: Nothing fetched for a BPM
- **WHEN** a 312-release label crate is shown while entry 1 plays
- **THEN** only previews within the download-ahead horizon are downloaded, and entries outside it show no BPM
