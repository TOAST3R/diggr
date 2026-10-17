## ADDED Requirements

### Requirement: Bleep sends come from the browser only
Bleep records SHALL reach the app only as a Bleep send from the paired browser extension (see `browser-bridge`), built from the page the user has open. The app SHALL NOT fetch any page of bleep.com or bleep.co.uk, and SHALL NOT pass any Bleep address to yt-dlp. A Bleep address pasted into the app SHALL be refused with a message saying to send it from the browser extension, and nothing SHALL be added.

#### Scenario: Pasted Bleep address
- **WHEN** the user pastes a bleep.com release address
- **THEN** the main window says Bleep pages are sent from the browser extension, no crate changes, and no request is made to bleep.com

#### Scenario: Old domain
- **WHEN** the user pastes a bleep.co.uk address
- **THEN** it is refused the same way

### Requirement: What a Bleep send holds
A Bleep send SHALL hold the page's address (on bleep.com) and 1 to 500 records. Each record SHALL have:
- a release id of 1 to 10 digits and its page's name part matching `[a-z0-9-]{1,200}` (`/release/‹id›-‹name›`);
- an artist and a title (1 to 200 characters each);
- optionally a label (≤ 200), a catalogue number (≤ 40), and a cover image name matching `[0-9]{1,10}(-[0-9]{1,3})?\.jpg`;
- 0 to 8 formats, each a name as shown ("LP", "Download", ≤ 30) and optionally a price as shown (≤ 20);
- 0 to 60 tracks (a release page shows them; a record in a list shows none), at most 5,000 in the whole send, each with a track id of 1 to 10 digits, a number from 1 to 99, a title (1 to 200), optionally an artist (≤ 200) and a length from 0 to 7,200 s.

Text SHALL hold no control characters. A record breaking any rule SHALL be refused alone, and a send with no record left SHALL be refused whole. The answer SHALL say how many records were added and how many refused. Release and cover addresses SHALL be rebuilt by the app from these ids (`https://bleep.com/release/‹id›-‹name›`, `https://d1rgjmn2wmqeif.cloudfront.net/r/s/‹image›`), and no address sent by the extension SHALL ever be fetched.

#### Scenario: A release page
- **WHEN** the extension sends release 100168 `bibio-zen-drumsdada-drums`, "Bibio - Zen Drums/Dada Drums", label Warp Records, catalogue WAP419, formats Download at 2.49 €, and tracks 1 "Zen Drums" (6:37) and 2 "Dada Drums"
- **THEN** the send is accepted with 1 record and 0 refused

#### Scenario: A list page
- **WHEN** the extension sends a label page's 24 records, each with its id, name part, artist, title, label, catalogue number and cover, and no tracks
- **THEN** the send is accepted with 24 records

#### Scenario: A bad record among good ones
- **WHEN** a send holds 20 records and one has the release id `100168/../x`
- **THEN** 19 records are added, and the answer says 1 was refused

#### Scenario: Too many
- **WHEN** a send holds 501 records
- **THEN** it is refused whole, and no crate changes

### Requirement: Bleep tracks into a crate
Each track of a Bleep record SHALL become an entry titled "Artist - Title". The artist is the track's own when given, else the record's. Each entry SHALL carry its number, its length, the record's catalogue number, label, formats and cover, and the Bleep release. The tracks SHALL be merged into the target crate as Juno tracks are (see `juno-intake`): a match that already plays or waits is skipped, and the rest are added, honouring skip passed. A record with no tracks (sent from a list) SHALL become one entry "Artist - Title" with the record's title, standing for the whole record. Bleep sends SHALL use the modes of Discogs sends. The main window SHALL say how many tracks were added and skipped. Adding SHALL take less than 50 ms for a 500-record send and SHALL cause zero underruns.

#### Scenario: Enqueue a release page
- **WHEN** release 100168 is enqueued while crate "Friday" is shown and empty
- **THEN** "Friday" holds "Bibio - Zen Drums" (1, 6:37) and "Bibio - Dada Drums" (2), each from Bleep, catalogue WAP419

#### Scenario: A record from a list
- **WHEN** a list record "Russell Haswell - Deep Time" with no tracks is enqueued
- **THEN** one entry "Russell Haswell - Deep Time" is added, standing for the record

### Requirement: Bleep previews
Bleep's own previews SHALL NOT be fetched: Bleep issues their addresses per request, behind the challenge (design D3). A Bleep entry with no other audio SHALL wait to be searched, and its preview SHALL be found by the preview search (see `preview-search`) from its artist, title and length, near the playhead, as for a Traxsource track; its tooltip SHALL say "Source: YouTube (found for Bleep)". An entry standing for a whole record SHALL be searched by its artist and the record's title. A Bleep entry that can't be previewed SHALL still keep its price note and Open on Bleep.

#### Scenario: Played from search
- **WHEN** a Bleep track comes near the playhead
- **THEN** it is searched for, the matching upload plays, and its tooltip says "Source: YouTube (found for Bleep)"

#### Scenario: Not found
- **WHEN** no preview is found for a Bleep track
- **THEN** it is "not found", and its menu still offers Open on Bleep

### Requirement: Bleep linking, price note and Open on Bleep
With a Discogs token, each Bleep record not linked before SHALL be linked to Discogs as a Juno record is (see `juno-intake`, Linking to Discogs), and the link SHALL be remembered across restarts. When a record standing as one entry (sent from a list) is linked, that release's tracks SHALL be brought into the entry's crate as a Discogs release is, and the entry standing for the record SHALL be removed. The formats and their prices SHALL be remembered with the date of the send that carried them, each new send replacing them. A Bleep entry's tooltip, and that of a Discogs entry whose release is linked to the record, SHALL show "Bleep: LP 22.99 €, Download 7.99 € (seen 18 Oct)"; a note older than 30 days SHALL add "may have changed". Both SHALL offer Open on Bleep in their menu, opening the release's page in the default browser. The app SHALL never check stock on its own.

#### Scenario: Note on a Discogs entry
- **WHEN** a Discogs release is linked to a Bleep record seen with LP at 22.99 € and Download at 7.99 € on 18 Oct
- **THEN** a Discogs entry of that release shows "Bleep: LP 22.99 €, Download 7.99 € (seen 18 Oct)" and its menu offers Open on Bleep

#### Scenario: A list record gets its tracks
- **WHEN** the entry "Russell Haswell - Deep Time", sent from a list, is linked to the one Discogs release that fits
- **THEN** that release's tracks are added to its crate, and the entry standing for the record is removed

#### Scenario: Open on Bleep
- **WHEN** the user chooses Open on Bleep on a Bleep entry
- **THEN** the default browser opens the release's page on bleep.com, rebuilt from its id and name part
