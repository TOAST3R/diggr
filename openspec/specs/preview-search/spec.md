# preview-search Specification

## Purpose
Finds a clip for each track of a record that has a tracklist but no videos on Discogs, by asking the user's yt-dlp to search near the playhead, trusting a result only when its title, artist and duration agree with the tracklist, and remembering what was found.
## Requirements
### Requirement: Tracks waiting to be searched
An entry made from a track of a record with no usable clip (see `discogs-intake`) SHALL be titled "Artist - Title" from the tracklist (the track's own artist when credited, else the record's), carry its side and its duration, and wait with the status "to search". It SHALL be in the crate's play order like any waiting entry, and SHALL be shown, selected, sorted, filtered and grouped like any other entry.

#### Scenario: A record without clips
- **WHEN** release 38583846 "Hidden Soul EP" by The 89th Passenger, with tracks A1 "Paper Wings" (4:51) to B3 "Analog Serenade" (4:01) and no clip, is expanded into a crate
- **THEN** the crate holds five entries, "The 89th Passenger - Paper Wings" (A1, 4:51) to "The 89th Passenger - Analog Serenade" (B3, 4:01), each "to search"

### Requirement: Search near the playhead
An entry waiting to be searched SHALL be searched only when it is in the download-ahead window (the armed entry, the playing entry and the next 3 in play order, see `preview-fetch`). At most one search SHALL run at a time, before the window's downloads. A search SHALL ask the user's yt-dlp to list up to 5 results for the track's artist and title, without downloading anything, and SHALL time out after 30 s. A search not yet started SHALL be dropped when its entry leaves the window. Searching SHALL NOT delay playback, SHALL cause zero underruns, and SHALL NOT make any Discogs request.

#### Scenario: Only what is about to play
- **WHEN** a crate holds 60 entries to search and entry 10 is playing
- **THEN** only entries 11 to 13 (and the playing one, when it is to search) are searched, one at a time

#### Scenario: Armed first
- **WHEN** the user double-clicks an entry to search far down the crate
- **THEN** it is searched before the other entries of the window, and plays once its preview is downloaded

### Requirement: Trusting a result
A result SHALL be used only when all of the following hold, compared without regard to case, accents or punctuation, and ignoring suffixes such as "Original Mix":
- its title contains the track's title;
- the track's or the record's artist appears in its title or its channel name;
- when the tracklist gives the track's duration, the result's duration is within 10 seconds or 5 % of it, whichever is larger.

Among usable results, the one closest in duration SHALL be used (the first one when the tracklist gives no duration). When none is usable, the entry SHALL become unavailable with the reason "not found by search". A found result's video id SHALL become the entry's clip, and its preview SHALL then be downloaded and played like any clip. Every entry of the same track, in every crate, SHALL take the same result.

#### Scenario: Found
- **WHEN** "The 89th Passenger - Paper Wings" (4:51) is searched, and a result titled "The 89th Passenger – Paper Wings [AF069]" on the channel "Analogical Force", 4:52 long, is listed
- **THEN** that video becomes the entry's clip, and its preview is downloaded

#### Scenario: Wrong length
- **WHEN** the only result naming the track and the artist is a 9:30 live set
- **THEN** it is not used, and the entry becomes unavailable with "not found by search"

#### Scenario: Another artist
- **WHEN** the only result titled "Paper Wings" is by another artist, on another channel
- **THEN** it is not used

### Requirement: Search results are remembered
Each search result SHALL be kept in the cache folder by the record's release (or master) and the track's position: the video id and title when found, or the time when not found. A remembered result SHALL be used without running a search. A "not found" older than 7 days SHALL be searched again. A search that failed (yt-dlp missing, offline, timed out) SHALL NOT be remembered, and the entry SHALL wait to be searched again. Clearing the preview cache SHALL NOT forget search results.

#### Scenario: Sent again
- **WHEN** a label whose AF069 tracks were found last week is sent to a new crate
- **THEN** AF069's entries get their clips without running yt-dlp's search

#### Scenario: Offline
- **WHEN** a search fails because YouTube can't be reached
- **THEN** the entry stays "to search", nothing is remembered, and it is searched again when it is next in the window

### Requirement: A found track says so
The tooltip of an entry from a searched track SHALL say "Preview: to search", "Preview: found by search (‹video title›)" or "Preview: not found by search". The found video's title SHALL be saved with the crate.

#### Scenario: Hover a found track
- **WHEN** the pointer rests on "The 89th Passenger - Paper Wings" after its search found a video
- **THEN** the tooltip says "Preview: found by search (The 89th Passenger – Paper Wings [AF069])"

