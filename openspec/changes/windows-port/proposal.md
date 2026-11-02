## Why

Diggr has only ever been built and run on macOS and Linux. The README already lists Windows
folders and the code already switches `APP_DIR` for it, but nobody has compiled the app for
Windows, let alone run it. We need to know that the player starts, plays within the latency
targets and digs previews on a real Windows 10 machine, and fix what doesn't. The maintainer
builds everything on one Mac and has one Windows 10 laptop to test on, which has no developer
tools. So the port is cross-built from the Mac, published as a GitHub Release, downloaded in
that laptop's browser (the way a user would get it), and confirmed by hand.

## What Changes

- A spike first: cross-build `diggr.exe` from the Mac for `x86_64-pc-windows-msvc` and find out
  what fails before designing any fix beyond the known ones. Every failure is written down as a
  finding.
- Cross-build setup: `cargo-xwin` and the target, with the C runtime linked statically
  (`+crt-static`) so a fresh Windows 10 needs no Visual C++ Redistributable, and a check that
  the exe imports only system DLLs.
- Console behaviour: the GUI opens without a console window, while `--help`, `--bench`,
  `--tui`, `--startup-time` still print to the terminal they were started from.
- Quiet subprocesses: yt-dlp (previews, version checks) never flashes a console window.
- The exe carries the digging-D mark as its icon, generated from the logo like every other
  raster.
- Anything else the spike turns up that is specific to Windows (paths, file handling, the
  browser bridge, audio) is fixed in the same change, or recorded as a known gap when it
  cannot be fixed from the Mac.
- A compile-level check for the Windows target from the Mac (`cargo xwin check --workspace
  --all-targets`, clippy), since the tests themselves cannot run there.
- **A new release, v0.1.4, with a Windows download.** `scripts/make-windows.sh` builds
  `Diggr-0.1.4-x86_64-windows.zip` (`Diggr.exe` and a `README.txt` with the first-open steps);
  `scripts/release.sh` publishes it with the macOS and Linux files and one SHA256SUMS for all
  three. The version is bumped, `CHANGELOG.md` gets its 0.1.4 section with the Windows entry,
  and the release is made as a **pre-release** so `releases/latest` (and the README's download
  links) keep leading to the last confirmed release until the maintainer has tried the Windows
  download on the laptop and confirmed it works. Confirming promotes it to a normal release.
- Unsigned, with the SmartScreen "More info ▸ Run anyway" steps in the notes and the zip.

Not in this change: signing and Smart App Control (Windows 10 has none; signing is a later
change, see the exploration notes), an installer, a managed yt-dlp for Windows (Windows users
install it themselves, as the preview spec already says, until the spike shows whether
`yt-dlp.exe`'s start time makes a managed copy worth it), native arm64, and testing on Windows
11 (expected to work, not tested). The README's "Download for Windows" link is added only when
the release is promoted.

## Capabilities

### New Capabilities
- `windows-support`: what it means for Diggr to run on Windows 10 x86_64: how it is
  cross-built, that it meets the playback latency targets, its console and subprocess
  behaviour, yt-dlp lookup, data folders and its icon, and how the first run is recorded.

### Modified Capabilities
- `release`: a third platform. Adds the Windows zip and its first-open steps, adds
  `scripts/make-windows.sh` to the one-command build, and changes the publishing requirement
  to three files, a Windows section in the notes, and a test (pre-)release that is promoted
  once confirmed.

## Impact

- `apps/native`: `#![windows_subsystem = "windows"]` with a console attach for the CLI modes,
  an icon resource (build script), the version bump to 0.1.4.
- `crates/dig`: yt-dlp spawns (`preview/fetcher.rs`, `tools.rs`) go through one helper that sets
  the no-window creation flag on Windows; the yt-dlp lookup is checked for `yt-dlp.exe`.
- `crates/ui`: `icon-gen` also writes the `.ico` (and a test checks it, as for the PNGs).
- `.cargo/config.toml`: static CRT for the Windows target.
- `scripts/make-windows.sh` (new), `scripts/release.sh` (third file, `--prerelease`), a
  `packaging/windows/README.txt`.
- Dependencies: Windows-only crates behind `cfg(windows)`, so macOS, Linux and the wasm check
  are unaffected. `audio` and `platform` keep compiling for `wasm32-unknown-unknown`.
- Maintainer tooling (not in the repo's build): `cargo-xwin`, Homebrew `llvm`, possibly `nasm`.
- Outward-facing: pushes tag `v0.1.4` and creates a GitHub Release. Done only when the
  maintainer says go, after the Mac-side checks pass.
- Docs: README (Windows build and test notes, tests count if it changes, the download link at
  promotion), CHANGELOG (0.1.4, which also carries the Spanish record stores entry that is
  currently unreleased).
