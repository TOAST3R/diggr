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
A send request SHALL contain exactly: a Discogs page address, a mode (Play, Enqueue or Crate), a crate name of 1 to 40 characters when the mode is Crate, and the skip passed switch. A `vinyl_only` field, which older extensions send, SHALL be accepted and ignored. Its body SHALL be at most 16 KB. The address SHALL be checked as a supported Discogs page before anything happens. A request with anything else (a file path, another kind of address, an unknown field) SHALL be refused, and nothing SHALL be added.

#### Scenario: Not a Discogs page
- **WHEN** a paired extension sends https://example.com/track.mp3
- **THEN** the request is refused as unsupported, and no crate changes

#### Scenario: Label sent
- **WHEN** a paired extension sends a label page with mode Enqueue
- **THEN** the label is sent exactly as if it had been pasted, and the answer names the label and the target crate

#### Scenario: Older extension
- **WHEN** a paired extension sends a label page with `vinyl_only: true`
- **THEN** the send is accepted, and the crate gets the label's records in every format

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

