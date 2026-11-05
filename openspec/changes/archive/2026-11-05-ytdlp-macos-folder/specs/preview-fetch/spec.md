# Spec Delta

## MODIFIED Requirements

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
