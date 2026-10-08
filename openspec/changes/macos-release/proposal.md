## Why

Today the player runs only for people who can install Rust and type `cargo run`. To hand it to anyone on a Mac, it needs to be a normal app: one download, drag to Applications, and a first open without "Apple could not verify…". Previews also need `brew install yt-dlp`, which a standard user can't do.

## What Changes

- **A real macOS app:** `Diggr.app` with its name, an original icon and an `Info.plist`, holding one universal binary that runs natively on Apple Silicon and Intel Macs (macOS 11 or later). The skin, scenes and rules are already compiled into the binary, so nothing else is needed.
- **A drag-to-install DMG:** a window with the app and an Applications shortcut, about 19 MB, named `Diggr-‹version›.dmg`.
- **Trusted first launch:** releases are signed with a Developer ID, use the hardened runtime, and are notarized and stapled by Apple. The user sees only the standard "downloaded from the internet" confirmation. Until an Apple Developer account exists, the same script makes unsigned builds for testing, clearly labelled, and refuses to publish them.
- **One command:** `scripts/make-dmg.sh` builds both architectures, merges them, bundles, signs, notarizes and makes the DMG in `dist/`. `scripts/release.sh` tags the version and uploads the DMG to a GitHub Release.
- **Published on GitHub Releases, never in git:** the README gets a "Download for Mac" link to `releases/latest`, and `dist/` is ignored.
- **yt-dlp on first need (BREAKING for the `preview-fetch` spec, which forbids downloading it):** when previews are needed and no yt-dlp is found, the app offers to download it (37 MB), verifies its checksum, keeps it in its cache folder and keeps it up to date. A yt-dlp the user installed is still used first. This applies on macOS; other platforms keep "install it yourself".

Not in this change: Windows and Linux packages, auto-updating the app itself (users download the new DMG), CI builds (the scripts are written to move there later), and the Chrome Web Store.

## Capabilities

### New Capabilities
- `macos-release`: the universal app bundle, the drag-to-install DMG, signing and notarization, the build and release scripts, and publishing on GitHub Releases.

### Modified Capabilities
- `preview-fetch`: the yt-dlp requirement changes from "the user installs it; the app never downloads it" to "the user's yt-dlp is used when present; otherwise the app offers to download, verify and update its own copy (macOS)".

## Impact

- **`packaging/macos/Info.plist.in`:** the bundle's `Info.plist` template (name, identifier, version from `apps/native/Cargo.toml`, minimum macOS).
- **`scripts/`:** `make-dmg.sh` and `release.sh` (new). Tools: `rustup` targets `aarch64-apple-darwin` and `x86_64-apple-darwin`, plus what macOS already has (`lipo`, `hdiutil`, `sips`, `iconutil`, `codesign`, `xcrun notarytool` and `stapler`), and `gh`.
- **`assets/icon/`:** the app's logo (a crate of records), as a cleaned-up `logo.png` and a 1024 px `icon-1024.png`; the `.icns` is made at build time. The same logo becomes the window icon and the Chrome extension's icons.
- **`crates/dig`:** a yt-dlp installer module (latest release lookup, download, SHA-256 check, atomic install, daily update check). The finder looks at the managed copy after the user's own.
- **`crates/ui`:** the "Previews need yt-dlp: download it?" prompt, and OPT ▸ Discogs… showing whether yt-dlp is the user's or the app's, with a download button.
- **Repository:** `.gitignore` gains `dist/`. README: download link, "Build a release" section, the managed yt-dlp location, updated test count.
- **Outside the repo:** an Apple Developer Program membership ($99/year) for signing and notarization, and a `notarytool` keychain profile on the build Mac. Releases are unsigned until these exist.
- **No change** to `audio`, `platform`, `analysis` or `visuals`.
