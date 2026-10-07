# discogs-intake Specification

## Purpose
Turns a Discogs page (a release, master release, artist, label, wantlist or list) into tracks in a crate. It respects Discogs' rate limit, caches what it learns, and never touches the playback path.
## Requirements
### Requirement: Discogs account
The user SHALL be able to enter a Discogs personal access token in Options ▸ Discogs…. The app SHALL check the token with Discogs and show the account's username, or say that the token was rejected. The token SHALL be stored only in the user's config folder, in a file only the user can read, and SHALL never be shown in full or written to a log. Without a token, pages SHALL still be expanded, at the unauthenticated rate limit, and actions that need an account (adding to the wantlist, reading a private wantlist) SHALL say that a token is needed.

#### Scenario: Valid token
- **WHEN** the user enters a valid token
- **THEN** Options ▸ Discogs… shows "Connected as ‹username›", and later requests use the authenticated rate limit

#### Scenario: Rejected token
- **WHEN** Discogs rejects the token
- **THEN** the dialog says the token was rejected, and the token is not saved

### Requirement: Supported pages
The app SHALL accept Discogs web addresses of a release, a master release, an artist, a label, a user's wantlist, a user's collection (`/user/‹name›/collection`), a user list and a marketplace listing (`/shop/item/…` or `/sell/item/…`), with or without `www.`, a language prefix (such as `/de/`), the name part after the id, a query string or a fragment. A marketplace listing SHALL be treated as the release it sells, looked up with at most one request to Discogs (none when that listing was looked up before). Any other address SHALL be refused with a message listing the supported kinds of page, and nothing SHALL be added.

#### Scenario: Language prefix
- **WHEN** the address is `https://www.discogs.com/de/label/12345-Lowtide-Tapes`
- **THEN** it is recognised as label 12345

#### Scenario: No name part
- **WHEN** the address is `https://discogs.com/release/123456`
- **THEN** it is recognised as release 123456

#### Scenario: Marketplace listing
- **WHEN** the address is `https://www.discogs.com/shop/item/3923678974`, a listing of release 123456
- **THEN** release 123456 is sent, exactly as if its release page had been sent

#### Scenario: Listing gone
- **WHEN** the address names a listing that Discogs doesn't know
- **THEN** the main window says the page wasn't found, and no crate changes

#### Scenario: A collection
- **WHEN** the address is `https://www.discogs.com/user/digger/collection`
- **THEN** it is recognised as digger's collection, and sending it adds one entry per clip of every record in it, like a wantlist

#### Scenario: Unsupported page
- **WHEN** the address is a Discogs forum thread
- **THEN** a message lists the supported kinds of page, and no crate changes

### Requirement: Sending a page
A page SHALL be sent in one of three modes:
- Enqueue: its tracks are added to the end of the shown crate.
- Play: a new crate is named after the page (for example "Label: Lowtide Tapes"), shown, and its first track plays as soon as its preview is ready. When a crate with that name exists, it is reused.
- Crate: its tracks are added to the named crate, which is created if it doesn't exist. A crate created this way SHALL be shown; sending to a crate that already exists SHALL NOT change the shown crate.

Whenever a send shows a crate, the playlist SHALL be opened if it was hidden.

Pasting a Discogs address in the player window (Cmd+V, or Ctrl+V on Linux and Windows) SHALL send it with Enqueue; text that isn't a Discogs address SHALL be ignored. Each send SHALL be acknowledged in the main window with the page's name, and its progress SHALL be shown until it finishes (for example "Lowtide Tapes: 120 of 312 releases").

#### Scenario: Paste a label
- **WHEN** the user copies a label's Discogs address and presses Cmd+V in the player window
- **THEN** the label's tracks are added to the end of the shown crate, and the main window names the label

#### Scenario: Play as a new crate
- **WHEN** a label page is sent with Play
- **THEN** a crate "Label: Lowtide Tapes" is created and shown, and its first track plays as soon as its preview is ready

#### Scenario: Send to a new crate
- **WHEN** an artist page is sent to crate "Friday", which doesn't exist yet, while the playlist is hidden
- **THEN** a crate "Friday" is created with the artist's tracks, shown, and the playlist opens

#### Scenario: Send to an existing crate
- **WHEN** an artist page is sent to crate "Friday", which exists, while crate "Playlist" is shown
- **THEN** the tracks are added to "Friday", and "Playlist" stays on screen

### Requirement: Expanding a page into tracks
Each usable clip of a record SHALL become one entry. The entry is titled "Artist - Title" from the record's tracklist when the clip matches a track, and from the clip's own title otherwise. It carries its origin: page, release, master, label, catalog number, year, side and clip. The records of a page SHALL be:
- a release: that release;
- a master release: the master's own clips, with the label, catalog number, year and for-sale numbers of its main release;
- an artist: the releases where the artist has the main role or a remix credit, oldest first; for a remix credit, only the clips whose title names the artist;
- a label: all its releases, in the order Discogs lists them;
- a wantlist: every release in it;
- a list: its releases and master releases, in list order.

