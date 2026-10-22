## Context

- **Releases are Mac-only.** `scripts/make-dmg.sh` builds `dist/Diggr-‹v›-unsigned.dmg`, and `scripts/release.sh` publishes exactly that file and its `.sha256` (see the archived `macos-release` change). There is no CI: releases are built on the maintainer's Apple Silicon Mac, which has Docker installed but no cross toolchain (`zig`, `cargo-zigbuild` and `cross` are all absent).
- **The binary already runs on Linux.** The skin is `include_bytes!`, and `crates/ui/Cargo.toml` turns on winit's `x11` and `wayland` for Linux. Only `diggr` needs shipping. At runtime it links `libasound.so.2` and glibc, and it loads the X11/Wayland and GPU (Vulkan/GL) libraries itself (`dlopen`).
- **What is macOS-only today:** the app-managed yt-dlp (`dig::tools::ASSET = "yt-dlp_macos"`, and `tools: cfg!(target_os = "macos")` in `ui::app::digging`), and `diggr://show` (`dig::bridge::front_url`), which this change leaves Mac-only.
- **The work Mac sits behind Zscaler, which intercepts TLS.** HTTPS from inside a container fails unless the container trusts Zscaler's certificate.
- **The glibc floor decides who can run the binary.** A binary needs at least the glibc it was linked against. Debian 11 (bullseye) has 2.31, the same as Ubuntu 20.04. Debian 12 has 2.36, which is newer than Ubuntu 22.04's 2.35.

## Goals / Non-Goals

**Goals:**
- A Linux x86_64 user downloads one tarball, extracts it, and either runs `./diggr` or installs it with `./install.sh`, with no root and no Rust.
- Previews work on Linux without a package manager, the same as on a Mac.
- One `release.sh` run publishes the Mac and Linux builds of a version together.

**Non-Goals:**
- aarch64 Linux, AppImage, `.deb`, Flatpak, AUR, Windows.
- `diggr://` on Linux. Opening it starts a second process instead of reaching the running one, which would mean building single-instance handling. The running app already asks for focus itself (`ViewportCommand::Focus` + `RequestUserAttention`). X11 window managers usually grant it, and Wayland usually only highlights the window.
- CI builds; a system-wide install; an uninstaller.

## Decisions

### D1. The binary is built in an x86_64 Debian 11 container
`scripts/make-linux.sh` runs `docker run --platform linux/amd64` on `rust:‹toolchain›-bullseye`, where the toolchain is the one the repo builds with. The container installs `pkg-config libasound2-dev` and runs `cargo build --release --locked --offline -p diggr` with `CARGO_PROFILE_RELEASE_STRIP=symbols` and `CARGO_TARGET_DIR=target/linux-x86_64`. The repo is mounted at `/src`.
- **Why Debian 11:** its glibc 2.31 covers Ubuntu 20.04+, Debian 11+, Fedora 32+ and Mint 20+, which is "the standard distros" in practice.
- **No HTTPS inside the container (Zscaler):** the script runs `cargo fetch --locked` on the host first and mounts the host's `~/.cargo/registry` and `~/.cargo/git` into the container, so cargo builds `--offline`. Debian's apt mirrors use plain HTTP, which Zscaler doesn't break. The only HTTPS left is `docker pull`, which goes through Docker Desktop's proxy settings.
- **Its own target dir:** it never mixes Linux objects with the host's `target/release`, and it persists across runs, so later builds are incremental.
- **Why emulation, not cross-compiling:** with `--platform linux/amd64` the container is a real x86_64 Debian, so `pkg-config` and `libasound2-dev` work as on any Linux box. Cross-compiling from an arm64 container (Debian multiarch, `gcc-x86-64-linux-gnu`, `libasound2-dev:amd64`) or with `cargo-zigbuild` would be faster, but adds a linker, sysroot or pkg-config setup to get right. If emulated builds turn out too slow, the multiarch build is the fallback; the script's output stays the same.

### D2. The script checks the binary on a bare system
After building, the script runs `diggr --help` in a fresh `debian:bullseye-slim` container (also `linux/amd64`) that has only `libasound2` installed. That proves the glibc floor holds and that no other shared library is linked. It also runs `readelf -V` and fails if any required `GLIBC_` version is above 2.31. A GUI launch can't be checked there (no display), so that one stays a manual check on a real desktop (tasks).

