# preview-fetch Specification

## Purpose
Gets a playable preview of each clip just before it is needed, using a yt-dlp that the user installs, and prepares it (sections, drops, waveform) so every preview can be navigated from its first second.

## Requirements

### Requirement: yt-dlp
The app SHALL use a yt-dlp installed by the user when there is one: at the path set in OPT ▸ Discogs…, on the PATH, or in the usual install folders. On macOS, on Linux x86_64 and on Windows x86_64, when none is found, the app SHALL manage its own copy:
- On macOS it SHALL be yt-dlp's official folder build (`yt-dlp_macos.zip`: the program with its libraries already unpacked beside it), unpacked whole into its own folder of `tools/`, never the one-file `yt-dlp_macos`. On Linux x86_64 it SHALL be yt-dlp's official build for that system (`yt-dlp_linux`). The first time previews are needed, it SHALL ask once whether to download yt-dlp, saying its size. Declining SHALL leave entries waiting, and the download SHALL stay available in OPT ▸ Discogs….
- On Windows it SHALL be yt-dlp's platform-independent zip app (`yt-dlp`), run by python.org's embeddable Python for x86_64, with Deno as its JavaScript runtime; Python's and Deno's version, address and SHA-256 are fixed in the app. Every program the app starts for it SHALL be signed (by the Python Software Foundation, Deno Land Inc. or Microsoft), so Windows 11's Smart App Control can allow it. The Python SHALL be used for nothing else, and SHALL ignore any other Python installed on the machine. It SHALL be asked for and installed as `ytdlp-setup` requires.
- yt-dlp SHALL come only from the latest release on yt-dlp's official GitHub releases, over HTTPS, and SHALL run only after its SHA-256 matches the one published with that release. Python (from python.org) and Deno (from Deno's GitHub releases) SHALL come only over HTTPS, and SHALL be unpacked only after their SHA-256 matches the one fixed in the app. A file that doesn't match SHALL be deleted and never run.
- The copy SHALL be kept in `tools/` in the cache folder, and replaced atomically (the macOS folder build, Python and Deno each as a whole folder), so a failed download or unpack never leaves a broken copy.
- On macOS, installing SHALL end with a first start of the new copy before it replaces the old one, allowed up to 180 s, while the question (or OPT ▸ Discogs…) says "Preparing yt-dlp…". The copy SHALL be used only if that start reports the version of the release it was downloaded from; otherwise the install SHALL fail, saying so, and the old copy (if any) SHALL stay in use.
- The app's own copy SHALL be found without being started: it is there when its program and the version recorded when it was installed are. A yt-dlp the user installed SHALL be found by starting it.
- A Mac with the app's earlier one-file copy SHALL get the folder build the next time yt-dlp is looked for, without asking again; the one-file copy SHALL be deleted once the folder build is in place, and SHALL NOT be started meanwhile.
- While previews are being fetched, the app SHALL check for a newer yt-dlp release at most once a day (on Windows also at once when `ytdlp-setup` finds it may be out of date), and replace its copy with a verified newer one. Python and Deno SHALL change only with a new version of the app. A copy the user installed SHALL never be updated by the app.

OPT ▸ Discogs… SHALL show the version in use and whether it is the user's or the app's. While yt-dlp can't be found or downloaded, entries SHALL wait with the status "needs yt-dlp", and the main window SHALL say once what to do (on macOS and Linux x86_64, how to download it; on Windows, the `ytdlp-setup` modal; elsewhere, how to install it). Text about installing SHALL name only tools of the system it runs on (no `brew` outside macOS). yt-dlp SHALL be looked for again at least every 30 s while entries are waiting for it, and whenever OPT ▸ Discogs… is opened; at most one look SHALL run at a time. When the app stops a yt-dlp it started (a time limit, a cancel, quitting), every process that yt-dlp started SHALL stop with it, on macOS and Linux. Starting a preview with the Windows copy SHALL add no console window, and SHALL take no longer than with `yt-dlp.exe` on the same machine.

#### Scenario: First previews on a fresh Mac
- **WHEN** a label is sent on a Mac with no yt-dlp, and the user accepts the download
- **THEN** the app downloads and verifies `yt-dlp_macos.zip`, says "Preparing yt-dlp…" while its first start runs, and the first previews start downloading without restarting the app

#### Scenario: The Mac copy starts fast once installed
- **WHEN** the app's macOS copy has been installed, and yt-dlp is looked for again (on an idle Mac)
- **THEN** the app's copy is found within 100 ms without starting any process, and a preview download starts yt-dlp within 2 s