A clip that the target crate already holds SHALL NOT be added again. A record with no usable clip SHALL become one entry per track of its tracklist, waiting to be searched (see `preview-search`). A record with no usable clip and no tracklist SHALL appear once, as an unavailable entry marked "no clip". A record that a page's listing gives more than once (a label credited several times on a release) SHALL be listed, fetched and counted once.

#### Scenario: Listed three times
- **WHEN** a label's listing gives release 38583846 three times and it has no clip
- **THEN** the crate holds one "no clip" entry for it, and the progress counts it once

#### Scenario: Release with three clips
- **WHEN** a release with a four-track tracklist and three clips is sent
- **THEN** three entries appear, each titled after its matching track, with its side, and all three carry the release's catalog number and year

#### Scenario: Remix credit
- **WHEN** an artist is credited with one remix on a release that has four clips
- **THEN** only the clip whose title names that artist is added

#### Scenario: No clip
- **WHEN** one release of a label has no clips
- **THEN** one entry for that release appears, unavailable, marked "no clip"

#### Scenario: Sent twice
- **WHEN** the same release is sent twice to the same crate
- **THEN** its clips appear only once

#### Scenario: No clip, but a tracklist
- **WHEN** release 38583846, with five tracks and no clip, is expanded
- **THEN** the crate holds five entries, "The 89th Passenger - Paper Wings" to "The 89th Passenger - Analog Serenade", with their sides and durations, waiting to be searched

### Requirement: Filters
Every send SHALL apply the skip passed filter, on by default: clips the user has passed on are left out. Its default can be changed in Options ▸ Discogs…, and each send can override it. Pasting SHALL use the default. A send SHALL keep records in every format: vinyl, digital files, CD, cassette and others.

#### Scenario: Every format
- **WHEN** a label with 10 vinyl releases and 5 CD-only releases is sent
- **THEN** all 15 releases add entries

#### Scenario: Skip passed
- **WHEN** a label is sent again after the user passed on 20 of its clips
- **THEN** those 20 clips are not added

### Requirement: Progressive filling
Each record SHALL appear as one waiting entry marked "listed" as soon as the page of the listing that contains it arrives, before any record's details are fetched. That entry SHALL be replaced by the record's clip entries, or by its "no clip" entry, when its details arrive. Details SHALL be fetched in crate order, starting from the playing entry, or from the selected entry when nothing in that crate is playing. A send that is interrupted by quitting SHALL resume when its crate is next shown or played.

#### Scenario: Big label
- **WHEN** a label with 312 releases is sent
- **THEN** its first 100 releases appear as "listed" entries as soon as the first page of the listing arrives, before any release's details are fetched

#### Scenario: Near the playhead first
- **WHEN** the user is playing entry 40 of a crate whose releases are still being expanded
- **THEN** the details of releases from entry 40 onwards are fetched before those of earlier entries

#### Scenario: Resume after quitting
- **WHEN** the app is quit halfway through expanding a label, then started again and that crate is shown
- **THEN** the expansion continues from where it stopped, without adding any entry twice

### Requirement: Rate limit and cache
The app SHALL never send Discogs more than 60 requests in any 60-second window with a token, or 25 without one. It SHALL slow down further when Discogs reports few remaining requests, and wait and retry when Discogs says the limit was reached. Release, master, artist and label data SHALL be cached on disk and reused without a request. Marketplace numbers older than 24 hours SHALL be refreshed when their track starts playing.

#### Scenario: 800 releases
- **WHEN** a label with 800 releases is expanded with a token
- **THEN** no 60-second window contains more than 60 requests to Discogs

#### Scenario: Cached release
- **WHEN** a release whose data was fetched yesterday is sent again
- **THEN** its tracks appear without a new request for the release's data

### Requirement: For-sale snapshot
Each entry from a release SHALL carry the release's number of copies for sale and its lowest price, in the user's Discogs currency, and the time they were fetched.

#### Scenario: Snapshot on the entry
- **WHEN** a release with 6 copies for sale from €9.00 is expanded for a user whose Discogs currency is the euro
- **THEN** its entries carry 6 copies for sale and a lowest price of €9.00

### Requirement: Errors and offline
Failures SHALL be reported in the main window, and no entry SHALL be lost. This covers a rejected token, a page that doesn't exist, a private wantlist, and Discogs being unreachable. While Discogs is unreachable, expansion SHALL pause and the main window SHALL say that Discogs is offline. Expansion SHALL resume by itself when Discogs answers again.

