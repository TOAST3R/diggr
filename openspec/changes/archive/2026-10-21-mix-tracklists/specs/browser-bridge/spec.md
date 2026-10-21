## MODIFIED Requirements

### Requirement: Discogs references only
A send request SHALL contain exactly:
- a Discogs or Bandcamp page address, a Juno, Beatport, Traxsource or Bleep send (see `juno-intake`, `beatport-intake`, `traxsource-intake`, `bleep-intake`), a YouTube or SoundCloud mix's address (see `mix-tracklists`), or a tracklist send (see `mix-tracklists`);
- a mode (Play, Enqueue or Crate);
- a crate name of 1 to 40 characters when the mode is Crate;
- the skip passed switch;
- for a Bandcamp page only, optionally, the page's title, at most 200 characters, used only to name the label.

A `vinyl_only` field, which older extensions send, SHALL be accepted and ignored. Its body SHALL be at most 16 KB, except a store send's (Juno, Beatport, Traxsource or Bleep), which SHALL be at most 1 MiB, and a tracklist send's, which SHALL be at most 256 KB and SHALL be checked as `mix-tracklists` says and answered within 100 ms. A store send SHALL be checked as its store's spec says before anything happens, and SHALL be answered within 100 ms like any send. The address SHALL be checked as a supported Discogs page, or as a Bandcamp page passing the check in `bandcamp-intake`, before anything happens. A request with anything else (a file path, another kind of address, an unknown field) SHALL be refused, and nothing SHALL be added.

Seller pages:
- A request whose address is a seller's page SHALL add or refresh that seller (see `seller-crates`), whatever its mode.
- Its answer SHALL name the seller and say whether it was added or refreshed.
- The app SHALL then ask the system to bring its window to the front, or, where the system refuses, to draw the user's attention to it.

Label pages, Discogs or Bandcamp:
- A request whose address is a label's page SHALL follow or refresh that label (see `label-crates`), whatever its mode.
- Its answer SHALL name the label and say whether it was added, merged into a followed label, or refreshed.
- The app SHALL NOT come to the front.

A Bandcamp album or track SHALL be handled like a Discogs release (see `bandcamp-intake`). A store send SHALL be handled like a Discogs release, whatever page it came from, and SHALL NOT bring the app to the front.

#### Scenario: Not a Discogs page
- **WHEN** a paired extension sends https://example.com/track.mp3
- **THEN** the request is refused as unsupported, and no crate changes

#### Scenario: Label sent
- **WHEN** a paired extension sends a label page with mode Enqueue, and the label isn't followed
- **THEN** the label is followed and its crate fills under LABELS, no tracks are added to the shown crate, the answer says "Added label Siesta Records", and the app stays behind the browser

#### Scenario: Older extension
- **WHEN** a paired extension sends a label page with `vinyl_only: true`
- **THEN** the send is accepted, and the label's crate gets its records in every format

#### Scenario: Seller sent
- **WHEN** a paired extension sends `https://www.discogs.com/seller/decks.de/profile` with mode Enqueue, and decks.de isn't in the list
- **THEN** decks.de is added to TOP SELLERS, no tracks are added to the shown crate, the answer says "Added seller decks.de", and the app asks to come to the front

#### Scenario: Bandcamp album sent
- **WHEN** a paired extension sends `https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep` with mode Enqueue
- **THEN** its tracks are merged into the shown crate (or into the label crate following that Bandcamp), and the answer names the album

#### Scenario: Bandcamp label merged
- **WHEN** a paired extension sends `https://analogicalforce.bandcamp.com/music`, and "Label: Analogical Force" follows the Discogs label
- **THEN** the answer says "Merged into Label: Analogical Force", and the app stays behind the browser

#### Scenario: Look-alike host
- **WHEN** a paired extension sends `https://bandcamp.com.evil.net/album/x`
- **THEN** the request is refused as unsupported, and yt-dlp is not run

#### Scenario: Juno send
- **WHEN** a paired extension sends a Juno send of 20 records with mode Enqueue
- **THEN** the answer arrives within 100 ms saying 20 records were added, and their tracks are merged into the shown crate

#### Scenario: Beatport chart sent
- **WHEN** a paired extension sends a Beatport send of 100 tracks with mode Enqueue
- **THEN** the answer arrives within 100 ms saying 100 tracks were added, and they are merged into the shown crate

#### Scenario: Juno send too big
- **WHEN** a paired extension sends a Juno send of 1.5 MiB
- **THEN** it is refused, and no crate changes

#### Scenario: Large body that isn't Juno
- **WHEN** a paired extension sends a Discogs address in a 40 KB body
- **THEN** it is refused as too large

#### Scenario: Bleep send
- **WHEN** a paired extension sends a Bleep send with 3 valid records and mode Enqueue
- **THEN** it is checked as `bleep-intake` says, answered within 100 ms, its tracks are merged into the shown crate, and the app stays behind the browser

#### Scenario: Tracklist send
- **WHEN** a paired extension sends a 1001tracklists tracklist of 18 tracks, one of them with a time past 12 hours
- **THEN** the answer arrives within 100 ms saying 18 tracks were sent and 1 refused, and a Mix crate fills with the 18

#### Scenario: A mix's page sent
- **WHEN** a paired extension sends `https://soundcloud.com/tracklistings3-0/tracklistings-mixtape-781?in=toast3r/sets/x` with mode Play
- **THEN** the answer says the mix's tracklist is being read, and the player reads it and plays its first track

