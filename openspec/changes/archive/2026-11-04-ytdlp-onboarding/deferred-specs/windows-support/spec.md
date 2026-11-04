## MODIFIED Requirements

### Requirement: yt-dlp is found on Windows
On Windows the app SHALL find a yt-dlp that the user installed: at the path set in OPT ▸ Discogs… (with or without `.exe`), or on the PATH as `yt-dlp.exe`, and SHALL prefer it to its own copy. When none is found, it SHALL offer to install its own (Python, the zip app and Deno, see `preview-fetch` and `ytdlp-setup`). While yt-dlp is missing, entries SHALL say "needs yt-dlp", no text SHALL mention `brew`, and the app SHALL look again within 30 s of one being installed, as the preview spec requires.

#### Scenario: On the PATH
- **WHEN** `yt-dlp.exe` is in a PATH folder and a record is sent
- **THEN** its previews start downloading and OPT ▸ Discogs… shows its version as "installed by you"

#### Scenario: Not installed
- **WHEN** a record is sent on Windows with no yt-dlp and the user plays one of its entries
- **THEN** its entries say "needs yt-dlp", the modal offers Install, and no text mentions `brew`
