# browser-bridge Specification

## Purpose
Lets a browser extension send Discogs pages to the running player safely: only from this computer, only from paired browsers, and only Discogs addresses.
## Requirements
### Requirement: Local only
The bridge SHALL listen only on the loopback address (127.0.0.1), on port 47800 by default, which can be changed in Options ▸ Browser…. It SHALL start only after the window is interactive, so launch stays within 300 ms. If the port is taken, the app SHALL start normally without the bridge and say so in Options ▸ Browser….

#### Scenario: Not reachable from the network
- **WHEN** another computer on the same network connects to the player's computer on the bridge port
- **THEN** the connection is refused

#### Scenario: Port taken
- **WHEN** another program already uses port 47800
- **THEN** the app starts normally, and Options ▸ Browser… says the port is taken

### Requirement: Pairing
Options ▸ Browser… SHALL show a 6-digit pairing code that is valid for 2 minutes. A request presenting that code SHALL receive a new random key of at least 128 bits, and the code SHALL then stop working. After 5 wrong codes, every pairing attempt SHALL be refused for 1 minute. The app SHALL store only a hash of each key. Forget browsers SHALL revoke every key.

#### Scenario: Pair once
- **WHEN** the extension sends the code shown in Options ▸ Browser… within 2 minutes
- **THEN** it receives a key, the dialog says a browser was paired, and the same code no longer works

#### Scenario: Guessing is stopped
- **WHEN** 5 wrong codes are sent in a row
- **THEN** every pairing attempt is refused for 1 minute, even one with the right code

### Requirement: Only paired browsers
Every request other than pairing and a minimal introduction (the app's name and the bridge version) SHALL carry a valid key in a custom header, and SHALL be refused otherwise. The bridge SHALL refuse requests coming from web pages: requests whose origin is a web page, requests addressed to any host name other than 127.0.0.1 or localhost, and cross-origin preflight requests. Its responses SHALL carry no header that allows other origins.

#### Scenario: A web page tries
- **WHEN** a script on a website sends a request to the bridge
- **THEN** the request is refused and no crate changes

#### Scenario: Revoked key
- **WHEN** the extension sends a request with a key revoked by Forget browsers
- **THEN** the request is refused, and the extension asks to pair again

### Requirement: Discogs references only
A send request SHALL contain exactly:
- a Discogs or Bandcamp page address;
- a mode (Play, Enqueue or Crate);
- a crate name of 1 to 40 characters when the mode is Crate;
- the skip passed switch;
- for a Bandcamp page only, optionally, the page's title, at most 200 characters, used only to name the label.

A `vinyl_only` field, which older extensions send, SHALL be accepted and ignored. Its body SHALL be at most 16 KB. The address SHALL be checked as a supported Discogs page, or as a Bandcamp page passing the check in `bandcamp-intake`, before anything happens. A request with anything else (a file path, another kind of address, an unknown field) SHALL be refused, and nothing SHALL be added.

Seller pages:
- A request whose address is a seller's page SHALL add or refresh that seller (see `seller-crates`), whatever its mode.
- Its answer SHALL name the seller and say whether it was added or refreshed.
- The app SHALL then ask the system to bring its window to the front, or, where the system refuses, to draw the user's attention to it.

Label pages, Discogs or Bandcamp:
- A request whose address is a label's page SHALL follow or refresh that label (see `label-crates`), whatever its mode.
- Its answer SHALL name the label and say whether it was added, merged into a followed label, or refreshed.
- The app SHALL NOT come to the front.

A Bandcamp album or track SHALL be handled like a Discogs release (see `bandcamp-intake`).

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

### Requirement: Fast answers
The bridge SHALL answer every request within 100 ms, without waiting for Discogs. A send SHALL be accepted and queued, and its expansion SHALL continue in the background.

#### Scenario: Big label
- **WHEN** a paired extension sends a label with 312 releases
- **THEN** the answer arrives within 100 ms, and the crate fills in afterwards

### Requirement: Crates and status
A paired extension SHALL be able to read the list of crate names, with the shown and the playing crate marked, and the player's status: its name, what is playing (artist, title and crate), and the progress of sends that are still running.

#### Scenario: Crate list
- **WHEN** the extension asks for the crates
- **THEN** it receives every crate's name, with the shown crate and the playing crate marked

#### Scenario: Status
- **WHEN** the extension asks for the status while a label is being expanded and a track plays
- **THEN** it receives the playing track's artist, title and crate, and the label's progress (for example 120 of 312 releases)

### Requirement: Owned check
A paired extension SHALL be able to ask whether the record of a Discogs release, master release or marketplace item page is in the user's collection, sending only the page's address. The answer SHALL be one of:
- owned (this pressing);
- owned in another pressing (naming its catalog number and year);
- not owned;
- checking;
- no token (so the extension can say what's missing);
- unknown (no collection yet).

It SHALL be given from the cached collection without waiting for the UI or for Discogs, and SHALL cause no request to Discogs for a release or master release page. For a marketplace item whose release isn't known yet, the answer SHALL be "checking", and the item's release SHALL be looked up once, then remembered. An unpaired request SHALL be refused.

#### Scenario: Owned release
- **WHEN** a paired extension asks about https://www.discogs.com/release/123456 and release 123456 is in the collection
- **THEN** the answer is "owned", within 50 ms, and no request is sent to Discogs

#### Scenario: New marketplace item
- **WHEN** a paired extension asks about a marketplace item seen for the first time
- **THEN** the answer is "checking", one lookup is made, and asking again after it completes gives the owned answer without another request

#### Scenario: No token
- **WHEN** no Discogs token is set
- **THEN** the answer is "no token"