#### Scenario: A Mac with the earlier one-file copy
- **WHEN** the app starts on a Mac whose `tools/` holds only the earlier one-file `yt-dlp`
- **THEN** entries wait with "preparing yt-dlp" instead of "needs yt-dlp", the folder build is downloaded and verified without a question, previews start once it is in place, and the one-file copy is gone — and the one-file copy was never started

#### Scenario: The first start fails
- **WHEN** the new macOS copy's first start doesn't report the release's version within 180 s
- **THEN** the install fails saying so, everything it started has stopped, the old copy (if any) is still used, and nothing half-installed is left in `tools/`

#### Scenario: No orphaned processes
- **WHEN** on macOS or Linux, a yt-dlp the app started is stopped at its time limit, while it has started child processes of its own
- **THEN** within 1 s no process it started is left running

#### Scenario: Looks never pile up
- **WHEN** a user's yt-dlp takes longer than its time limit to answer, for 10 minutes while entries wait
- **THEN** at no time is more than one yt-dlp started by the lookup running

#### Scenario: First previews on a fresh Linux desktop
- **WHEN** a label is sent on Linux x86_64 with no yt-dlp, and the user accepts the download
- **THEN** the app downloads and verifies `yt-dlp_linux`, and the first previews start downloading without restarting the app

#### Scenario: First previews on a fresh Windows PC
- **WHEN** a record is sent on Windows with no yt-dlp, and the user chooses Install
- **THEN** the app downloads and verifies Python, the zip app and Deno, and the first previews start downloading without restarting the app

#### Scenario: YouTube with the Windows copy
- **WHEN** the app's Windows copy fetches a YouTube preview with an empty yt-dlp cache
- **THEN** it is started with `--no-js-runtimes --js-runtimes deno:‹its Deno›`, and yt-dlp reports no missing JavaScript runtime

#### Scenario: Smart App Control on
- **WHEN** the app's Windows copy fetches a preview on Windows 11 with Smart App Control on
- **THEN** the preview downloads and plays, with no block from Smart App Control

#### Scenario: Download declined
- **WHEN** the user declines the download on macOS or Linux
- **THEN** entries wait with "needs yt-dlp", the question isn't asked again this session, and OPT ▸ Discogs… offers the download

#### Scenario: Checksum mismatch
- **WHEN** a downloaded file's SHA-256 doesn't match the published (or, for Python and Deno, the fixed) one
- **THEN** the file is deleted without being run or unpacked, entries keep waiting, and the app says the download failed

#### Scenario: The user's own yt-dlp
- **WHEN** yt-dlp is installed with Homebrew, with the Linux distribution's package manager, or as `yt-dlp.exe` on the Windows PATH
- **THEN** the app uses it, never offers a download, and never updates it

#### Scenario: A newer release
- **WHEN** the app's copy is older than the latest release and previews are fetched, a day after the last check
- **THEN** the newer release is downloaded, verified (on macOS, also started once) and used for the next downloads

#### Scenario: Installed later (other platforms)
- **WHEN** on non-x86_64 Linux or on arm64 Windows the user installs yt-dlp while entries are waiting for it
- **THEN** downloads start within 30 s, without restarting the app

### Requirement: Download ahead
The app SHALL keep previews ready for the playing entry and the next 3 entries in its crate's play order. When playback is stopped, it SHALL do the same for the shown crate's current entry (its first entry when it has none) and the 3 after it. An armed entry SHALL be downloaded first. While a record card is open, its record's previews SHALL come right after the armed and playing entries' and before the next 3 (see `record-card`). No other preview SHALL be downloaded, except those of a label crate the user asked to download in full (see `label-crates`), which come after all of these. At most 2 downloads SHALL run at once, audio only, and each entry being downloaded SHALL show its progress (for example "downloading 40%").

#### Scenario: Three ahead
- **WHEN** entry 3 of a 50-entry crate is playing
- **THEN** the previews of entries 4, 5 and 6 are ready or downloading, and no preview after entry 6 is downloaded

#### Scenario: Armed first
- **WHEN** the user double-clicks entry 30 while the previews of entries 4 to 6 are downloading
- **THEN** entry 30's preview starts downloading as soon as a download slot is free, before any other waiting preview

#### Scenario: Play from nothing
- **WHEN** a label is sent with Play while nothing is playing
- **THEN** the previews of the new crate's first 4 entries are downloaded, and the first entry plays as soon as its preview is ready

#### Scenario: An open record card
- **WHEN** entry 3 plays and the user opens the card of a record of 4 tracks further down
- **THEN** entry 3's preview and the record's 4 come before entries 4 to 6, two at a time

