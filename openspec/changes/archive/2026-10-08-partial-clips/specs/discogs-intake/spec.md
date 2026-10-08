## MODIFIED Requirements

### Requirement: Expanding a page into tracks
Each track of a record's tracklist SHALL become exactly one entry: an entry with the track's clip when a clip matches the track, otherwise an entry waiting to be searched (see `preview-search`). The entry is titled "Artist - Title" from the tracklist. Clips that match no track SHALL each become one more entry after the record's tracks, titled from the clip's own title, except a full-album clip (see below). A clip that a record lists more than once (the same video id) SHALL count once. Each entry carries its origin: page, release, master, label, catalog number, year, side and clip. The records of a page SHALL be:
- a release: that release;
- a master release: the master's own tracklist and clips, with the label, catalog number, year and for-sale numbers of its main release;
- an artist: the releases where the artist has the main role or a remix credit, oldest first; for a remix credit, only the tracks and clips whose title names the artist, each track of those becoming one entry as above;
- a label: all its releases, in the order Discogs lists them;
- a wantlist: every release in it;
- a list: its releases and master releases, in list order.

A full-album clip is a clip that matches no track and either has "full album" in its title (compared without regard to case, accents or punctuation) or is the record's only clip. When the record has tracks waiting to be searched, its first full-album clip SHALL NOT be added with the record's entries: it SHALL be kept with them, saved with the crate, and added after the record's entries once a search for one of them ends "not found by search" (see `preview-search`).

A clip that the target crate already holds SHALL NOT be added again. A track waiting to be searched, or found by search, that the target crate already holds (the same record and position) SHALL NOT be added again. A record with no tracklist SHALL become one entry per usable clip; with nothing to search, a full-album clip is such an entry. A record with no usable clip and no tracklist SHALL appear once, as an unavailable entry marked "no clip". A record that a page's listing gives more than once (a label credited several times on a release) SHALL be listed, fetched and counted once.

#### Scenario: Listed three times
- **WHEN** a label's listing gives release 38583846 three times and it has no clip
- **THEN** the crate holds one "no clip" entry for it, and the progress counts it once

#### Scenario: Release with three clips
- **WHEN** a release with a four-track tracklist and three clips, each matching a track, is sent
- **THEN** four entries appear in tracklist order: three with their clips, each titled after its matching track, with its side, and one waiting to be searched; all four carry the release's catalog number and year

#### Scenario: One video for eleven tracks
- **WHEN** release 5077187, Massive Attack "Mezzanine", with 11 tracks (A1 "Angel" to D2 "(Exchange)") and the video "Massive Attack - Teardrop (Official Video)" listed twice, is sent
- **THEN** the crate holds 11 entries in tracklist order: A3 "Massive Attack - Teardrop" with that video, and the other 10 waiting to be searched

#### Scenario: Remix credit
- **WHEN** an artist is credited with one remix on a release that has four clips
- **THEN** only the clip whose title names that artist is added

#### Scenario: Remix credit without its clip
- **WHEN** Lumen is credited with the remix "Glasshouse (Lumen Remix)" on a release whose four clips are all of other tracks
- **THEN** one entry, "Nightcraft - Glasshouse (Lumen Remix)", is added, waiting to be searched

#### Scenario: Full-album upload
- **WHEN** a release with 8 tracks has one clip, "Nightcraft - Glasshouse EP (Full Album)", which matches no track
- **THEN** the crate holds 8 entries waiting to be searched and no entry for the full-album clip

#### Scenario: Full-album clip as a fallback
- **WHEN** the search for one of those 8 tracks ends "not found by search"
- **THEN** one entry for the full-album clip is added after the record's last entry, and a later failed search of that record adds no other

#### Scenario: After a restart
- **WHEN** the app is quit while those 8 tracks wait to be searched, started again, and one of them is not found by search
- **THEN** the full-album clip is added after the record's last entry

#### Scenario: No clip
- **WHEN** one release of a label has no clips and no tracklist
- **THEN** one entry for that release appears, unavailable, marked "no clip"

#### Scenario: Sent twice
- **WHEN** the same release is sent twice to the same crate
- **THEN** its clips appear only once

#### Scenario: Tracks to search sent twice
- **WHEN** "Mezzanine" is sent twice to the same crate
- **THEN** the crate still holds its 11 entries, and none of the 10 searched tracks appears twice

#### Scenario: No clip, but a tracklist
- **WHEN** release 38583846, with five tracks and no clip, is expanded
- **THEN** the crate holds five entries, "The 89th Passenger - Paper Wings" to "The 89th Passenger - Analog Serenade", with their sides and durations, waiting to be searched
