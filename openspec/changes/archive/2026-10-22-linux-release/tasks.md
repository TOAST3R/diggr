## 1. Linux build

- [x] 1.1 Write `scripts/make-linux.sh` (D1): read the version and toolchain, `cargo fetch --locked` on the host, then `docker run --platform linux/amd64 rust:‹toolchain›-bullseye` with the repo at `/src`, the host's `~/.cargo/registry` and `~/.cargo/git` mounted, apt-install `pkg-config libasound2-dev`, and `cargo build --release --locked --offline -p diggr` into `target/linux-x86_64` with symbols stripped. Support `--dry-run` like `make-dmg.sh`. Check: runs on the release Mac (with Zscaler on) and produces an x86_64 ELF (`file`). Record the first-build and incremental times in design.md's open question; if the image tag doesn't exist, apply the fallback from the Risks section.
- [x] 1.2 Add the binary check (D2): fail when `readelf -V` lists a `GLIBC_` version above 2.31, then run `diggr --help` in `debian:bullseye-slim` with only `libasound2` installed. Check: both pass; temporarily building on bookworm makes the glibc check fail (not run: a 40-minute build to prove a negative).

## 2. Linux packaging

- [x] 2.1 Add `packaging/linux/diggr.desktop` (D3: name, `Exec=diggr %F`, icon, categories, audio MIME types, no `x-scheme-handler`). Check: `desktop-file-validate` passes (run in the container).
- [x] 2.2 Add `packaging/linux/install.sh` (POSIX sh, no sudo): binary to `~/.local/bin/`, icon to `hicolor/256x256/apps/diggr.png`, desktop file to `${XDG_DATA_HOME:-~/.local/share}/applications/` with an absolute `Exec`; refresh the desktop and icon caches if the tools exist; print the three paths. Check: `shellcheck` clean; in a `debian:bullseye-slim` container with `HOME` set to a temp folder, it writes exactly those three files, and running it twice gives the same result.
- [x] 2.3 Pack `dist/Diggr-‹v›-x86_64-linux.tar.gz` in `make-linux.sh` with the D3 layout (`diggr`, `diggr.desktop`, `diggr.png` from `assets/icon/icon-256.png`, `install.sh`, all under `Diggr-‹v›/`). Check: `tar tzf` lists exactly those four files; size ≤ 25 MB.

## 3. Publishing

- [x] 3.1 Change `scripts/release.sh` (D4): require both the disk image (same `--unsigned`/`spctl` rules) and the Linux tarball, naming the script for whichever is missing; write `dist/Diggr-‹v›-SHA256SUMS` over both; upload both and the sums file. Check: `--dry-run` with the tarball missing stops and names `make-linux.sh`; with both present it prints both files and the sums; `cd dist && shasum -a 256 -c Diggr-‹v›-SHA256SUMS` passes.
- [x] 3.2 Write the per-platform notes: the given notes, then "macOS" first-open steps (unsigned only) and a "Linux" block (extract, `./install.sh` or `./diggr`, needs ALSA and a Vulkan or OpenGL driver). Check: the notes file a `--dry-run` writes reads correctly with and without `--notes-file`.

## 4. yt-dlp on Linux

- [x] 4.1 In `crates/dig/src/tools.rs`, pick the release asset and its size per OS at compile time (D5): `yt-dlp_macos` on macOS, `yt-dlp_linux` on Linux x86_64, none elsewhere; check `SIZE_MB` against the current release's files. Update the module doc and `preview/fetcher.rs`'s doc. Unit test: the sums parser picks exactly the current OS's asset from a `SHA2-256SUMS` that lists `yt-dlp`, `yt-dlp_macos`, `yt-dlp_linux` and `yt-dlp_linux_aarch64`.
- [x] 4.2 In `crates/ui/src/app/digging.rs`, enable the managed `GitHub` source exactly when there is an asset, and make the "needs yt-dlp" hint offer the download wherever there is one. Check: existing headless yt-dlp tests pass; `cargo check -p ui` compiles for `x86_64-unknown-linux-gnu` in the container.

## 5. Docs and checks

- [x] 5.1 README: "Download for Linux" link next to the Mac one; in "Build a release", `make-linux.sh` (needs Docker), the tarball and what `install.sh` writes (the uninstall paths); the yt-dlp note now says macOS and Linux; the `SHA256SUMS` file; the test count. CHANGELOG `## Unreleased`: a Linux download entry and a yt-dlp-on-Linux entry, written with the `changelog` skill.
- [x] 5.2 Run the four checks from AGENTS.md (`cargo test --workspace`, clippy, fmt, the wasm check), plus `scripts/make-linux.sh` and `scripts/release.sh --unsigned --dry-run` end to end.
- [x] 5.3 Manual check on a real x86_64 desktop (or a VM with GPU acceleration) running Ubuntu 22.04: extract, `./diggr` plays a local file; `./install.sh` → the menu entry has the icon and opens the player; accepting the yt-dlp download makes a Discogs preview play.