### Requirement: Safe invocation
yt-dlp SHALL only be given one of:
- **A YouTube clip id:** exactly 11 letters, digits, `-` or `_`.
- **A YouTube search** (see `preview-search`): a query built only from a tracklist's artist and title, without control characters, at most 120 characters, with the `ytsearch5:` prefix, listing results without downloading them.
- **A Bandcamp address** that passes the check in `bandcamp-intake`, rebuilt from its parts, given to read a page without downloading or to download one track.
- **A mix to read** (see `mix-tracklists`): a YouTube video id passing the clip id check, or a SoundCloud track address `https://soundcloud.com/‹user›/‹slug›` with both parts matching `[a-z0-9_-]{1,100}`, rebuilt by the app, given only to read its metadata (`--dump-single-json --skip-download --no-playlist`), never to download. Mixcloud and 1001tracklists addresses SHALL never be given to yt-dlp.

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

#### Scenario: Reading a mix
- **WHEN** the user pastes `https://soundcloud.com/some-dj/summer-mix-2026?si=abc`
- **THEN** yt-dlp receives `https://soundcloud.com/some-dj/summer-mix-2026` after `--` with `--skip-download`, and no media is written

### Requirement: Preview cache
Previews SHALL be stored in `previews/` in the cache folder. Together with the waveform overviews in `overviews/`, they SHALL use at most the preview cache size, which is 10 GB by default and set in Options ▸ Discogs…. When a new preview doesn't fit, the previews and overviews used least recently SHALL be deleted first. A preview downloaded for Download all tracks SHALL never cause a deletion: downloading pauses instead (see `label-crates`). The previews of the playing entry, the armed entry and the next 3 entries, and their overviews, SHALL never be deleted. A deleted preview SHALL be downloaded again when it is needed, whether or not its crate was open when it was deleted, and also when its file is missing for any other reason (such as a cache folder from an older version). Such an entry SHALL go back to waiting for its download, never to failed. A deleted preview's waveform overview SHALL be deleted with it. When the downloaded file is identical, its cached analysis SHALL be reused.

#### Scenario: Over the limit
- **WHEN** a new preview would take the cache over its limit
- **THEN** the least recently played previews are deleted until it fits, and the playing, armed and next 3 previews are kept

#### Scenario: Downloaded again
- **WHEN** a preview that was deleted is downloaded again as an identical file
- **THEN** it starts with its whole waveform and sections, without being analyzed again

#### Scenario: Missing file downloads again
- **WHEN** a collection-crate track whose preview file no longer exists is played or its details are read
- **THEN** the track waits for its preview again (not red, not failed), its preview is downloaded, and it plays

### Requirement: Ready to navigate
Right after a preview is downloaded, its sections, drops and waveform SHALL be computed in the background and cached. This SHALL start only once the playing track's own analysis covers its first 32 bars, so that it never competes with a track start.
- A preview whose preparation has finished before it starts SHALL show its whole waveform, section bands and drop markers in its first frame of playback, and section jumps, drop jumps and loops SHALL work from its first second.
- Preparing a 6-minute preview SHALL take no more than 20 s on an M-series Mac while another track plays.
- Seeking within a preview SHALL take less than 50 ms, as for a local file.

#### Scenario: Next track opens ready
- **WHEN** the next entry's preview was downloaded and prepared while the current track played, and the user presses B
- **THEN** the new track's whole waveform, section bands and drop markers are drawn in its first frame, and Shift+] jumps to its first drop

#### Scenario: Out of order
- **WHEN** the user plays an entry whose preview has just been downloaded and isn't prepared yet
- **THEN** it plays at once, and its waveform and sections fill in as they do for a local file played for the first time

### Requirement: Previews are for listening
Previews SHALL only be played. The app SHALL NOT offer to save, export or copy preview audio. Exporting a crate SHALL write each preview entry's clip address, never a file from the preview folder.

#### Scenario: Export a dig crate
- **WHEN** a crate of downloaded previews is exported as M3U8
- **THEN** it lists the clips' addresses and no file from the preview folder

### Requirement: YouTube limiting
When yt-dlp's error for a download or a search says that YouTube is limiting requests (HTTP 429, "Too Many Requests", or a request to sign in to confirm the user isn't a bot), the app SHALL treat it as limited, not as a failure of that track: the track SHALL NOT be marked "clip failed" or "not found", and its tries SHALL NOT be counted. While limited, no preview download or search SHALL start. The app SHALL try again after 10 minutes, then after 20, 40 and at most 60 minutes while YouTube keeps limiting, and the wait SHALL start again at 10 minutes once a download or search succeeds. The main window SHALL say once per limited period "YouTube is limiting requests: trying again in ‹N› min". Entries waiting for their preview meanwhile SHALL keep waiting, and their tooltip SHALL say "waiting for YouTube". Being limited SHALL never touch playback of previews already downloaded.

