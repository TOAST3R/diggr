## ADDED Requirements

### Requirement: Bandcamp pages
The app SHALL accept Bandcamp addresses of a label or artist (`https://‹name›.bandcamp.com/` or `/music`), an album (`/album/‹slug›`) and a track (`/track/‹slug›`), where ‹name› is 1 to 63 lowercase letters, digits or `-`, and ‹slug› is 1 to 200 of the same. A query string or fragment SHALL be ignored. Any other address on `bandcamp.com`, or on any other host, SHALL be refused as Discogs addresses are, and nothing SHALL be added. They SHALL be accepted by paste (Cmd+V, or Ctrl+V on Linux and Windows) and from the browser extension.

#### Scenario: Album with a query
- **WHEN** the address is `https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep?from=search`
- **THEN** it is recognised as that album, and its tracks are read

#### Scenario: Not a Bandcamp page
- **WHEN** the address is `https://analogicalforce.bandcamp.com/merch`
- **THEN** a message lists the supported kinds of page, and no crate changes

#### Scenario: A hostile address
- **WHEN** the address is `https://evil.bandcamp.com.attacker.net/album/x` or `https://a.bandcamp.com/album/x;rm`
- **THEN** it is refused, and yt-dlp is not run

### Requirement: Reading a Bandcamp page
An album or track page SHALL be read with one yt-dlp request that downloads nothing. It SHALL give each track's artist, title, length, album, Bandcamp track id and cover. A label page SHALL list its albums with one request, then read them one at a time, at least 1 s apart. Progress SHALL be shown as for Discogs sends ("Analogical Force: 40 of 78 albums"). The catalogue number SHALL be taken from a leading bracket in the album title ("[AF070] The Ooze EP" → catalogue AF070, album "The Ooze EP"). A track that Bandcamp doesn't stream (a pre-order, for example) SHALL be added unavailable with "no clip (not streamable)". Reading SHALL run off the UI thread and the audio path.

#### Scenario: An album
- **WHEN** `https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep` is pasted into an empty crate
- **THEN** two entries "Patricia - The Ooze" and "Patricia - Swamp" are added with album "The Ooze EP", catalogue number AF070, label Analogical Force and their lengths, and each plays from Bandcamp

#### Scenario: Not streamable
- **WHEN** an album page lists a pre-order track with no stream
- **THEN** its entry is added as unavailable "no clip (not streamable)"

#### Scenario: Read while playing
- **WHEN** a label page with 78 albums is read while a track plays
- **THEN** playback has zero underruns, and seeks stay under 50 ms

### Requirement: Merging into a crate
Each track of a Bandcamp page SHALL be matched against the target crate's entries:
- **The key:** the normalized "artist - title": lowercase, without accents, punctuation, "(original mix)" or "feat. …".
- **Narrowing:** when the album has a catalogue number and some of the crate's entries have the same one, only those entries SHALL be compared.

What happens to each track:
- **A matching entry that plays, or is waiting for or downloading its preview:** the track SHALL be skipped.
- **A matching entry that is unavailable with "no clip", "clip failed" or "not found":** it SHALL get the Bandcamp audio and go back to waiting for its preview, keeping its place, its Discogs origin and its pass.
- **No matching entry:** the track SHALL be added as a new entry, after the crate's last entry with the same catalogue number, or at the end. With skip passed on, it SHALL NOT be added when it was passed before.

The main window SHALL say how many tracks were added, given Bandcamp audio and skipped ("The Ooze EP: 1 added, 1 fixed, 0 skipped").

#### Scenario: Fixes a failed track
- **WHEN** "Label: Analogical Force" holds "Patricia - The Ooze" (AF070) marked "not found", and the album page of AF070 is sent
- **THEN** that entry gets Bandcamp audio and plays, no second "The Ooze" is added, and the main window says 1 fixed

#### Scenario: Skips what plays
- **WHEN** the crate holds "Patricia - Swamp" (AF070) playing from YouTube
- **THEN** the album send leaves it as it is, and counts it as skipped

#### Scenario: Adds what is missing
- **WHEN** the album has a digital-only bonus track that no entry matches
- **THEN** it is added after the crate's last AF070 entry

#### Scenario: Same title, other record
- **WHEN** the crate holds "Patricia - Swamp" from catalogue AF012, and the album AF070 also has "Patricia - Swamp"
- **THEN** the AF070 track is added as a new entry, and the AF012 one is untouched

### Requirement: Sending a Bandcamp page
- **Album or track pages** SHALL be sent in the same modes as Discogs pages (see `discogs-intake`: Enqueue, Play, Crate), with one exception. When a label crate follows that page's Bandcamp label, the tracks SHALL be merged into the label crate, whatever the mode, without changing the shown crate.
- **A Bandcamp label page** SHALL follow that label (see `label-crates`) when sent from the extension, and SHALL add its tracks to the shown crate when pasted, as a Discogs label page does.

#### Scenario: Enqueue an album
- **WHEN** a Bandcamp album is pasted while crate "Friday" is shown and no label crate follows its label
- **THEN** its tracks are merged into "Friday"

#### Scenario: Album of a followed label
- **WHEN** an album of Analogical Force is enqueued from the extension while "Friday" is shown and "Label: Analogical Force" follows its Bandcamp
- **THEN** its tracks are merged into "Label: Analogical Force", and "Friday" stays on screen unchanged

### Requirement: Source of the sound
Every entry whose audio comes from the internet SHALL show its source at the end of its row: **YT** for YouTube, **BC** for Bandcamp. Local files SHALL show none. The tooltip SHALL say "Source: YouTube" or "Source: Bandcamp", with the Bandcamp album when known. When an entry has both a YouTube clip and a Bandcamp track, its menu SHALL offer Play from Bandcamp or Play from YouTube, whichever isn't in use. Choosing it SHALL switch the source, get that preview (keeping the other in the cache) and remember the choice across restarts. Switching the playing entry SHALL take effect the next time it starts.

#### Scenario: Badges
- **WHEN** a crate holds an entry from a Discogs video, one from Bandcamp and a local file
- **THEN** their rows end with YT, BC and nothing

#### Scenario: Switch
- **WHEN** an entry fixed from Bandcamp also has a YouTube clip, and the user chooses Play from YouTube
- **THEN** its badge reads YT, its YouTube preview is downloaded, and it stays YT after a restart

### Requirement: Bandcamp entries
An entry with a Bandcamp track SHALL offer Open on Bandcamp in its menu, opening the track's page in the default browser. An entry with no Discogs release SHALL show Add to wantlist, Add to collection, Open for-sale page, Open release on Discogs and Copy Discogs link disabled. Y and I SHALL do nothing on it, and the main window SHALL say the track isn't on Discogs. Pass (N) and Undo pass SHALL work on every entry.

#### Scenario: Buy it
- **WHEN** the user chooses Open on Bandcamp on "Patricia - The Ooze"
- **THEN** the browser opens https://analogicalforce.bandcamp.com/track/the-ooze

#### Scenario: Not on Discogs
- **WHEN** the user presses Y on a Bandcamp-only entry
- **THEN** nothing is added to the wantlist, and the main window says the track isn't on Discogs
