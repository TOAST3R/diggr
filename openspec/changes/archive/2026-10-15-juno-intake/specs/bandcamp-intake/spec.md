## MODIFIED Requirements

### Requirement: Source of the sound
Every entry whose audio comes from the internet SHALL show its source at the end of its row: **YT** for YouTube, **BC** for Bandcamp, **JN** for Juno. Local files SHALL show none. The tooltip SHALL say "Source: YouTube", "Source: Bandcamp", with the Bandcamp album when known, or "Source: Juno sample (2 min)". When an entry has more than one of a YouTube clip, a Bandcamp track and a Juno sample, its menu SHALL offer Play from ‹source› for each one that isn't in use. Choosing it SHALL switch the source, get that preview (keeping the other in the cache) and remember the choice across restarts. Switching the playing entry SHALL take effect the next time it starts.

#### Scenario: Badges
- **WHEN** a crate holds an entry from a Discogs video, one from Bandcamp and a local file
- **THEN** their rows end with YT, BC and nothing

#### Scenario: Switch
- **WHEN** an entry fixed from Bandcamp also has a YouTube clip, and the user chooses Play from YouTube
- **THEN** its badge reads YT, its YouTube preview is downloaded, and it stays YT after a restart

#### Scenario: Juno badge
- **WHEN** a crate holds an entry sent from a Juno page
- **THEN** its row ends with JN, and its tooltip says "Source: Juno sample (2 min)"

#### Scenario: Three sources
- **WHEN** an entry plays from Juno and also has a YouTube clip and a Bandcamp track
- **THEN** its menu offers Play from YouTube and Play from Bandcamp
