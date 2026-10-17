# beatport-intake Specification

## Purpose
Beatport pages sent from the browser extension become crates: the tracks a page shows, with the store's BPM and key, played from Beatport's 2-minute samples, linked to Discogs, with a price note and Open on Beatport.
## Requirements
### Requirement: Beatport sends come from the browser only
Beatport tracks SHALL reach the app only as a Beatport send from the paired browser extension (see `browser-bridge`), built from the page the user has open. The app SHALL NOT fetch any page of beatport.com, SHALL NOT search Beatport, and SHALL NOT pass any Beatport address to yt-dlp. A Beatport address pasted into the app SHALL be refused with a message saying to send it from the browser extension, and nothing SHALL be added.

#### Scenario: Pasted Beatport address
- **WHEN** the user pastes `https://www.beatport.com/release/infinite-now/4012345`
- **THEN** the main window says Beatport pages are sent from the browser extension, no crate changes, and no request is made to beatport.com

### Requirement: What a Beatport send holds
A Beatport send SHALL hold the page's address and 1 to 500 tracks. Each track SHALL have:
- a track id of 1 to 10 digits, and its page's name part matching `[a-z0-9-]{1,200}`;
- one or more artist names and a title (1 to 200 characters each), and optionally a mix name and remixer names (≤ 200 each);
- a sample id matching `[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}`;
- optionally: a BPM (a whole number from 40 to 250), a key as a Camelot code matching `(1[0-2]|[1-9])[AB]`, a length from 0 to 7,200 s, a release id (1 to 10 digits) with its name, label (≤ 200) and catalogue number (≤ 40), a price as shown (≤ 20) and an image id in the same form as the sample id.

Text SHALL hold no control characters. A track breaking any rule SHALL be refused alone, and a send with no track left SHALL be refused whole. The answer SHALL say how many tracks were added and how many refused. Track, sample and image addresses SHALL be rebuilt by the app from these ids, and no address sent by the extension SHALL ever be fetched.

#### Scenario: A release page
- **WHEN** the extension sends release 4012345 "Infinite Now" (label Shall Not Fade, catalogue SNF123) with two tracks, each with a sample id, 128 BPM and key 8A
- **THEN** the send is accepted with 2 tracks and 0 refused

#### Scenario: A bad track among good ones
- **WHEN** a send holds 100 chart tracks and one has the sample id `../../etc`
- **THEN** 99 tracks are added, and the answer says 1 was refused

#### Scenario: Too many
- **WHEN** a send holds 501 tracks
- **THEN** it is refused whole, and no crate changes

### Requirement: Beatport tracks into a crate
Each track SHALL become an entry titled "Artist - Title (Mix)", leaving out a mix named "Original Mix", with the artists joined by ", ". Each entry SHALL carry its length, the release's catalogue number, label and image, the track's BPM and key, and the Beatport track. Tracks SHALL be merged into the target crate as Juno tracks are (see `juno-intake`): matched by normalized artist and title, narrowed by catalogue number. A match that plays is skipped, a match that is unavailable gets the Beatport sample, and the rest are added in the page's order, honouring skip passed. Beatport sends SHALL use the modes of Discogs sends (Play, Enqueue, Crate). The main window SHALL say how many tracks were added, fixed and skipped. Adding SHALL take less than 50 ms for a 500-track send and SHALL cause zero underruns.

#### Scenario: Enqueue a chart
- **WHEN** a 100-track Beatport chart is enqueued while crate "Friday" is shown and empty
- **THEN** "Friday" holds 100 entries in chart order, each showing its BPM before any preview is downloaded

#### Scenario: Fixes a failed track
- **WHEN** a crate holds "Two Shell - Infinite Now" from Discogs marked "not found", with the same catalogue number as the Beatport release sent
- **THEN** that entry gets the Beatport sample and plays, and no second entry is added

### Requirement: Beatport samples
A Beatport entry's preview SHALL be its sample, downloaded with one HTTPS request to `https://geo-samples.beatport.com/track/‹sample id›.LOFI.mp3`, rebuilt from the entry's key, without yt-dlp. It SHALL follow the download-ahead window, slots, cache and preparation rules of every preview (see `preview-fetch`). The request SHALL name the app honestly in its user agent. A redirect to a host outside `beatport.com` and its subdomains, an answer that isn't `audio/mpeg`, a body over 10 MB, or one that doesn't start as an MP3 SHALL fail that try. Download all tracks SHALL skip Beatport entries.

#### Scenario: Three ahead
- **WHEN** entry 3 of a 50-entry Beatport crate plays
- **THEN** the samples of entries 4, 5 and 6 are downloaded or downloading, and no other Beatport sample is requested

### Requirement: Beatport limiting
When Beatport answers a sample request with HTTP 429, HTTP 403 or an HTML page, it SHALL be handled as Juno limiting is (see `juno-intake`): the entry stays waiting, Beatport downloads pause with the 10 → 20 → 40 → 60 minute waits, and other sources go on. The main window SHALL say once per period "Beatport is refusing downloads: trying again in ‹N› min", and tooltips SHALL say "waiting for Beatport". A source that can't be reached SHALL pause as `preview-fetch` says. A 404 SHALL count as a failed try, and two SHALL make the entry "clip failed".

#### Scenario: Refused
- **WHEN** Beatport answers a sample request with 403
- **THEN** Beatport entries keep waiting with "waiting for Beatport", YouTube, Bandcamp and Juno downloads go on, and Beatport is tried again after 10 minutes

### Requirement: Store BPM and key
A Beatport entry SHALL show the BPM Beatport gives until its audio is analysed, and its tooltip SHALL say "128 BPM (from Beatport)" meanwhile. The key SHALL be kept with the entry, saved with the crate, and shown in its tooltip as "Key: 8A (A minor)".

#### Scenario: Before any download
- **WHEN** a Beatport track at 126 BPM, key 5A, is added and its preview hasn't downloaded
- **THEN** its row shows "(126 BPM)", the BPM filter 124–128 shows it, and its tooltip shows "Key: 5A (C minor)"

### Requirement: Beatport linking, price and Open on Beatport
With a Discogs token, each Beatport release not linked before SHALL be looked up and linked as a Juno product is (see `juno-intake`, Linking to Discogs), and its link remembered across restarts. Each record's price (a release's, or a lone track's) SHALL be remembered with the date of the send that carried it, each new send replacing it. The tooltip SHALL show "Beatport: $1.69 (seen 16 Oct)", adding "may have changed" after 30 days. The entry's menu SHALL offer Open on Beatport, opening `https://www.beatport.com/track/‹name›/‹id›` in the default browser. The app SHALL never check prices on its own.

#### Scenario: Open on Beatport
- **WHEN** the user chooses Open on Beatport on the entry of track 17654321 `infinite-now`
- **THEN** the default browser opens `https://www.beatport.com/track/infinite-now/17654321`

