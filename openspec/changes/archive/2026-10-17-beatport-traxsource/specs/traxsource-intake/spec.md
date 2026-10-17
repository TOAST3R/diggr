## ADDED Requirements

### Requirement: Traxsource sends come from the browser only
Traxsource tracks SHALL reach the app only as a Traxsource send from the paired browser extension (see `browser-bridge`), built from the page the user has open. The app SHALL NOT fetch any page or file of traxsource.com, and SHALL NOT pass any Traxsource address to yt-dlp. A Traxsource address pasted into the app SHALL be refused with a message saying to send it from the browser extension, and nothing SHALL be added.

#### Scenario: Pasted Traxsource address
- **WHEN** the user pastes `https://www.traxsource.com/title/2345678/infinite-now`
- **THEN** the main window says Traxsource pages are sent from the browser extension, no crate changes, and no request is made to traxsource.com

### Requirement: What a Traxsource send holds
A Traxsource send SHALL hold the page's address and 1 to 500 tracks. Each track SHALL have a track id of 1 to 10 digits and its page's name part matching `[a-z0-9-]{1,200}`; one or more artist names and a title (1 to 200 characters each); and optionally a version (≤ 200), a BPM (a whole number from 40 to 250), a key (≤ 20, kept as a Camelot code when it can be converted), a length from 0 to 7,200 s, a release id (1 to 10 digits) with its title, label (≤ 200) and catalogue number (≤ 40), and a price as shown (≤ 20). Text SHALL hold no control characters. A track breaking any rule SHALL be refused alone, and a send with no track left SHALL be refused whole. The answer SHALL say how many tracks were added and how many refused. Track addresses SHALL be rebuilt by the app from these ids.

#### Scenario: A chart page
- **WHEN** the extension sends a Traxsource top 100 with 100 valid tracks
- **THEN** the send is accepted with 100 tracks and 0 refused

### Requirement: Traxsource tracks into a crate
Each track SHALL become an entry titled "Artist - Title (Version)", leaving out a version named "Original Mix", carrying its length, catalogue number, label, BPM and key, and the Traxsource track. Tracks SHALL be merged into the target crate as Beatport tracks are (see `beatport-intake`). An entry with no other audio SHALL wait to be searched: its preview SHALL be found by the preview search (see `preview-search`) from its artist, title with version and length, near the playhead, as for a Discogs track with no clip. Its badge SHALL then be YT, and its tooltip SHALL say "Source: YouTube (found for Traxsource)". Traxsource sends SHALL use the modes of Discogs sends. Adding SHALL take less than 50 ms for a 500-track send and SHALL cause zero underruns.

#### Scenario: Previewed by search
- **WHEN** "Kerri Chandler - Rain (Extended Mix)", 7:12, is sent from Traxsource and comes near the playhead
- **THEN** it is searched for, the matching upload plays, and its tooltip says "Source: YouTube (found for Traxsource)"

#### Scenario: Not found
- **WHEN** no usable upload is found for a Traxsource track
- **THEN** it is "not found", keeps its BPM, key and price note, and still offers Open on Traxsource

### Requirement: Traxsource BPM, key, linking, price and Open on Traxsource
A Traxsource entry SHALL show the BPM Traxsource gives until its audio is analysed, as for Beatport (see `beatport-intake`, Store BPM and key), and its key in its tooltip. With a Discogs token it SHALL be linked to Discogs as a Beatport release is. Its price SHALL be remembered and shown as "Traxsource: $2.49 (seen 16 Oct)", with "may have changed" after 30 days. Its menu SHALL offer Open on Traxsource, opening `https://www.traxsource.com/track/‹id›/‹name›`.

#### Scenario: Open on Traxsource
- **WHEN** the user chooses Open on Traxsource on the entry of track 12815016 `rain`
- **THEN** the default browser opens `https://www.traxsource.com/track/12815016/rain`
