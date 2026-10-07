## MODIFIED Requirements

### Requirement: Safe invocation
yt-dlp SHALL only be given a clip id made of exactly 11 letters, digits, `-` or `_`, or a search (see `preview-search`): a query built only from a tracklist's artist and title, without control characters, at most 120 characters, after `--` with the `ytsearch5:` prefix, listing results without downloading them. Only a result's id that passes the same check SHALL ever be kept. The id or query SHALL be passed as its own argument, never through a shell, and the user's yt-dlp configuration SHALL be ignored. Output SHALL be confined to the preview folder. A clip whose id doesn't match SHALL be treated as unusable. A download that hasn't finished after 120 s SHALL be stopped and retried once. A clip that fails twice SHALL make its entry unavailable, with the reason "clip failed".

#### Scenario: Malformed clip id
- **WHEN** a record's clip address carries an id with other characters (for example `abc;rm -rf`)
- **THEN** yt-dlp is not run for it, and the clip is treated as unusable

#### Scenario: Dead clip
- **WHEN** a clip no longer exists
- **THEN** after one retry its entry becomes unavailable with "clip failed", and playback moves on to the next ready entry

#### Scenario: A hostile track title
- **WHEN** a tracklist's title is `--exec rm -rf ~` and its entry is searched
- **THEN** yt-dlp receives it inside one `ytsearch5:` argument after `--`, runs no command, and nothing is downloaded by the search
