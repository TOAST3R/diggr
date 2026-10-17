## MODIFIED Requirements

### Requirement: Safe invocation
yt-dlp SHALL only be given one of:
- **A YouTube clip id:** exactly 11 letters, digits, `-` or `_`.
- **A YouTube search** (see `preview-search`): a query built only from a tracklist's artist and title, without control characters, at most 120 characters, with the `ytsearch5:` prefix, listing results without downloading them.
- **A Bandcamp address** that passes the check in `bandcamp-intake`, rebuilt from its parts, given to read a page without downloading or to download one track.

How it is run:
- Only a search result's id that passes the same check SHALL ever be kept.
- Only a Bandcamp result whose address passes the check and whose track id is 1 to 20 digits SHALL ever be kept.
- The id, query or address SHALL be passed as its own argument after `--`, never through a shell.
- The user's yt-dlp configuration SHALL be ignored.
- Output SHALL be confined to the preview folder, named after the YouTube id or `bc.‹track id›`.

Plain downloads: the only requests the app SHALL make for a preview without yt-dlp are a Juno sample (see `juno-intake`) and a Beatport sample (see `beatport-intake`):
- a Juno sample's address SHALL be `https://mp3.juno.co.uk/MP3/SF‹product›-‹sample›.mp3`, and a Beatport sample's `https://geo-samples.beatport.com/track/‹sample id›.LOFI.mp3`, each rebuilt from a key that passes its store's checks, never taken from a page or the extension;
- a Juno request SHALL follow redirects only to `mp3.juno.co.uk` or `www.juno.co.uk`, and a Beatport request only to `beatport.com` and its subdomains, over HTTPS;
- each SHALL be written only to the preview folder, named `jn.‹product›-‹sample›.mp3` or `bp.‹sample id›.mp3`.

Failures:
- A clip whose id or address doesn't match SHALL be treated as unusable.
- A download that hasn't finished after 120 s SHALL be stopped and retried once.
- A clip that fails twice SHALL make its entry unavailable, with the reason "clip failed".

#### Scenario: Malformed clip id
- **WHEN** a record's clip address carries an id with other characters (for example `abc;rm -rf`)
- **THEN** yt-dlp is not run for it, and the clip is treated as unusable

#### Scenario: A Beatport sample
- **WHEN** an entry plays from Beatport sample `0f1e2d3c-4b5a-6978-8a9b-0c1d2e3f4a5b`
- **THEN** the app requests exactly `https://geo-samples.beatport.com/track/0f1e2d3c-4b5a-6978-8a9b-0c1d2e3f4a5b.LOFI.mp3`, runs no yt-dlp, and writes `bp.0f1e2d3c-4b5a-6978-8a9b-0c1d2e3f4a5b.mp3` in the preview folder

#### Scenario: Dead clip
- **WHEN** a clip no longer exists
- **THEN** after one retry its entry becomes unavailable with "clip failed", and playback moves on to the next ready entry

#### Scenario: A hostile track title
- **WHEN** a tracklist's title is `--exec rm -rf ~` and its entry is searched
- **THEN** yt-dlp receives it inside one `ytsearch5:` argument after `--`, runs no command, and nothing is downloaded by the search

#### Scenario: A Bandcamp track
- **WHEN** an entry plays from Bandcamp track 3020153053 at `https://analogicalforce.bandcamp.com/track/the-ooze`
- **THEN** yt-dlp receives exactly that address after `--`, and writes `bc.3020153053.mp3` in the preview folder

#### Scenario: A Juno sample
- **WHEN** an entry plays from Juno key `jn.1178909-01-01-01`
- **THEN** the app requests exactly `https://mp3.juno.co.uk/MP3/SF1178909-01-01-01.mp3`, runs no yt-dlp, and writes `jn.1178909-01-01-01.mp3` in the preview folder

#### Scenario: A redirect elsewhere
- **WHEN** a Juno sample request is redirected to `https://example.net/x.mp3`
- **THEN** the redirect is not followed, and the try fails