#### Scenario: Connection drops mid-label
- **WHEN** the network drops after 50 of 312 releases have been expanded, and comes back a minute later
- **THEN** the 50 releases' entries stay, the main window says Discogs is offline, and expansion resumes at release 51

#### Scenario: Page doesn't exist
- **WHEN** the address names a label id that Discogs doesn't know
- **THEN** the main window says the page wasn't found, and no crate changes

### Requirement: Playback isolation
Talking to Discogs, downloading previews and analyzing them SHALL never block or delay the playback path:
- playback SHALL have zero underruns while an 800-release label is expanded and its previews are downloaded and analyzed;
- a preview that is ready SHALL start within the fast-start budget (< 30 ms);
- no request SHALL be sent to Discogs before the window is interactive, so launch stays within 300 ms.

#### Scenario: Big dig while playing
- **WHEN** an 800-release label is expanded, and previews are downloaded and analyzed, while a track plays
- **THEN** playback has zero underruns

#### Scenario: Launch unaffected
- **WHEN** the app is launched with a Discogs token and a crate whose expansion was interrupted
- **THEN** the window is interactive within 300 ms, and requests to Discogs start only after that

### Requirement: Formats on entries
Each entry from Discogs SHALL carry its record's formats, grouped as Vinyl, File, CD, Cassette or Other:
- from the release's details when they are known;
- from the listing until then.

A format written with a count ("2x12\"", "3xLP", "17xFile") SHALL count as that format. Descriptions such as Album, Comp or Ltd, and containers such as Box Set or All Media, SHALL NOT count as formats. A record SHALL be vinyl when one of its formats is Vinyl. The formats SHALL be saved with the crate. For an entry saved without them, showing its crate SHALL fill them from the cached data, and SHALL NOT make any request to do so. An entry whose record has formats but no Vinyl SHALL show a dim mark naming its first format (FILE, CD, CASS or OTHER) where the OWNED badge goes.

#### Scenario: Triple LP
- **WHEN** a label listing gives AF060LP's format as "3x12\", Comp, Ltd, Tur"
- **THEN** AF060LP is vinyl, and its entries show no format mark

#### Scenario: Digital only
- **WHEN** a release whose only format is "5xFile, FLAC, EP" is expanded
- **THEN** its entries carry the format File and show the mark FILE

#### Scenario: Older crate
- **WHEN** a crate saved before formats existed is shown, and its releases' data is in the disk cache
- **THEN** its entries carry their formats, and no request is sent to Discogs

### Requirement: Vinyl first
In a crate filled from a page (any crate but the user's wantlist and collection crates), two releases SHALL be the same record when they have the same master release, or, when either has none, the same catalog number and title, ignoring case. When a record has a vinyl release in the crate:
- its tunes SHALL belong to the vinyl release: a non-vinyl release of the same record SHALL add no entry, and its listed placeholder SHALL leave the crate. The one exception is a non-vinyl release that brings clips while the vinyl release has none: its entries stay;
- a non-vinyl release that has no clip SHALL leave the crate, whichever of the two was fetched first;
- when the non-vinyl release was expanded first, each of its entries whose clip the vinyl release also has SHALL become an entry of the vinyl release (its origin, catalog number, side, cover and for-sale data), keeping its place, its audio and, when it plays, its playback; its other entries SHALL leave the crate, except the entry that is playing, which SHALL leave once it stops;
- the expansion SHALL fetch a vinyl release before a non-vinyl release with the same catalog number and title, when both are waiting.

A record with no vinyl release SHALL stay in the crate with all its entries. Resolving twins SHALL NOT make any request beyond expanding the releases, and SHALL NOT interrupt playback. The wantlist and collection crates SHALL keep every release they hold.

#### Scenario: Neither has a clip
- **WHEN** a label lists AF069 on vinyl (38583846) and as FLAC (38518245), both of master 4361292, and neither has a clip on Discogs
- **THEN** the crate holds one "no clip" entry, of the vinyl release, whichever is fetched first

#### Scenario: Vinyl and digital of one record
- **WHEN** a label lists AF060LP as "17xFile, FLAC" (release 33337220) and as "3x12\"" (release 33988281), both of master 3847755
- **THEN** the crate holds AF060LP's tracks once, as entries of release 33988281, and nothing of release 33337220

#### Scenario: Digital fetched first
- **WHEN** the focus makes release 33337220 (File) expand before 33988281 (Vinyl), and one of its tracks is playing
- **THEN** once 33988281 is expanded, the playing entry and the others it shares become entries of 33988281, the track plays on without a gap, and no entry of 33337220 is left once it stops

#### Scenario: Digital only
- **WHEN** a label lists a release only as "4xFile, MP3"
- **THEN** its tracks are in the crate, marked FILE

#### Scenario: Wantlist untouched
- **WHEN** the wantlist crate holds a FLAC release and the vinyl release of the same master
- **THEN** both keep their entries

