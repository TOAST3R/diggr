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

Failures:
- A clip whose id or address doesn't match SHALL be treated as unusable.
- A download that hasn't finished after 120 s SHALL be stopped and retried once.
- A clip that fails twice SHALL make its entry unavailable, with the reason "clip failed".

#### Scenario: Malformed clip id
- **WHEN** a record's clip address carries an id with other characters (for example `abc;rm -rf`)
- **THEN** yt-dlp is not run for it, and the clip is treated as unusable

#### Scenario: Dead clip
- **WHEN** a clip no longer exists
- **THEN** after one retry its entry becomes unavailable with "clip failed", and playback moves on to the next ready entry

#### Scenario: A hostile track title
- **WHEN** a tracklist's title is `--exec rm -rf ~` and its entry is searched
- **THEN** yt-dlp receives it inside one `ytsearch5:` argument after `--`, runs no command, and nothing is downloaded by the search

#### Scenario: A Bandcamp track
- **WHEN** an entry plays from Bandcamp track 3020153053 at `https://analogicalforce.bandcamp.com/track/the-ooze`
- **THEN** yt-dlp receives exactly that address after `--`, and writes `bc.3020153053.mp3` in the preview folder

## ADDED Requirements

### Requirement: Bandcamp limiting
When yt-dlp's error for a Bandcamp read or download says that Bandcamp is limiting requests (HTTP 429 or "Too Many Requests"), it SHALL be handled as YouTube limiting is: not a failure, with the same 10 → 20 → 40 → 60 minute waits. The pause SHALL be kept apart from YouTube's: while one source is limited, the other SHALL go on. The main window, the tooltips ("waiting for Bandcamp") and Download all tracks' window SHALL name the source that is limited.

#### Scenario: Only Bandcamp limited
- **WHEN** Bandcamp answers 429 while a crate mixes YT and BC entries
- **THEN** BC entries keep waiting with "waiting for Bandcamp", YT entries keep downloading, and the main window says Bandcamp is limiting requests
