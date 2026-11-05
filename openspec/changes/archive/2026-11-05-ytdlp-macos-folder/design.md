# Design

## Context

See proposal.md (Why) for the failure and the measurements. The code as it stands:

- `dig::tools` installs the app's yt-dlp. On macOS and Linux it writes one file,
  `tools/yt-dlp` (via `tools/.yt-dlp.part` and a rename). On Windows it also unpacks the pinned
  Python and Deno, each into its own folder, with `install_pin`: unpack into `.‹folder›.part`,
  write a `diggr-version` stamp, move the old folder to `.‹folder›.old`, rename, delete the old.
- `Scheduler::look` (scheduler thread) first asks the finder for a user's yt-dlp, then for the
  app's copy, and the finder runs `‹program› --version` for each, giving it 10 s
  (`YtDlp::version`). On a timeout `Child::kill` sends SIGKILL to that one pid. It runs at least
  every `RECHECK` (30 s) while entries wait, and once after every install or update.
- yt-dlp's macOS builds are PyInstaller programs. The one-file build's bootloader unpacks
  `_internal/` into `$TMPDIR/_MEI‹random›` and starts a second process for Python, which loads
  102 native libraries. Each one is new to macOS on each run, so `dyld` waits in `fcntl` while
  XProtect scans it (seen in `sample`). The folder build (`yt-dlp_macos.zip`: `yt-dlp_macos` and
  `_internal/`, 162 regular files, no symlinks, 54 MB zipped, 129 MB unpacked) loads the same
  libraries from a fixed place, so the scan's result is kept.

## Goals / Non-Goals

**Goals:**
- The app's macOS copy answers in about 1 s from its second start on.
- A slow or stuck yt-dlp can never leave processes or temp folders behind, or build up.
- Finding the app's copy never waits on a process.
- A Mac with the old one-file copy recovers by itself.

**Non-Goals:**
- Linux's copy stays one-file (`yt-dlp_linux`). Linux has no per-file scan, so its start is a
  few seconds, not a minute. Switching to `yt-dlp_linux.zip` would follow the same pattern later.
- Windows process trees (Python → Deno): they'd need a Job Object; Windows behaviour is unchanged.
- Deleting the `_MEI*` folders the old copy already left in `$TMPDIR`. Other PyInstaller programs
  use the same prefix, so the app can't tell which are its own. The changelog tells users this.
- Signing or notarizing yt-dlp, or clearing its quarantine: it's ad-hoc signed by yt-dlp, it
  carries no quarantine attribute (the app downloads it itself), and it ran without a prompt.

## Decisions

### D1. The folder build, unpacked like a pin

macOS's `ASSET` becomes `yt-dlp_macos.zip`, still checked against `SHA2-256SUMS` (which lists it).
Its contents are unpacked into `tools/yt-dlp_macos/`, so the program is
`tools/yt-dlp_macos/yt-dlp_macos`. `install_pin`'s unpack-then-swap is pulled out into one
function, `install_folder(bytes, dir, folder, exe, version, prepare)`, used by both the pins and
the macOS build. Same guarantees: `.part` and `.old` folders, nothing half-installed, the old copy
in use until the rename. `unpack` already keeps Unix modes and refuses entries outside the folder.

- *Alternative: keep the one-file build and give it a longer time limit.* Every start would still
  take ~50 s of a CPU core in XProtect, previews would start a minute late, and each start would
  still leave a 71 MB folder in `$TMPDIR`. Rejected.
- *Alternative: unpack the one-file build ourselves.* That relies on PyInstaller's internals, for
  something yt-dlp already publishes and checksums. Rejected.
- *Folder name:* `yt-dlp_macos`, not `yt-dlp`, because `tools/yt-dlp` is the old file (D5). It
  also matches the release's own name. `YtDlp::at` treats a program as the Windows zip app only
  when it is named `yt-dlp` and has `python/python.exe` beside it, so neither applies here.

### D2. A first start before it's used

