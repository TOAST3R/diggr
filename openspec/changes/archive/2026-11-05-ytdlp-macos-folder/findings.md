# Findings

Measured on 2026-11-05 on the maintainer's Mac (Intel x86_64, macOS 15.7.4), yt-dlp release
2026.10.26.

## What happened

- The app's one-file copy, `~/Library/Caches/diggr/tools/yt-dlp` (37 466 304 bytes, universal,
  ad-hoc signed, no quarantine attribute, only `com.apple.provenance`), was installed at 08:16.
  At 08:17 entries said "needs yt-dlp".
- `ps`: the app had started `yt-dlp --version` about every 30 s; 17 were running, 15 of them with
  parent pid 1 (orphaned). `XprotectService` was at 91–97 % CPU. After the probes were killed and
  the app quit, it went to 0 %.
- `sample` of a hung probe (the PyInstaller child): Python's import → `_imp_create_dynamic` →
  `dlopen` → `dyld4::Loader::mapSegments` → `fcntl`, waiting for the kernel's check of a freshly
  written library.
- `$TMPDIR` held 76 `_MEI*` folders of 71 MB each (≈ 5.4 GB), created between 01:08 and 08:20,
  one per killed run. They were deleted by hand; free space went from 62 to 67 GB.

## Start times (`--version`, one at a time, nothing else running, process group killed at 120 s)

| Build | Run | Time |
|---|---|---|
| one-file `yt-dlp_macos` (the installed copy) | 1 | 57.1 s |
| one-file `yt-dlp_macos` | 2 | 49.2 s |
| folder `yt-dlp_macos.zip`, unpacked with `unzip` | 1 | 40.4 s |
| folder | 2 | 1.0 s |
| folder | 3 | 0.7 s |

Every run printed `2026.10.26`. The one-file build never gets faster: each run unpacks to a new
`_MEI*` folder that is scanned again.

## The release

- `SHA2-256SUMS` of 2026.10.26 lists `yt-dlp_macos`
  (`0f922c15…efe0aa`) and `yt-dlp_macos.zip` (`20d98fa6…de3ef1`); the downloaded zip's SHA-256
  matched.
- `yt-dlp_macos.zip`: 54 165 284 bytes, 162 entries (129 401 729 bytes unpacked): the program
  `yt-dlp_macos` at the top and `_internal/` beside it (102 `.so`/`.dylib`, `Python`,
  `Python.framework/…`). No symlinks: every entry is a regular file or a folder (the
  `Python.framework` "links" are 14.7 MB copies). Executables carry mode 0755 in the zip.
- The release also has `yt-dlp_linux.zip`, `yt-dlp_linux_aarch64.zip` and `yt-dlp_musllinux.zip`
  (for a possible Linux follow-up).

## Checks for the design

- 1.2 (start, then rename; design D2): a fresh unpack into `tools/.yt-dlp_macos.part/`, started
  once there (57.9 s), then renamed to `tools/yt-dlp_macos/`: the next starts took **1.5 s and
  1.2 s**. The scan's result survives the rename, so the first start can run before the swap.
  No `_MEI*` folder was created. The first start varies (40.4 s and 57.9 s): the 180 s limit
  leaves room for a slower or busier Mac.
- 1.3 (first start after a reboot; design risk 1): not measured (no reboot during this work).
  Finding the app's copy doesn't depend on it (D3); at worst one download starts late.

## The implementation, against the real release

- `cargo test -p dig -- --ignored real_ytdlp` (2026.10.26 from GitHub, through `tools::install`):
  download, checksum, unpack, first start and swap in **51.4 s** in all; the next start of the
  installed copy took **1.27 s**; no yt-dlp process was left afterwards.

## In the app (task 6.2)

- `cargo run --release -p diggr` on this Mac with the old one-file `tools/yt-dlp` in place: the
  app replaced it by itself with `tools/yt-dlp_macos/` (stamped `2026.10.26`, installed at
  10:56), the old file is gone, and previews download (confirmed by the maintainer: "working
  now"). Afterwards no `yt-dlp` process was left once downloads ended, and `$TMPDIR` held no
  `_MEI*` folder.
