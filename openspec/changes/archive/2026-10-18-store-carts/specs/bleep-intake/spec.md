## MODIFIED Requirements

### Requirement: What a Bleep send holds
A Bleep send SHALL hold the page's address (on bleep.com) and 1 to 500 records. Each record SHALL have:
- a release id of 1 to 10 digits and its page's name part matching `[a-z0-9-]{1,200}` (`/release/‹id›-‹name›`);
- an artist and a title (1 to 200 characters each);
- optionally a label (≤ 200), a catalogue number (≤ 40), and a cover image name matching `[0-9]{1,10}(-[0-9]{1,3})?\.jpg`;
- 0 to 8 formats, each a name as shown ("LP", "Download", ≤ 30), optionally a price as shown (≤ 20), and optionally its product id on Bleep (1 to 10 digits, the `ReleaseFormat` id its buy button carries; see `store-carts`);
- 0 to 60 tracks (a release page shows them; a record in a list shows none), at most 5,000 in the whole send, each with a track id of 1 to 10 digits, a number from 1 to 99, a title (1 to 200), optionally an artist (≤ 200) and a length from 0 to 7,200 s.

Text SHALL hold no control characters. A record breaking any rule SHALL be refused alone, and a send with no record left SHALL be refused whole. The answer SHALL say how many records were added and how many refused. Release and cover addresses SHALL be rebuilt by the app from these ids (`https://bleep.com/release/‹id›-‹name›`, `https://d1rgjmn2wmqeif.cloudfront.net/r/s/‹image›`), and no address sent by the extension SHALL ever be fetched.

#### Scenario: A release page
- **WHEN** the extension sends release 100168 `bibio-zen-drumsdada-drums`, "Bibio - Zen Drums/Dada Drums", label Warp Records, catalogue WAP419, formats Download at 2.49 € (product 277377), and tracks 1 "Zen Drums" (6:37) and 2 "Dada Drums"
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
