## ADDED Requirements

### Requirement: Every downloaded clip is analyzed
The app SHALL analyze the whole file of every downloaded preview in the playing crate and the shown crate, once, in the background, and give its entries the resulting tempo. It SHALL analyze in this order:
1. the playing track, unless its score is complete;
2. entries the user asked for with Analyze BPM, in the order asked;
3. the next tracks to play;
4. the other downloaded previews, in crate order.

It SHALL analyze up to 3 files at a time (half the cores, at least one), on low-priority threads, each taking the next file in that order. A file whose analysis fails SHALL be passed without a tempo, and the others SHALL still be analyzed. It SHALL start only while nothing plays, or the playing track's own analysis has 32 bars or is complete, or the track has played for 15 s. A file whose complete score is already cached SHALL take its tempo from the cache without analysis. The app SHALL NOT download audio only to analyze it.

#### Scenario: Downloaded in the background
- **WHEN** "Download all tracks" downloads 40 previews of a label crate while entry 1 plays and its analysis has 32 bars
- **THEN** each of the 40 entries shows its BPM once its analysis finishes, without being played

#### Scenario: Played straight away
- **WHEN** the user double-clicks a downloaded 6-minute track no one has analyzed, then seeks to 2:00 and skips to the next one at 2:30
- **THEN** the track's whole file is still analyzed and its entry shows its BPM, and the next track starts within 30 ms

#### Scenario: Playing first
- **WHEN** 10 downloaded previews wait to be analyzed and the user starts an unanalyzed one
- **THEN** the one playing is the next file analyzed, after the file in progress

#### Scenario: Cached
- **WHEN** a preview is downloaded again whose complete score is in the cache
- **THEN** its entry shows its BPM within one UI frame of its turn, and nothing is decoded for it

#### Scenario: Playback undisturbed
- **WHEN** 40 previews are analyzed in the background while tracks play, seek and skip
- **THEN** there are 0 underruns, starts stay under 30 ms and seeks under 50 ms

#### Scenario: Beatless track playing
- **WHEN** a track with no beat has played for 15 s
- **THEN** the background analysis starts, even though the track's own analysis never reaches 32 bars

### Requirement: Analysis progress in the BPM column
While an entry's audio is being analyzed for its tempo, its BPM cell SHALL show a bar filled to the share of the file analyzed. An entry waiting in the analysis queue, with no BPM, SHALL show the bar empty (its frame only) in its BPM cell. The entry's tooltip SHALL say "analyzing N%" or "BPM queued". An entry with a known BPM SHALL show that BPM, even while it is analyzed again. When the BPM column is hidden or the playlist is narrow, the bar SHALL be shown in a place of its own before the row's time, so a long name never hides it.

#### Scenario: Analyzing
- **WHEN** a 5-minute preview is analyzed and 2 minutes of it are done
- **THEN** its BPM cell shows a bar 40% full and its tooltip says "analyzing 40%"

#### Scenario: Queued
- **WHEN** a downloaded preview with no BPM waits behind 3 others
- **THEN** its BPM cell shows an empty bar frame and its tooltip says "BPM queued"

#### Scenario: Done
- **WHEN** that preview's analysis finishes at 124.3 BPM
- **THEN** within one UI frame its BPM cell shows 124 and no bar

### Requirement: Analyze BPM on request
An entry's right-click menu SHALL offer Analyze BPM for the selection (or the entry) when any of it has audio on disk and no complete score: a downloaded preview or a local file. Choosing it SHALL put those entries' audio at the front of the analysis queue, after the playing track, say "Analyzing the BPM of N tracks next", and their BPM cells SHALL show the empty bar at once. Once each is analyzed the app SHALL say "‹title›: 124 BPM" or "‹title›: no steady tempo found". Local files SHALL be analyzed only when asked or when played.

#### Scenario: Ask for a local file
- **WHEN** the user selects 3 local files with no BPM and chooses Analyze BPM
- **THEN** their cells show the empty bar at once, they are analyzed before the other queued previews, and each then shows its BPM

#### Scenario: Nothing to analyze
- **WHEN** the user right-clicks an entry still waiting for its download
- **THEN** the menu shows no Analyze BPM
