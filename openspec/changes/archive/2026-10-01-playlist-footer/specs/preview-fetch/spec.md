## MODIFIED Requirements

### Requirement: yt-dlp
The app SHALL use a yt-dlp program installed by the user, found on the PATH or at a path set in Options ▸ Discogs…, and SHALL show its version there. The app SHALL NOT bundle, download or update yt-dlp itself. While yt-dlp can't be found, entries SHALL wait with the status "needs yt-dlp", and the main window SHALL say once how to install it. yt-dlp SHALL be looked for again at least every 30 s while entries are waiting for it, and whenever Options ▸ Discogs… is opened.

#### Scenario: Missing yt-dlp
- **WHEN** a label is sent and yt-dlp isn't installed
- **THEN** its entries wait with "needs yt-dlp", and the main window says once how to install it

#### Scenario: Installed later
- **WHEN** the user installs yt-dlp while entries are waiting for it
- **THEN** downloads start within 30 s, without restarting the app

### Requirement: Preview cache
Previews SHALL be stored in `previews/` in the cache folder. They SHALL use at most the preview cache size, which is 2 GB by default and set in Options ▸ Discogs…. When a new preview doesn't fit, the previews played least recently SHALL be deleted first. The previews of the playing entry, the armed entry and the next 3 entries SHALL never be deleted. A deleted preview SHALL be downloaded again when it is needed. When the downloaded file is identical, its cached analysis and waveform SHALL be reused.

#### Scenario: Over the limit
- **WHEN** a new preview would take the cache over its limit
- **THEN** the least recently played previews are deleted until it fits, and the playing, armed and next 3 previews are kept

#### Scenario: Downloaded again
- **WHEN** a preview that was deleted is downloaded again as an identical file
- **THEN** it starts with its whole waveform and sections, without being analyzed again
