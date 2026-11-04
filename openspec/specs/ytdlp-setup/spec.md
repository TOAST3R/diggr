# ytdlp-setup Specification

## Purpose
On Windows, offers to install the app's own yt-dlp (its Python, zip app and Deno) whenever previews need it and it is missing, installs it with its progress in view, and notices when a yt-dlp has probably gone out of date.
## Requirements
### Requirement: Asked at first launch
On Windows x86_64, while yt-dlp can't be found, the welcome at launch (see `welcome-setup`) SHALL include a Previews step, listed first. It SHALL say that previews from Discogs, Bandcamp and the other shops need yt-dlp, the download size, where it comes from, that the user's own files play without it, and that it also installs a Python and a JavaScript runtime used only by Diggr. It SHALL offer Install, which installs and shows the progress in the step as the guided question does, and Later. It SHALL follow the welcome's rules for when it opens, so no separate modal SHALL open at launch for yt-dlp. Showing it SHALL NOT delay launch: the lookup SHALL run off the UI thread, and the step SHALL appear once it has found no yt-dlp.

#### Scenario: Fresh Windows PC
- **WHEN** Diggr is opened for the first time on a Windows PC without yt-dlp
- **THEN** the player opens, and within 2 s the welcome lists the Previews step first, with Install and Later

#### Scenario: yt-dlp already there
- **WHEN** Diggr is opened on a Windows PC with `yt-dlp.exe` on the PATH
- **THEN** the welcome has no Previews step

#### Scenario: Turned off
- **WHEN** the user ticks "Don't show at startup" and reopens the app without yt-dlp
- **THEN** no modal opens at launch; the question still opens when the user plays a preview

### Requirement: Asked when previews are needed
On Windows, while yt-dlp can't be found, the same modal SHALL open each time the user does something that needs a yt-dlp preview: plays or arms an entry whose preview isn't there, sends a page from the extension, pastes an address that makes a crate, or clicks an entry's "needs yt-dlp" icon. Entries whose previews need no yt-dlp (Juno, Beatport samples) SHALL NOT open it. Previews prefetched in the background and refreshes the app starts itself SHALL NOT open it. Not now (or Escape) SHALL close it until the next such action. On arm64 Windows, where the app can't manage a copy, the modal SHALL say how to install yt-dlp instead of offering Install. macOS and Linux SHALL keep the question `preview-fetch` describes for them.

#### Scenario: Play without yt-dlp
- **WHEN** the user double-clicks a YouTube entry and yt-dlp is missing
- **THEN** the modal opens; after Install succeeds, that entry plays without a further click

#### Scenario: Browsing only
- **WHEN** a crate fills with entries and prefetching starts, without a user action needing a preview
- **THEN** the modal does not open, and the entries say "needs yt-dlp"

#### Scenario: The entry icon
- **WHEN** the user clicks the "needs yt-dlp" icon of an entry after choosing Not now
- **THEN** the modal opens again

### Requirement: Installing from the modal
On Windows, Install SHALL download and verify Python, the zip app and Deno as the `preview-fetch` spec requires, showing the progress in the modal. On success the modal SHALL say "Previews are ready" and close within 2 s, and waiting entries SHALL start downloading without restarting the app. On failure it SHALL say why, in words: a host couldn't be reached (naming it), a download didn't match its checksum, or it couldn't be unpacked; and SHALL offer Try again. Nothing SHALL be downloaded before the user chooses Install.

#### Scenario: Behind a blocking proxy
- **WHEN** Install is chosen and python.org can't be reached
- **THEN** the modal says python.org couldn't be reached, nothing is left half-installed, and Try again is offered

#### Scenario: No answer, no download
- **WHEN** the modal is closed with Not now
- **THEN** the app has made no request to GitHub or python.org for yt-dlp, Python or Deno

### Requirement: An outdated yt-dlp is noticed
On Windows, when 3 previews of different clips in a row fail (not because the source is limiting requests, not because it can't be reached), the app SHALL treat yt-dlp as possibly out of date. For its own copy, it SHALL check for a newer release at once, regardless of the daily limit but at most once an hour, and use a verified newer one. For a copy the user installed whose version is more than 60 days old, the main window SHALL say once per session that it may be out of date, with its version and where to get a newer one. A success SHALL reset the count.

#### Scenario: The app's copy is stale
- **WHEN** three YouTube previews in a row fail with an extraction error, using the app's copy, 3 hours after the last update check
- **THEN** the app checks for a newer release at once, and installs and uses it if there is one

#### Scenario: The user's copy is stale
- **WHEN** the same happens with a `yt-dlp.exe` of version 2026.05.01 on the PATH in November 2026
- **THEN** the main window says that yt-dlp 2026.05.01 may be out of date and where to get a newer one, once this session

#### Scenario: YouTube throttling
- **WHEN** three previews in a row fail because YouTube is limiting requests
- **THEN** no update check or out-of-date message is triggered

