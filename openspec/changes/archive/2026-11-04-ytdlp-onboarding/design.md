## Context

- Previews come from yt-dlp. `crates/dig/src/tools.rs` manages the app's own copy on macOS and
  x86_64 Linux (`MANAGED`): the official one-file build, checked against `SHA2-256SUMS`, kept in
  `tools/` in the cache folder, replaced atomically, updated at most daily. A user's own yt-dlp
  (configured path, PATH, usual folders) always wins and is never touched.
- Windows is not managed. A Windows user without yt-dlp gets a toast saying
  `brew install yt-dlp` once per session, and entries that say "needs yt-dlp".
- The question to download (`ask-ytdlp`, an `egui::Modal`) exists, but is shown only where the
  app can download, and only the first time the scheduler reports `NeedsYtDlp` in a session.
  It closes on Download; progress shows in the status line.
- Measured (findings.md): `yt-dlp.exe` is unsigned; the embeddable Python's 34 binaries and
  `deno.exe` are signed, and Windows 11 reports them Valid; the zip app on the embeddable
  Python fetches a YouTube m4a (with Deno, no runtime warning) and a Bandcamp mp3; it starts in
  1.14 s against `yt-dlp.exe`'s 3.24 s, and a preview takes 5.04 s against 6.36 s.
- Smart App Control (Windows 11) checks every process start: signed by a trusted certificate,
  or known to Microsoft's cloud, or blocked, with no way past it.

## Goals / Non-Goals

**Goals:**
- A Windows user can install yt-dlp from inside the app, with one click, at first launch or at
  any moment previews are needed.
- Everything the app starts for previews on Windows is signed.
- The question appears when the user wants sound, never while they just browse.
- A stale yt-dlp is noticed and fixed or explained.

**Non-Goals:**
- Any change on macOS or Linux: their managed copy and their question stay as they are
  (decided by the maintainer on 2026-11-04, after trying the guided modal there).
- A runtime for a yt-dlp the user installed themselves.
- An installer (NSIS, MSI), yt-dlp inside the release zip, or signing yt-dlp ourselves.
- arm64 Windows, ffmpeg, yt-dlp's optional extras.

## Decisions

### 1. Signed embeddable Python plus the yt-dlp zip app
`tools/python/` holds python.org's embeddable Python, unpacked; `tools/yt-dlp` is the zip app.
A preview runs `tools/python/python.exe tools/yt-dlp <args>`, started through
`dig::proc::command` (no console window). Its `python315._pth` keeps it apart from any other
Python on the machine.

Alternatives: `yt-dlp.exe` (unsigned, slower to start, and each daily update brings the
least-known file to Smart App Control); `yt-dlp_win.zip` (its exe is unsigned too).

### 2. Deno as the JavaScript runtime
`tools/js/deno.exe`, from Deno's GitHub release zip, signed by Deno Land Inc. yt-dlp's default
and fastest runtime. The managed copy is always started with
`--no-js-runtimes --js-runtimes deno:‹absolute path›` among its leading arguments. Node is the
same size and signed too, but second in yt-dlp's order; QuickJS is small but unsigned and slower.

### 3. Pins and checks
Python's and Deno's version, address and SHA-256 are constants in `tools.rs`, changed with a
Diggr release (a release checklist line; an ignored test downloads each real file and checks
its pin). The zip app is checked against `SHA2-256SUMS` and updated at most daily. A version
file inside each unpacked folder tells whether it is the pinned one; an older one is replaced
on the next install or update.

### 4. Unpacking with the `zip` crate
The `zip` crate (`dig`, `default-features = false`, deflate only) unpacks each zip into
`tools/.‹name›.part/`, which then replaces `tools/‹name›/` (the old one is moved aside first,
so a failure at any step leaves it working). An entry that would land outside its folder fails
the unpack. Alternative: Windows' `tar.exe`: a program to start, and code only the spike could
test.

### 5. A yt-dlp is a program plus leading arguments
`YtDlp { program, lead }`. `YtDlp::at(path)` recognises the managed layout (`tools/yt-dlp` with
`tools/python/python.exe` beside it) and splits it into Python plus the zip app and the
runtime arguments; anything else is a program of its own. So the scheduler's lookup (the
user's yt-dlp first, then `tools/yt-dlp`) and its `Finder` stay as they are.

### 6. The guided setup, Windows only
A `DigSetup` field, `guided_ytdlp`, true on Windows builds, picks it; the headless tests on a
Mac switch it on to cover it. With it:
- **First launch**: while `settings.ytdlp_setup_asked` isn't set, the UI asks the scheduler to
  look for yt-dlp as soon as previews start (on the scheduler's thread), and opens the modal if
  none is found. Install or Not now sets the flag.
- **User actions**: playing or arming a waiting entry whose preview needs yt-dlp, sending a
  page from the extension, pasting an address that makes a crate, clicking an entry's "needs
  yt-dlp" icon. One function, `dig_wants_ytdlp`, is called from each; if the lookup hasn't
  answered yet, the modal opens when it does. Store samples (Juno, Beatport) don't count.
  Background refreshes and prefetching never call it.
- **The modal**: the question (size, source, own files play without it, the Python note),
  then "Installing yt-dlp: N%", then "Previews are ready" (2 s) or the failure with Try again.
  Not now and Escape close it until the next such action.
Without it (macOS, Linux) the modal is the one from before this change.

### 7. Outdated detection from consecutive failures
With the guided setup, the scheduler counts consecutive downloads that end in
`FetchError::Failed` on different clips, reset by any success; throttling, network errors and
store samples don't count. At 3: the app's own copy gets an update check at once (at most once
an hour); for a user's copy, the UI is told its version and says once a session, if it is more
than 60 days old, that it may be out of date and where to get a newer one.

## Risks / Trade-offs

- **Smart App Control blocks something after all** → not testable here (no machine with it
  on); Windows reports every signature Valid. Confirmed in the first pre-release with a tester
  who has it on; a block is a finding before promotion.
- **Size**: about 56 MB to download and 120 MB on disk, mostly Deno → stated in the modal.
- **Two hosts** (python.org, GitHub) → the error names the one that failed.
- **Pinned Python and Deno go stale** → release checklist line and the ignored pin tests.
- **The managed copy writes "DASH m4a" without ffmpeg** → checked in the Windows test (7.2).
- **The Mac signature check** (osslsigncode against Homebrew's `cert.pem`) is `windows-signing`'s.

## Migration Plan

1. Land the code; Windows users with a yt-dlp of their own keep using it.
2. A pre-release; on first launch, Windows users without yt-dlp see the modal once.

Rollback: the previous release; `tools/` is inside the cache folder and can be deleted.

## Open Questions

- Does Smart App Control allow `python.exe`, its `.pyd` files and `deno.exe`? (Pre-release.)
