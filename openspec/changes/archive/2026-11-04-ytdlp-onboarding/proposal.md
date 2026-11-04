## Why

A Windows user downloaded `yt-dlp.exe`, left it in Downloads, and Diggr never found it: on
Windows the app only looks on the PATH and at a path typed into OPT ▸ Discogs…, offers no
download, and says once, in a toast, to run `brew install yt-dlp`. Every entry then shows
"needs yt-dlp" with nothing explaining why. And `yt-dlp.exe` is unsigned, so on a Windows 11
with Smart App Control on (the group `windows-signing` is for) it would likely be blocked even
when found. YouTube now also wants a JavaScript runtime for yt-dlp; without one its extraction
is deprecated.

## What Changes

Windows only. macOS and Linux keep how they find, download and ask for yt-dlp today.

- **A managed yt-dlp that Smart App Control can let through**: the app installs python.org's
  embeddable Python (every exe, DLL and `.pyd` in it is signed by the Python Software
  Foundation or Microsoft), yt-dlp's platform-independent zip app (`yt-dlp`, not an exe) and
  Deno (signed by Deno Land Inc.) as yt-dlp's JavaScript runtime. Previews run as
  `python.exe yt-dlp --no-js-runtimes --js-runtimes deno:‹path› …`. Every download is checked
  against a SHA-256 (pinned in the app for Python and Deno, `SHA2-256SUMS` for yt-dlp) before
  anything runs. The zip app is updated daily; Python and Deno change only with a Diggr
  release. It starts faster than `yt-dlp.exe` (measured on Windows 11: 1.1 s against 3.2 s).
- **The user decides, at first launch and whenever previews are needed**: on the first launch
  with no yt-dlp, a modal offers to set up previews (size, source, "your own files play
  without it"), with Install and Not now; the answer is remembered. After that, every user
  action that needs a preview while yt-dlp is missing (play or arm an entry, send or paste a
  page, click an entry's "needs yt-dlp" icon) opens the same modal. Background prefetching
  never opens it.
- **The modal installs in place**: progress, then "Previews are ready", and an armed preview
  starts. A failed install says why (which host couldn't be reached, a checksum mismatch, an
  unpack failure) and offers Try again.
- **An outdated yt-dlp is caught**: when several different clips in a row fail (not
  throttling, not network), the app checks for an update of its own copy at once, and for a
  copy the user installed says it may be out of date.
- **No macOS advice on Windows**: the toast and OPT ▸ Discogs… stop saying `brew` there.

## Capabilities

### New Capabilities
- `ytdlp-setup`: on Windows, when and how the app offers to install yt-dlp, what the modal
  says and does, and the outdated-copy detection.

### Modified Capabilities
- `preview-fetch`: the yt-dlp requirement gains Windows as a managed system (Python, the zip
  app and Deno), asked for as `ytdlp-setup` says; macOS and Linux are unchanged.
- `windows-support` (added by `windows-port`): "yt-dlp is found on Windows" no longer forbids
  offering a download. This change is archived after `windows-port`.

## Impact

- `crates/dig/src/tools.rs`: the Windows install (Python unpacked, the zip app, Deno unpacked),
  the pinned versions and SHA-256s, atomic replacement, the daily update.
- `crates/dig/src/preview/fetcher.rs`: a yt-dlp is a program plus leading arguments.
- `crates/dig/src/preview/scheduler.rs`: consecutive failures for the outdated check.
- `crates/ui/src/app/digging.rs`, `crates/ui/src/app.rs`: the guided modal and its triggers,
  chosen by a `DigSetup` field (on for Windows builds, so the tests on a Mac cover both).
- New dependency: the `zip` crate (`dig`, deflate only; `flate2` is already in the lockfile).
- Downloads on Windows: python.org (13 MB, once), GitHub (3 MB yt-dlp, 40 MB Deno). About
  120 MB on disk in `%LOCALAPPDATA%\Diggr\tools\`.
- `packaging/windows/README.txt`, `README.md`, `CHANGELOG.md`.
- Unchanged: macOS and Linux, the audio path, launch time (the lookup stays off the UI thread).
