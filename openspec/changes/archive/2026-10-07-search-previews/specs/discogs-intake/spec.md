## MODIFIED Requirements

### Requirement: Expanding a page into tracks
Each usable clip of a record SHALL become one entry. The entry is titled "Artist - Title" from the record's tracklist when the clip matches a track, and from the clip's own title otherwise. It carries its origin: page, release, master, label, catalog number, year, side and clip. The records of a page SHALL be:
- a release: that release;
- a master release: the master's own clips, with the label, catalog number, year and for-sale numbers of its main release;
- an artist: the releases where the artist has the main role or a remix credit, oldest first; for a remix credit, only the clips whose title names the artist;
- a label: all its releases, in the order Discogs lists them;
- a wantlist: every release in it;
- a list: its releases and master releases, in list order.

A clip that the target crate already holds SHALL NOT be added again. A record with no usable clip SHALL become one entry per track of its tracklist, waiting to be searched (see `preview-search`). A record with no usable clip and no tracklist SHALL appear once, as an unavailable entry marked "no clip". A record that a page's listing gives more than once (a label credited several times on a release) SHALL be listed, fetched and counted once.

#### Scenario: Listed three times
- **WHEN** a label's listing gives release 38583846 three times and it has no clip
- **THEN** the crate holds one "no clip" entry for it, and the progress counts it once

#### Scenario: Release with three clips
- **WHEN** a release with a four-track tracklist and three clips is sent
- **THEN** three entries appear, each titled after its matching track, with its side, and all three carry the release's catalog number and year

#### Scenario: Remix credit
- **WHEN** an artist is credited with one remix on a release that has four clips
- **THEN** only the clip whose title names that artist is added

#### Scenario: No clip
- **WHEN** one release of a label has no clips
- **THEN** one entry for that release appears, unavailable, marked "no clip"

#### Scenario: Sent twice
- **WHEN** the same release is sent twice to the same crate
- **THEN** its clips appear only once

#### Scenario: No clip, but a tracklist
- **WHEN** release 38583846, with five tracks and no clip, is expanded
- **THEN** the crate holds five entries, "The 89th Passenger - Paper Wings" to "The 89th Passenger - Analog Serenade", with their sides and durations, waiting to be searched
