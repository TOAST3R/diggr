# Proposal

## Why

On macOS the app's own yt-dlp never becomes usable: entries stay "needs yt-dlp" right after a
successful download, and the Mac slows to a crawl. The one-file `yt-dlp_macos` unpacks 102
native libraries into a new `$TMPDIR/_MEI…` folder on every run, and XProtect scans all of them
each time: measured on an idle Mac, `yt-dlp_macos --version` took **49–57 s on every run**,
while the app gives it 10 s. When the time runs out the app kills only PyInstaller's parent
process, so the Python child is orphaned (still waiting on XProtect) and its 71 MB folder is left
behind; 30 s later the app tries again. In one morning this left 17 hung processes, 76 leftover
folders (≈ 5.4 GB) and `XprotectService` at 97 % CPU.

yt-dlp also publishes `yt-dlp_macos.zip`, the same program already unpacked (`yt-dlp_macos` beside
`_internal/`), with its checksum in the same `SHA2-256SUMS`. Unpacked once, it is scanned once:
its first run took 40 s, later runs **0.7–1.0 s**.

## What Changes

- On macOS the app's copy becomes yt-dlp's folder build, `yt-dlp_macos.zip`, verified against its
  published SHA-256 and unpacked whole into `tools/yt-dlp_macos/`, replaced atomically as the
  Windows Python and Deno already are. The download is 54 MB (was 37 MB).
- Installing ends with a first start of the new copy, before it replaces the old one: this pays
  the one-time XProtect scan while the user is shown "Preparing yt-dlp…", checks that the copy
  runs and reports the release it came from, and records that version.
- The app's own copy is no longer started just to be found: it is there when its folder holds the
  program and the version recorded at install. A user's own yt-dlp is still found by running it.
- Every yt-dlp the app starts on macOS and Linux runs in its own process group, and stopping it
  (timeout, cancel, quitting) stops the whole group: no orphaned processes.
- A Mac that already has the old one-file copy (`tools/yt-dlp`) gets the folder build at the next
  look, without being asked again (the user already said yes), and the old file is deleted once
  the folder build is in place.
- Linux and Windows keep their copies (`yt-dlp_linux`, the zip app with Python and Deno).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `preview-fetch`: the yt-dlp requirement changes what the app's macOS copy is (the folder build),
  how it is installed (a first start before it is used), how the app's copy is found (without
  running it), that stopping a yt-dlp stops all its processes, and how an existing one-file copy
  is replaced.

## Impact

- `crates/dig/src/tools.rs`: macOS asset and size, a folder install (shared with the Windows pins'
  unpack and swap), the first start, the recorded version, replacing the old file.
- `crates/dig/src/proc.rs`: process groups on Unix, and stopping a whole group.
- `crates/dig/src/preview/fetcher.rs`: probe, `run` and `fetch` stop the group; the app's macOS
  copy is started from its folder.
- `crates/dig/src/preview/scheduler.rs`: `look` finds the app's copy without running it; the
  one-file copy is replaced in the background.
- `crates/ui/src/app/digging.rs`, `welcome.rs`: the size and the "Preparing yt-dlp…" stage.
- Tests in `crates/dig` (fake folder release, process groups), README (cache layout, test
  count), CHANGELOG.
- No new dependency: `zip` and `sha2` are already used; process groups need only `std` and
  `libc`'s `killpg` (already a transitive dependency, made direct for `dig` on Unix).