Once unpacked, still in `.yt-dlp_macos.part`, the new copy runs `yt-dlp_macos --version`. It has
up to 180 s (the first run took 40 s on an idle Mac), and its whole process group is stopped if
it runs out (D4). Its output must equal the release tag read before the download (`latest_tag`,
or the update's tag). That proves the copy runs, pays XProtect's one-time scan before any preview
waits on it, and gives the version to record. The tag goes into the `diggr-version` stamp before
the swap. The install's progress gets a last stage: the download's percentages, then "Preparing
yt-dlp…" with no percentage. It is sent through the same `u8` progress as the reserved value
`tools::PREPARING` (255, never a percentage), so the scheduler's `YtDlpProgress` event and the
UI's `ytdlp_download` carry it unchanged; one UI helper words both. Release files are fetched
by tag (`releases/download/‹tag›/…`, the tag read first), so the tag the first start must
answer is the release actually downloaded.

Running it from `.part` and then renaming the folder is safe: the scan is keyed to the files, and
a rename keeps them. To be checked in task 1.4: the second run after the rename is ~1 s.

### D3. The app's copy is found without running it

`look` no longer passes the app's copy to the finder. The copy is "there" when
`tools::installed(dir) -> Option<(PathBuf, String)>` sees its program and its stamp, and the
scheduler opens it with `Config::open` (an `Opener`: `YtDlp::at` in the app, the fake fetcher
in tests; the UI passes `DigSetup::opener`). On Linux and Windows the one file's version is
recorded beside it (`tools/yt-dlp.diggr-version`); a copy installed before that has no record
and is still found by starting it, until its next update records one. The
version shown in OPT ▸ Discogs… is the stamp. Nothing runs on the scheduler thread for it, so a
rescan by XProtect (after a reboot or an XProtect update, untested) only makes that preview's
download start later. Downloads already have a 120 s limit and an error path. A copy that turns
out broken fails its downloads, which the outdated check (`ytdlp-setup`'s three failures in a
row, a forced update) already handles.

- *Alternative: probe the app's copy with a longer limit, off the thread.* That's more machinery
  for a file whose checksum the app itself verified and whose start it already watched in D2.

A user's yt-dlp is still found by `--version` with the 10 s limit, as before. `look` already runs
on the scheduler thread, one at a time. That property becomes a requirement ("at most one look"),
and a test keeps it.

### D4. Process groups on Unix

`proc::command` sets `process_group(0)` on Unix (`std::os::unix::process::CommandExt`), so each
helper leads its own group. A new `proc::stop(&mut Child)` sends SIGKILL to the group
(`libc::killpg(pid, SIGKILL)`) and then waits for the child. On Windows it is `Child::kill` as
today. `YtDlp::version`, `run` and `fetch`'s cancel use `proc::stop` instead of `child.kill()`.
This also keeps Ctrl+C in a terminal from reaching the helpers directly. The app stops them itself
when it quits, through the scheduler's cancel.

- *Alternative: `nix`.* It adds a dependency for one call; `libc` is already in the lockfile.
- *Alternative: SIGTERM first so PyInstaller cleans its `_MEI`.* The folder build makes no
  `_MEI`. A user's one-file build would, but a stuck process doesn't answer SIGTERM, which is the
  very case here.

### D5. Replacing the old one-file copy

On macOS, `look` finds no folder build but finds `tools/yt-dlp`, the old file. That means the
user already said yes, so the scheduler starts the same install as Download (once per session;
a failure falls back to the usual "needs yt-dlp" and the question). Lookups during an install
say nothing (the copy is announced when it is in place). The install's progress events reach
the UI as for Download, and entries waiting for yt-dlp wait as "preparing yt-dlp"
(`WaitKind::PreparingYtDlp`, with the queued icon, not the clickable "needs yt-dlp" one).
On success the old file is deleted. The old file is never started, which is the whole problem.

### D6. Sizes and words

`SIZE_MB` becomes 54 on macOS, the download's size, which is what the question promises (on
disk it's 129 MB). The question's "Download it?" wording stays. While a stage is Preparing, the
modal, OPT ▸ Discogs… and the welcome step show "Preparing yt-dlp… (the first start takes up to
a minute)". The `zip` dependency's comment in `crates/dig/Cargo.toml` stops saying "Windows only".

## Risks / Trade-offs

- [XProtect rescans the folder after a reboot or a definitions update, so a start takes ~40 s
  again] → Finding the copy doesn't depend on it (D3). Only that one download starts late, within
  the 120 s download limit. Measured in task 1.4 after a reboot if possible; if it's common,
  a follow-up can warm it in the background at launch.
- [The release stops publishing `yt-dlp_macos.zip`, or renames it] → `expected_sum` fails with
  "SHA2-256SUMS has no entry for yt-dlp_macos.zip", the install fails and says so, and the old copy
  stays. The ignored `real_ytdlp_release_installs_and_runs` test catches it before a release.
- [The folder's layout changes, e.g. the program is renamed] → The install checks that the
  program exists (as `install_pin` checks `pin.exe`) and that D2's start reports the tag. Either
  failure leaves the old copy.
- [129 MB on disk instead of 37 MB] → Accepted. It sits in the cache folder, and the 71 MB
  one-file build used to leave a copy in `$TMPDIR` on every run.
- [`process_group(0)` means a helper doesn't get the terminal's Ctrl+C] → The app's own shutdown
  cancels running helpers. A test checks that none are left after the scheduler stops.

## Migration Plan

Automatic (D5): the next launch on a Mac with `tools/yt-dlp` installs the folder build in the
background and deletes the file. Rollback: an older app version finds no `tools/yt-dlp`, asks
again, and installs its one-file build next to the folder, and the two never use each other's
paths. The `_MEI*` folders already in `$TMPDIR` are left alone (see Non-Goals); macOS clears
`$TMPDIR` of old files by itself, and the changelog says so.
