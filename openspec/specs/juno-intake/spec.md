# juno-intake Specification

## Purpose
Juno Records as a third source beside Discogs and Bandcamp: records the browser extension reads from the Juno page the user has open (the app never fetches Juno pages), merged into crates without doubles, played from Juno's 2-minute samples, linked to Discogs when exactly one release fits, with the stock and price last seen.
## Requirements
### Requirement: Juno sends come from the browser only
Juno records SHALL reach the app only as a Juno send from the paired browser extension (see `browser-bridge`), built from the page the user has open. The app SHALL NOT fetch any page of juno.co.uk, SHALL NOT search Juno, and SHALL NOT pass any Juno address to yt-dlp. A Juno address pasted into the app SHALL be refused with a message saying to send it from the browser extension, and nothing SHALL be added.

#### Scenario: Pasted Juno address
- **WHEN** the user pastes `https://www.juno.co.uk/products/0-mika-vainio-rontgen-vinyl/1178909-01/`
- **THEN** the main window says Juno pages are sent from the browser extension, no crate changes, and no request is made to juno.co.uk

#### Scenario: No lookups on Juno
- **WHEN** a Discogs track becomes "not found" and its catalogue number is stocked at Juno
- **THEN** no request is made to juno.co.uk for it

### Requirement: What a Juno send holds
A Juno send SHALL hold the page's address and 1 to 500 records. Each record SHALL have:
- a product id matching `[0-9]{1,9}-[0-9]{2}` and its page's name part matching `[a-z0-9-]{1,200}`;
- an artist and a title (1 to 200 characters each);
- optionally a label (≤ 200), a catalogue number (≤ 40), a format (≤ 20), a stock state (in stock, out of stock, pre-order), a price as shown (≤ 20) and a cover image name matching `[A-Za-z0-9-]{1,40}\.jpg`;
- 1 to 60 tracks, at most 5,000 in the whole send, each with a side, a number from 1 to 99, a title and an artist (≤ 200 each, possibly empty), a length from 0 to 7,200 s, and a sample (its side and track in the sample's file name) matching `[0-9]{2}-[0-9]{2}`.

Text SHALL hold no control characters. A record breaking any rule SHALL be refused alone, and a send with no record left SHALL be refused whole. The answer SHALL say how many records were added and how many refused. Product, sample and cover addresses SHALL be rebuilt by the app from these ids, and no address sent by the extension SHALL ever be fetched.

#### Scenario: A product page
- **WHEN** the extension sends product 1178909-01, "0 aka Mika Vainio - Rontgen (reissue)", label Sahko Finland, catalogue SAHKO 00012, in stock at €14.78, with tracks 1.1 "Rontgen" (5:01, sample 01-01) and 2.1 "Track 2" (7:30, sample 02-01)
- **THEN** the send is accepted with 1 record and 0 refused

#### Scenario: A bad record among good ones
- **WHEN** a send holds 20 records and one has the product id `1178909-01/../x`
- **THEN** 19 records are added, and the answer says 1 was refused

#### Scenario: Too many
- **WHEN** a send holds 501 records
- **THEN** it is refused whole, and no crate changes

### Requirement: Juno tracks into a crate
Each track of a Juno send SHALL become an entry titled "Artist - Title". The artist is the track's own when given, else the record's. The title is the track's, or the record's title when the track's is empty. Each entry SHALL carry its side and number, its length, the record's catalogue number, label, format and cover, and the Juno product. The tracks SHALL be merged into the target crate as a Bandcamp album is (see `bandcamp-intake`, Merging into a crate): matched by normalized artist and title, narrowed by catalogue number. A match that plays is skipped, a match that is unavailable gets the Juno sample, and the rest are added, honouring skip passed. Juno sends SHALL use the modes of Discogs sends (Play, Enqueue, Crate). The main window SHALL say how many tracks were added, fixed and skipped. Adding SHALL take less than 50 ms for a 500-record send and SHALL cause zero underruns.

#### Scenario: Enqueue a product page
- **WHEN** product 1178909-01 is enqueued while crate "Friday" is shown and empty
- **THEN** "Friday" holds "0 aka Mika Vainio - Rontgen" (1.1, 5:01) and "0 aka Mika Vainio - Track 2" (2.1, 7:30), each from Juno, catalogue SAHKO 00012

#### Scenario: A listing item with one sample
- **WHEN** a genre page item "Two Shell - Infinite Now" is sent with one track whose title is empty
- **THEN** one entry "Two Shell - Infinite Now" is added, playing that sample

#### Scenario: Fixes a failed track
- **WHEN** a crate holds "Two Shell - Infinite Now" from Discogs marked "not found", with the same catalogue number as the Juno record sent
- **THEN** that entry gets the Juno sample and plays, and no second entry is added

#### Scenario: Big page
- **WHEN** a 500-record listing is enqueued while a track plays
- **THEN** the entries appear within 50 ms of the send being accepted, and playback has zero underruns

### Requirement: Juno samples
A Juno entry's preview SHALL be its sample, downloaded with one HTTPS request to `https://mp3.juno.co.uk/MP3/SF‹product›-‹sample›.mp3`, rebuilt from the entry's key, without yt-dlp. It SHALL follow the download-ahead window, slots, cache and preparation rules of every preview (see `preview-fetch`). The request SHALL name the app honestly in its user agent. A redirect to a host other than `mp3.juno.co.uk` or `www.juno.co.uk`, an answer that isn't `audio/mpeg`, a body over 10 MB, or one that doesn't start as an MP3 SHALL fail that try. Download all tracks SHALL skip Juno entries.

#### Scenario: Three ahead
- **WHEN** entry 3 of a 50-entry Juno crate plays
- **THEN** the samples of entries 4, 5 and 6 are downloaded or downloading, and no other Juno sample is requested

#### Scenario: Not audio
- **WHEN** the sample request is answered with an HTML page
- **THEN** the file is not kept, and the try counts as described in Juno limiting

#### Scenario: Download all tracks
- **WHEN** the user chooses Download all tracks on a label crate holding 40 YouTube and 10 Juno entries
- **THEN** only the 40 YouTube previews are downloaded

### Requirement: Juno limiting
When Juno answers a sample request with HTTP 429, HTTP 403 or an HTML page, it SHALL be handled as YouTube limiting is (see `preview-fetch`): the entry stays waiting and is not marked failed, Juno downloads pause with the 10 → 20 → 40 → 60 minute waits, and other sources go on. The main window SHALL say once per period "Juno is refusing downloads: trying again in ‹N› min", and tooltips SHALL say "waiting for Juno". A 404 SHALL count as a failed try, and two SHALL make the entry "clip failed".

#### Scenario: Refused
- **WHEN** Juno answers a sample request with 403
- **THEN** Juno entries keep waiting with "waiting for Juno", YouTube and Bandcamp downloads go on, and Juno is tried again after 10 minutes

#### Scenario: Gone
- **WHEN** a sample is answered with 404 twice
- **THEN** its entry is "clip failed"

### Requirement: Linking to Discogs
With a Discogs token, each Juno record not linked before SHALL be looked up with one Discogs database search by artist, title and catalogue number, behind other Discogs work and within Discogs' rate limit. It SHALL be linked only when exactly one release fits (as for Bandcamp collection items, see `bandcamp-collection`). Its entries SHALL then take the release, master, year and formats, and show ★ and owned marks like Discogs entries, keeping their Juno audio. A product's link SHALL be remembered across restarts and SHALL NOT be searched again. Without a token, or when no single release fits, the entries SHALL stay unlinked: Y, I and the Discogs menu items SHALL be disabled on them, and the main window SHALL say the record isn't linked to Discogs.

#### Scenario: Linked
- **WHEN** product 1178909-01 (SAHKO 00012) is sent with a token, and Discogs has exactly one release with that artist, title and catalogue number
- **THEN** its entries take that release, and pressing Y while one plays adds it to the wantlist

#### Scenario: Ambiguous
- **WHEN** two Discogs releases fit
- **THEN** the entries stay unlinked, and Y says the record isn't linked to Discogs

#### Scenario: Sent again
- **WHEN** the same product is sent a week later
- **THEN** no Discogs search is made for it

#### Scenario: A Discogs record comes after
- **WHEN** a crate holds Juno-only entries of SAHKO 00012, and the Discogs release SAHKO 00012 is sent into it
- **THEN** the Juno entries take the release in place, keep playing from Juno, and only tracks no entry matches are added

### Requirement: Stock note and Open on Juno
The stock state and price of each record SHALL be remembered with the date of the send that carried them, and each new send SHALL replace them. A Juno entry's tooltip SHALL show "Juno: in stock, €14.78 (seen 14 Oct)", or "out of stock" or "pre-order". The tooltip of a Discogs entry whose release is linked to a Juno product SHALL show the same line. A note older than 30 days SHALL add "may have changed". Both SHALL offer Open on Juno in their menu, opening the product's page in the default browser. The app SHALL never check stock on its own.

#### Scenario: Note on a Discogs entry
- **WHEN** release 123456 is linked to Juno product 1178909-01, seen in stock at €14.78 on 14 Oct, and a crate holds a Discogs entry of release 123456
- **THEN** its tooltip shows "Juno: in stock, €14.78 (seen 14 Oct)", and its menu offers Open on Juno

#### Scenario: Old note
- **WHEN** the note was seen 40 days ago
- **THEN** the tooltip adds "may have changed"

#### Scenario: Open on Juno
- **WHEN** the user chooses Open on Juno on a Juno entry of product 1178909-01
- **THEN** the default browser opens `https://www.juno.co.uk/products/0-mika-vainio-rontgen-vinyl/1178909-01/`

