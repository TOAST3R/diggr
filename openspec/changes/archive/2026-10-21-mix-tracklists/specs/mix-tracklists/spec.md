## ADDED Requirements

### Requirement: Mixes from YouTube and SoundCloud
Pasting the address of a YouTube video or a SoundCloud track in the player, or sending it from the browser extension's button on its page, SHALL read the mix's title, length, chapters and description with one yt-dlp metadata read, downloading no media, off the UI thread and the audio path. When it has at least 3 chapters, the chapters SHALL be its tracklist; otherwise the description's lines SHALL be parsed. When fewer than 3 tracks are found, the main window SHALL say "No tracklist found in ‹title›", and nothing SHALL be added.

#### Scenario: Chapters
- **WHEN** the user pastes a YouTube mix with 24 chapters titled "Artist - Title"
- **THEN** a crate "Mix: ‹video title›" appears with 24 entries in chapter order

#### Scenario: No tracklist
- **WHEN** the user pastes a YouTube video whose description has no track lines and no chapters
- **THEN** the main window says "No tracklist found in ‹title›", and no crate changes

### Requirement: Mixes from 1001tracklists and Mixcloud
1001tracklists and Mixcloud tracklists SHALL reach the app only as a tracklist send from the paired browser extension, built from the page the user has open (the extension's readers for those pages are a later change; the app takes the send already). The app SHALL NOT fetch any page of 1001tracklists.com or mixcloud.com, and SHALL NOT pass their addresses to yt-dlp. A tracklist send SHALL hold the mix's address and title (≤ 200) and 1 to 300 tracks, each with a time (0 to 43,200 s, optional), an artist and a title (≤ 200 each), and optionally a version and a label (≤ 200 each), or an ID mark. Text SHALL hold no control characters. A track breaking a rule SHALL be refused alone. A pasted address of either site SHALL be refused with a message saying to send it from the browser extension.

#### Scenario: A 1001tracklists page
- **WHEN** the extension sends a tracklist of 18 tracks, 3 of them IDs
- **THEN** a crate "Mix: ‹title›" holds 18 entries, 3 of them unidentified

### Requirement: Tracklist lines
Each tracklist line SHALL be read by taking off a leading time (`h:mm:ss`, `mm:ss`, or in brackets), track number and bullet; splitting artist from title at the first dash with spaces around it (`-`, `–` or `—`); taking a trailing `[…]` as the label and a trailing parenthesis ending in Remix, Mix, Edit, Dub, Rework or Version as the version. A line reading ID, "ID - ID" or Unknown SHALL be an unidentified track. Lines without such a dash SHALL be ignored, and so SHALL lines reading as a sentence: an artist of more than 8 words or a title of more than 15.

#### Scenario: A full line
- **WHEN** a description line reads "12:34 Kerri Chandler – Rain (Dennis Ferrer Remix) [Madhouse]"
- **THEN** the track has time 754 s, artist "Kerri Chandler", title "Rain", version "Dennis Ferrer Remix" and label "Madhouse"

#### Scenario: Not a track
- **WHEN** a description line reads "Follow me on Instagram"
- **THEN** it is ignored

#### Scenario: A sentence with a dash
- **WHEN** a description line reads "was on the phone so songs played on too long - also messed up some transitions"
- **THEN** it is ignored

### Requirement: The Mix crate
A tracklist SHALL become a crate named "Mix: ‹title›" (at most 40 characters), with one entry per track in mix order, titled "Artist - Title (Version)". Each identified entry SHALL wait to be searched as a Discogs track without a clip does (see `preview-search`). An unidentified entry SHALL be unavailable with the reason "unidentified in the mix". Sending or pasting the same mix again SHALL add only the tracks the crate doesn't hold. A pasted mix's crate SHALL be shown. A mix sent from its page, like a tracklist send, SHALL follow the send's mode. A tracklist send SHALL fill the mix's crate, or the crate its mode names; with mode Play its crate SHALL be shown and its first identified track started (one not found hands over to the next, see `not-found-notice`). Filling SHALL cause zero underruns.

#### Scenario: Played by search
- **WHEN** the Mix crate's third entry comes within the download-ahead window
- **THEN** it is searched for, and plays when a matching upload is found

#### Scenario: Updated tracklist
- **WHEN** a 1001tracklists page that had 3 IDs now names 2 of them, and it is sent again
- **THEN** the 2 named tracks are added, and nothing is doubled

### Requirement: Open in mix
Every Mix crate entry SHALL remember its mix's address and its time in the mix. Its menu SHALL offer "Open in mix at m:ss", opening in the default browser `https://www.youtube.com/watch?v=‹id›&t=‹s›s` for YouTube, `https://soundcloud.com/‹user›/‹slug›#t=‹m›:‹ss›` for SoundCloud, and the mix page for Mixcloud and 1001tracklists. An entry without a time SHALL offer "Open mix".

#### Scenario: YouTube moment
- **WHEN** the user chooses Open in mix at 12:34 on an entry of YouTube mix `dQw4w9WgXcQ`
- **THEN** the default browser opens `https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=754s`

### Requirement: Find on Discogs
The right-click menu of an identified mix track not on a Discogs record SHALL offer Find on Discogs. With a Discogs token it SHALL make one Discogs search by the track's artist and track name. When exactly one record (not a file release, by that artist) fits, the entry SHALL be linked to it and get the Discogs actions (wantlist, for sale, owned, cart). When several fit, or none, nothing SHALL be linked: the main window SHALL say so and Discogs' search for the track SHALL open in the default browser. Without a token the main window SHALL say one is needed. Nothing SHALL be looked up on Discogs for a mix unless asked.

#### Scenario: One release
- **WHEN** the user chooses Find on Discogs on "Kerri Chandler - Rain (Dennis Ferrer Remix)" and Discogs lists one vinyl release by Kerri Chandler with that track
- **THEN** the entry is linked to that release

#### Scenario: Several pressings
- **WHEN** the user chooses Find on Discogs on "Moodymann - Shades of Jae" and two releases fit
- **THEN** nothing is linked, the main window says "2 releases fit", and `https://www.discogs.com/search/?q=Moodymann%20Shades%20of%20Jae&type=release` opens