#### Scenario: Too many requests
- **WHEN** yt-dlp fails a download with "HTTP Error 429: Too Many Requests"
- **THEN** the track stays waiting (not "clip failed"), no other download or search starts, and the main window says YouTube is limiting requests and in how many minutes it tries again

#### Scenario: Bot check on a search
- **WHEN** a search fails with "Sign in to confirm you're not a bot"
- **THEN** the track stays waiting to be searched, it isn't remembered as not found, and searching pauses like downloading

#### Scenario: Longer waits
- **WHEN** YouTube is still limiting when the app tries again after 10 minutes
- **THEN** it waits 20 minutes, then 40, then 60 minutes between tries

#### Scenario: Back to normal
- **WHEN** a try after the wait succeeds
- **THEN** downloads and searches carry on, and the next limited period waits 10 minutes again

#### Scenario: A broken video is still a failure
- **WHEN** yt-dlp fails a download with "Video unavailable"
- **THEN** it counts as a failed try as before, and after two the track is "clip failed"

### Requirement: Bandcamp limiting
When yt-dlp's error for a Bandcamp read or download says that Bandcamp is limiting requests (HTTP 429 or "Too Many Requests"), it SHALL be handled as YouTube limiting is: not a failure, with the same 10 → 20 → 40 → 60 minute waits. The pause SHALL be kept apart from YouTube's: while one source is limited, the other SHALL go on. The main window, the tooltips ("waiting for Bandcamp") and Download all tracks' window SHALL name the source that is limited.

#### Scenario: Only Bandcamp limited
- **WHEN** Bandcamp answers 429 while a crate mixes YT and BC entries
- **THEN** BC entries keep waiting with "waiting for Bandcamp", YT entries keep downloading, and the main window says Bandcamp is limiting requests

### Requirement: Unreachable source
When yt-dlp's error for a download, a search or a Bandcamp read, or a Juno sample request, says that the source can't be reached (no connection, connection refused or reset, a name that doesn't resolve, or a certificate that fails to verify), the app SHALL treat it as unreachable, not as a failure of that track. The track SHALL NOT be marked "clip failed" or "not found", its tries SHALL NOT be counted, and a search SHALL NOT be remembered as failed. While a source is unreachable, no download, search or read from it SHALL start, and the other sources SHALL go on. The app SHALL try again after 1 minute, then after 2, 4, 8 and at most 10 minutes while the source stays unreachable, and once a request to it goes through, the wait SHALL start again at 1 minute. The main window SHALL say once per unreachable period "Can't reach ‹source›: trying again in ‹N› min", and when the cause is a certificate that fails to verify, it SHALL add that a proxy, VPN or security tool may be intercepting secure connections. Entries waiting meanwhile SHALL keep waiting, with the tooltip "waiting for ‹source›". The pause SHALL stay visible after that message: an armed entry waiting for the source SHALL say in the status line why it waits and when the next try is ("Waiting for ‹entry›. Can't reach YouTube: trying again in 1 min…"), and with nothing else to show, the status line SHALL keep saying it. An HTTP error status, "Video unavailable" and a download stopped after 120 s SHALL still count as failed tries. Being unreachable SHALL never touch playback of previews already downloaded.

#### Scenario: Intercepted connection
- **WHEN** every yt-dlp run fails with "[SSL: CERTIFICATE_VERIFY_FAILED] certificate verify failed"
- **THEN** no entry becomes "clip failed" or "not found", the entries keep waiting with "waiting for YouTube", and the main window says once that YouTube can't be reached, when it tries again, and that a proxy, VPN or security tool may be intercepting secure connections

#### Scenario: A double-clicked entry says why it waits
- **WHEN** the user double-clicks a track waiting to be searched while YouTube can't be reached
- **THEN** the status line says "Waiting for ‹track›. Can't reach YouTube: trying again in ‹N› min", with the interception hint after a certificate error, until it plays

#### Scenario: Shorter waits than limiting
- **WHEN** YouTube is still unreachable at each try
- **THEN** the tries come after 1, 2, 4 and 8 minutes, and then every 10 minutes

#### Scenario: Network comes back
- **WHEN** a try after the wait succeeds
- **THEN** downloads and searches carry on, the waiting entries download, and the next unreachable period waits 1 minute again

#### Scenario: Only one source unreachable
- **WHEN** YouTube can't be reached while Juno samples download normally
- **THEN** JN entries keep downloading and YT entries keep waiting

#### Scenario: A missing video is still a failure
- **WHEN** yt-dlp fails a download with "HTTP Error 404" or "Video unavailable"
- **THEN** it counts as a failed try, and after two the track is "clip failed"
