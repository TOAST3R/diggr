## MODIFIED Requirements

### Requirement: yt-dlp
The app SHALL use a yt-dlp installed by the user when there is one: at the path set in OPT ▸ Discogs…, on the PATH, or in the usual install folders. On macOS, when none is found, the app SHALL manage its own copy:
- The first time previews are needed, it SHALL ask once whether to download yt-dlp, saying its size. Declining SHALL leave entries waiting, and the download SHALL stay available in OPT ▸ Discogs….
- It SHALL download only the latest release from yt-dlp's official GitHub releases, over HTTPS, and SHALL run it only after its SHA-256 matches the one published with that release. A copy that doesn't match SHALL be deleted and never run.
- The copy SHALL be kept in `tools/` in the cache folder, and replaced atomically, so a failed download never leaves a broken copy.
- While previews are being fetched, the app SHALL check for a newer release at most once a day, and replace its copy with a verified newer one. A copy the user installed SHALL never be updated by the app.

OPT ▸ Discogs… SHALL show the version in use and whether it is the user's or the app's. While yt-dlp can't be found or downloaded, entries SHALL wait with the status "needs yt-dlp", and the main window SHALL say once what to do (on macOS, how to download it; elsewhere, how to install it). yt-dlp SHALL be looked for again at least every 30 s while entries are waiting for it, and whenever OPT ▸ Discogs… is opened.

#### Scenario: First previews on a fresh Mac
- **WHEN** a label is sent on a Mac with no yt-dlp, and the user accepts the download
- **THEN** the app downloads and verifies yt-dlp, and the first previews start downloading without restarting the app

#### Scenario: Download declined
- **WHEN** the user declines the download
- **THEN** entries wait with "needs yt-dlp", the question isn't asked again this session, and OPT ▸ Discogs… offers the download

#### Scenario: Checksum mismatch
- **WHEN** the downloaded file's SHA-256 doesn't match the published one
- **THEN** the file is deleted without being run, entries keep waiting, and the main window says the download failed

#### Scenario: The user's own yt-dlp
- **WHEN** yt-dlp is installed with Homebrew
- **THEN** the app uses it, never offers a download, and never updates it

#### Scenario: A newer release
- **WHEN** the app's copy is older than the latest release and previews are fetched, a day after the last check
- **THEN** the newer release is downloaded, verified and used for the next downloads

#### Scenario: Installed later (other platforms)
- **WHEN** on Linux or Windows the user installs yt-dlp while entries are waiting for it
- **THEN** downloads start within 30 s, without restarting the app
