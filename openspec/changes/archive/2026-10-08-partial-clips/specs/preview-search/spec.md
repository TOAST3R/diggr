## MODIFIED Requirements

### Requirement: Tracks waiting to be searched
An entry made from a track that no clip of its record matches (see `discogs-intake`), whether its record has no clip or some clips, SHALL be titled "Artist - Title" from the tracklist (the track's own artist when credited, else the record's), carry its side and its duration, and wait with the status "to search". It SHALL be in the crate's play order like any waiting entry, and SHALL be shown, selected, sorted, filtered and grouped like any other entry.

#### Scenario: A record without clips
- **WHEN** release 38583846 "Hidden Soul EP" by The 89th Passenger, with tracks A1 "Paper Wings" (4:51) to B3 "Analog Serenade" (4:01) and no clip, is expanded into a crate
- **THEN** the crate holds five entries, "The 89th Passenger - Paper Wings" (A1, 4:51) to "The 89th Passenger - Analog Serenade" (B3, 4:01), each "to search"

#### Scenario: A record with one clip
- **WHEN** "Mezzanine" (11 tracks, one video of A3 "Teardrop") is expanded into a crate
- **THEN** A1 "Massive Attack - Angel" (6:18), A2 "Massive Attack - Risingson" (4:58) and the 8 tracks after A3 are "to search", and A3 is not

### Requirement: Trusting a result
A result SHALL be used only when all of the following hold, compared without regard to case, accents or punctuation, and ignoring suffixes such as "Original Mix":
- its title contains the track's title;
- the track's or the record's artist appears in its title or its channel name;
- when the tracklist gives the track's duration, the result's duration is within 10 seconds or 5 % of it, whichever is larger.

Among usable results, the one closest in duration SHALL be used (the first one when the tracklist gives no duration). When none is usable, the entry SHALL become unavailable with the reason "not found by search", and when its record holds back a full-album clip (see `discogs-intake`), that clip SHALL be added after the record's last entry unless the crate already holds it. A found result's video id SHALL become the entry's clip, and its preview SHALL then be downloaded and played like any clip. Every entry of the same track, in every crate, SHALL take the same result. When the found video is already the clip of another entry of the crate, the entry SHALL instead become unavailable with the reason "already in crate", no other search SHALL be run for it, and no full-album clip SHALL be added for it.

#### Scenario: Found
- **WHEN** "The 89th Passenger - Paper Wings" (4:51) is searched, and a result titled "The 89th Passenger – Paper Wings [AF069]" on the channel "Analogical Force", 4:52 long, is listed
- **THEN** that video becomes the entry's clip, and its preview is downloaded

#### Scenario: Wrong length
- **WHEN** the only result naming the track and the artist is a 9:30 live set
- **THEN** it is not used, and the entry becomes unavailable with "not found by search"

#### Scenario: Another artist
- **WHEN** the only result titled "Paper Wings" is by another artist, on another channel
- **THEN** it is not used

#### Scenario: Already in the crate
- **WHEN** "Massive Attack - Exchange" (B2, 4:11) was found by search, and D2 "(Exchange)" (4:08), the same track by artist, title and length, comes up in the window
- **THEN** D2 takes B2's remembered result without running yt-dlp, finds its video already in the crate, and becomes unavailable with "already in crate"; the crate holds that video once

#### Scenario: Full-album fallback
- **WHEN** a record's tracks wait to be searched while its full-album clip is held back, and the search for its B2 ends "not found by search"
- **THEN** the full-album clip is added after the record's last entry, and its preview is downloaded like any clip

### Requirement: Search results are remembered
Each search result SHALL be kept in the cache folder by the track: its artist and title, compared without regard to case, accents or punctuation. A found result SHALL keep the video id, its title and its duration; a not found result the time. A remembered result SHALL be used without running a search, for any entry of the same track from any release, master, page or crate, as long as the track's duration, when the tracklist gives one, is within the tolerance of "Trusting a result" of the remembered duration. A result remembered before this change, by release (or master) and position, SHALL still be used for that release's track. A "not found" older than 7 days SHALL be searched again. A search that failed (yt-dlp missing, offline, timed out) SHALL NOT be remembered, and the entry SHALL wait to be searched again. Clearing the preview cache SHALL NOT forget search results.

#### Scenario: Sent again
- **WHEN** a label whose AF069 tracks were found last week is sent to a new crate
- **THEN** AF069's entries get their clips without running yt-dlp's search

#### Scenario: Vinyl and CD of one record
- **WHEN** "Angel" was found by search for the CD release of "Mezzanine" (position 1, 6:18), and the vinyl release (position A1, 6:18) is then sent
- **THEN** the vinyl entry for "Angel" gets the same clip without running yt-dlp's search

#### Scenario: Another length
- **WHEN** "Angel" was found with a 6:20 video, and a release lists "Angel" as a 3:50 edit
- **THEN** that release's "Angel" is searched

#### Scenario: Offline
- **WHEN** a search fails because YouTube can't be reached
- **THEN** the entry stays "to search", nothing is remembered, and it is searched again when it is next in the window