### D3. The tarball and `install.sh`
```
Diggr-‹v›-x86_64-linux.tar.gz
└── Diggr-‹v›/
    ├── diggr          stripped binary
    ├── diggr.desktop  from packaging/linux/diggr.desktop
    ├── diggr.png      assets/icon/icon-256.png
    └── install.sh     from packaging/linux/install.sh
```
- **`diggr.desktop`:** `Name=Diggr`, `Exec=diggr %F`, `Icon=diggr`, `Categories=AudioVideo;Audio;Player;`, `MimeType=` with the audio types the player opens. It has no `x-scheme-handler/diggr`.
- **`install.sh`** (POSIX sh, no sudo): copies `diggr` to `~/.local/bin/`, the icon to `~/.local/share/icons/hicolor/256x256/apps/`, and the desktop file to `~/.local/share/applications/` with `Exec` rewritten to the absolute path (a desktop session's PATH may not include `~/.local/bin`). It runs `update-desktop-database` and `gtk-update-icon-cache` if they exist. It honours `XDG_DATA_HOME`, and prints the three paths, which is how you uninstall.
- **Runnable without installing:** `./diggr` from the extracted folder works. Installing only adds the menu entry.
- **Why a tarball:** it's the simplest standard format, needs no tooling on the user's side, and leaves AppImage or distro packages open for later.

### D4. `release.sh` publishes every platform's file
- **Files, named exactly** (not globbed, so `notes.md` and old builds never go out): `dist/Diggr-‹v›-unsigned.dmg` (or the signed `Diggr-‹v›.dmg`, with the same `--unsigned` rule and `spctl` check as today) and `dist/Diggr-‹v›-x86_64-linux.tar.gz`. **Both are required.** A missing one stops the release and names the script that builds it, so a version never ships without one of its platforms.
- **One checksum file,** `dist/Diggr-‹v›-SHA256SUMS`, in `sha256sum -c` format. It replaces the per-file `.sha256`.
- **Notes:** the given notes, then the Mac first-open steps (unsigned only), now headed "macOS", then a "Linux" block: extract, `./install.sh` (or run `./diggr`), and "needs ALSA and a Vulkan or OpenGL driver, which desktop distros have".
- The other checks (clean tree, tag unused locally and on origin, `--dry-run`) stay as they are.

### D5. The app's yt-dlp per OS
`dig::tools` picks the release asset at compile time: `yt-dlp_macos` on macOS, `yt-dlp_linux` on Linux x86_64, and none elsewhere. Its size for the prompt is per asset (checked against the current release while implementing). `ui::app::digging` turns on the managed source exactly when there is an asset. Download, `SHA2-256SUMS` check, the atomic install into `tools/`, the `0o755` mode and the daily update are already OS-neutral. The user's yt-dlp still wins. The "needs yt-dlp" hint offers the download on Linux too, and keeps "install it yourself" for Windows.

## Risks / Trade-offs

- [Emulated x86_64 builds on Apple Silicon are slow (the first build could take tens of minutes)] → the target dir persists, so later releases are incremental. The multiarch cross build (D1) is the fallback.
- [`rust:‹toolchain›-bullseye` may not be published for a new toolchain (Debian 11 is near end of life)] → fall back to `debian:bullseye` plus a pinned rustup install done at image-build time. If neither works, moving to bookworm drops Ubuntu 20.04/22.04, and that would be a spec change rather than a quiet move.
- [No GPU driver with Vulkan or GL, e.g. some VMs] → the window can't open. The release notes state the requirement, and it's the same limit `cargo run` has today.
- [Wayland may refuse focus when a seller is added from the browser] → the window is still highlighted. That matches the spec's "or, where the system refuses, draw the user's attention".
- [The release needs both builds, so a broken Docker setup blocks a Mac release] → deliberate. The fix is to repair the build, not to ship one platform alone.

## Migration Plan

Nothing to migrate: the next version is the first with a Linux file. Anyone who fetched `Diggr-‹v›-unsigned.dmg.sha256` uses `Diggr-‹v›-SHA256SUMS` from then on. To roll back, revert the scripts; published releases are unaffected.

## Findings

- **Debian 11 moved to the archive.** `deb.debian.org` still lists Debian 11's security updates but no longer serves them (404). The script points apt at `archive.debian.org`, with `Acquire::Check-Valid-Until=false`. The glibc floor is unchanged, but the build no longer gets Debian's library fixes. The libraries it links to (glibc, ALSA) are the ones on the user's system at runtime, so only build-time headers are affected.
- **Build time on the release Mac:** the first build took 38 min 41 s, with Docker Desktop's Rosetta on (`/run/rosetta/rosetta` is mapped in the container). The Mac's endpoint-security agents (Jamf Protect at up to 90% CPU, BeyondTrust, Nexthink) keep its load average around 200, even with Docker idle. Later builds reuse `target/linux-x86_64/`.
- **The binary needs glibc 2.30** (the check's limit is 2.31). The tarball is 12 MB.
