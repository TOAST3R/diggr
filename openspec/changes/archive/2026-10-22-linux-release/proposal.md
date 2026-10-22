## Why

Diggr already builds and runs on Linux, but releases are Mac-only, so a Linux user still needs Rust and `cargo run`. Most desktop Linux is x86_64, so one download for it covers nearly everyone: extract a file, run the player, previews included.

## What Changes

- **A Linux x86_64 tarball:** `Diggr-‹version›-x86_64-linux.tar.gz` holds the `diggr` binary (skin built in), a `diggr.desktop` menu entry, the 256 px icon and an `install.sh`. It runs on any glibc distro from Debian 11 / Ubuntu 20.04 onward. You can run `./diggr` from the extracted folder, or run `install.sh` to install it for your user only (`~/.local`, no sudo).
- **One command, built from the Mac:** `scripts/make-linux.sh` builds the binary inside a Docker container with an old glibc, checks it starts in a bare container, and packs the tarball into `dist/`.
- **One release for every platform:** `scripts/release.sh` publishes the Mac disk image and the Linux tarball of a version, both required, plus one `Diggr-‹version›-SHA256SUMS`, in a single GitHub Release. The Open Anyway steps are labelled as Mac steps, and the notes get Linux install steps too. `--unsigned` still refers only to the Mac disk image. **BREAKING** for anyone using the release assets: the per-file `.dmg.sha256` becomes the one checksum file.
- **yt-dlp downloaded for you on Linux too:** the app's own yt-dlp copy, today macOS-only, also works on Linux x86_64. It uses yt-dlp's official `yt-dlp_linux` build, with the same prompt, checksum check and daily updates.
- **The release spec covers both platforms:** `macos-release` becomes `release`. Shared requirements (one-command build, GitHub Release, tag, checksums, nothing binary in git) come first, followed by a macOS section and a Linux section.

Not in this change: aarch64 Linux, AppImage/`.deb`/Flatpak/AUR, Windows, CI builds, and `diggr://` on Linux (opening it would start a second player). On Linux, adding a seller from the browser relies on the running app asking the window manager for focus, which it already does.

## Capabilities

### New Capabilities
- `release`: the release for every platform: the one-command build per platform, publishing on GitHub Releases with one checksum file, and per-platform sections (the macOS app and disk image, carried over from `macos-release`; the Linux x86_64 tarball and its per-user install).

### Modified Capabilities
- `macos-release`: removed; every requirement moves into `release`, unchanged except where publishing now carries several files.
- `preview-fetch`: the app-managed yt-dlp applies on Linux x86_64 as well as macOS.

## Impact

- **`scripts/`:** `make-linux.sh` (new; needs Docker), `release.sh` (publishes all of a version's files, writes `SHA256SUMS`, per-platform notes). `make-dmg.sh` stays as it is.
- **`packaging/linux/`:** `diggr.desktop` and `install.sh` (new).
- **`crates/dig/src/tools.rs`:** the release asset and its size are chosen per OS (`yt-dlp_macos` / `yt-dlp_linux`).
- **`crates/ui/src/app/digging.rs`:** the managed yt-dlp source is turned on for Linux x86_64 too.
- **Docs:** README (Download for Linux, Linux build and install steps in "Build a release", the yt-dlp note, test count) and a CHANGELOG entry. The `changelog` skill needs no change: its notes file feeds `release.sh` as before.
- **No change** to `audio`, `platform`, `analysis` or the browser bridge.
