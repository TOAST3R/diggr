## MODIFIED Requirements

### Requirement: yt-dlp
The app SHALL use a yt-dlp installed by the user when there is one: at the path set in OPT ▸ Discogs…, on the PATH, or in the usual install folders. On macOS, on Linux x86_64 and on Windows x86_64, when none is found, the app SHALL manage its own copy:
- On macOS and Linux x86_64 it SHALL be yt-dlp's official build for that system (`yt-dlp_macos` or `yt-dlp_linux`). The first time previews are needed, it SHALL ask once whether to download yt-dlp, saying its size. Declining SHALL leave entries waiting, and the download SHALL stay available in OPT ▸ Discogs….
- On Windows it SHALL be yt-dlp's platform-independent zip app (`yt-dlp`), run by python.org's embeddable Python for x86_64, with Deno as its JavaScript runtime; Python's and Deno's version, address and SHA-256 are fixed in the app. Every program the app starts for it SHALL be signed (by the Python Software Foundation, Deno Land Inc. or Microsoft), so Windows 11's Smart App Control can allow it. The Python SHALL be used for nothing else, and SHALL ignore any other Python installed on the machine. It SHALL be asked for and installed as `ytdlp-setup` requires.
- yt-dlp SHALL come only from the latest release on yt-dlp's official GitHub releases, over HTTPS, and SHALL run only after its SHA-256 matches the one published with that release. Python (from python.org) and Deno (from Deno's GitHub releases) SHALL come only over HTTPS, and SHALL be unpacked only after their SHA-256 matches the one fixed in the app. A file that doesn't match SHALL be deleted and never run.
- The copy SHALL be kept in `tools/` in the cache folder, and replaced atomically (Python and Deno each as a whole folder), so a failed download or unpack never leaves a broken copy.
- While previews are being fetched, the app SHALL check for a newer yt-dlp release at most once a day (on Windows also at once when `ytdlp-setup` finds it may be out of date), and replace its copy with a verified newer one. Python and Deno SHALL change only with a new version of the app. A copy the user installed SHALL never be updated by the app.

OPT ▸ Discogs… SHALL show the version in use and whether it is the user's or the app's. While yt-dlp can't be found or downloaded, entries SHALL wait with the status "needs yt-dlp", and the main window SHALL say once what to do (on macOS and Linux x86_64, how to download it; on Windows, the `ytdlp-setup` modal; elsewhere, how to install it). Text about installing SHALL name only tools of the system it runs on (no `brew` outside macOS). yt-dlp SHALL be looked for again at least every 30 s while entries are waiting for it, and whenever OPT ▸ Discogs… is opened. Starting a preview with the Windows copy SHALL add no console window, and SHALL take no longer than with `yt-dlp.exe` on the same machine.

#### Scenario: First previews on a fresh Mac
- **WHEN** a label is sent on a Mac with no yt-dlp, and the user accepts the download
- **THEN** the app downloads and verifies yt-dlp, and the first previews start downloading without restarting the app

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
- **THEN** the newer release is downloaded, verified and used for the next downloads

#### Scenario: Installed later (other platforms)
- **WHEN** on non-x86_64 Linux or on arm64 Windows the user installs yt-dlp while entries are waiting for it
- **THEN** downloads start within 30 s, without restarting the app
