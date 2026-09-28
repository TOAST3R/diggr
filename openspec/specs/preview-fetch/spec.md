# preview-fetch Specification

## Purpose
Gets a playable preview of each clip just before it is needed, using a yt-dlp that the user installs, and prepares it (sections, drops, waveform) so every preview can be navigated from its first second.

## Requirements
### Requirement: yt-dlp
The app SHALL use a yt-dlp program installed by the user, found on the PATH or at a path set in OPT ▸ Discogs…, and SHALL show its version there. The app SHALL NOT bundle, download or update yt-dlp itself. While yt-dlp can't be found, entries SHALL wait with the status "needs yt-dlp", and the main window SHALL say once how to install it. yt-dlp SHALL be looked for again at least every 30 s while entries are waiting for it, and whenever OPT ▸ Discogs… is opened.

#### Scenario: Missing yt-dlp
- **WHEN** a label is sent and yt-dlp isn't installed
- **THEN** its entries wait with "needs yt-dlp", and the main window says once how to install it

#### Scenario: Installed later
- **WHEN** the user installs yt-dlp while entries are waiting for it
- **THEN** downloads start within 30 s, without restarting the app

### Requirement: Download ahead
The app SHALL keep previews ready for the playing entry and the next 3 entries in its crate's play order. When playback is stopped, it SHALL do the same for the shown crate's current entry (its first entry when it has none) and the 3 after it. An armed entry SHALL be downloaded first. No other preview SHALL be downloaded. At most 2 downloads SHALL run at once, audio only, and each entry being downloaded SHALL show its progress (for example "downloading 40%").

#### Scenario: Three ahead
- **WHEN** entry 3 of a 50-entry crate is playing
- **THEN** the previews of entries 4, 5 and 6 are ready or downloading, and no preview after entry 6 is downloaded

#### Scenario: Armed first
- **WHEN** the user double-clicks entry 30 while the previews of entries 4 to 6 are downloading
- **THEN** entry 30's preview starts downloading as soon as a download slot is free, before any other waiting preview

#### Scenario: Play from nothing
- **WHEN** a label is sent with Play while nothing is playing
- **THEN** the previews of the new crate's first 4 entries are downloaded, and the first entry plays as soon as its preview is ready

### Requirement: Safe invocation
yt-dlp SHALL only be given a clip id made of exactly 11 letters, digits, `-` or `_`. The id SHALL be passed as its own argument, never through a shell, and the user's yt-dlp configuration SHALL be ignored. Output SHALL be confined to the preview folder. A clip whose id doesn't match SHALL be treated as unusable. A download that hasn't finished after 120 s SHALL be stopped and retried once. A clip that fails twice SHALL make its entry unavailable, with the reason "clip failed".

#### Scenario: Malformed clip id
- **WHEN** a record's clip address carries an id with other characters (for example `abc;rm -rf`)
- **THEN** yt-dlp is not run for it, and the clip is treated as unusable

#### Scenario: Dead clip
- **WHEN** a clip no longer exists
- **THEN** after one retry its entry becomes unavailable with "clip failed", and playback moves on to the next ready entry

### Requirement: Preview cache
Previews SHALL be stored in `previews/` in the cache folder. They SHALL use at most the preview cache size, which is 2 GB by default and set in OPT ▸ Discogs…. When a new preview doesn't fit, the previews played least recently SHALL be deleted first. The previews of the playing entry, the armed entry and the next 3 entries SHALL never be deleted. A deleted preview SHALL be downloaded again when it is needed. When the downloaded file is identical, its cached analysis and waveform SHALL be reused.

#### Scenario: Over the limit
- **WHEN** a new preview would take the cache over its limit
- **THEN** the least recently played previews are deleted until it fits, and the playing, armed and next 3 previews are kept

#### Scenario: Downloaded again
- **WHEN** a preview that was deleted is downloaded again as an identical file
- **THEN** it starts with its whole waveform and sections, without being analyzed again

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
