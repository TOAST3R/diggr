## Context

- **The binary is already self-contained.** The skin (`include_bytes!` in `crates/ui/src/skin`), the scenes, prelude and director rules (`include_str!` in `crates/visuals`) are compiled in. User data lives in the platform folders from `dirs` (`~/Library/Application Support/winamp_rust/`, `~/Library/Caches/winamp_rust/`). An app bundle only has to wrap one executable.
- **Size today:** the Apple Silicon release binary is 25.3 MB, 20.1 MB stripped, about 9 MB compressed. yt-dlp's macOS build (`yt-dlp_macos`, universal) is 37.1 MB and barely compresses.
- **The build Mac:** only the `aarch64-apple-darwin` target is installed, there is no signing identity yet, and `gh` is set up for this public repository, which has no releases yet.
- **yt-dlp today:** `dig::preview::fetcher::find` tries the configured path, `yt-dlp` on the PATH, then `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin` and `~/.local/bin`. That list already exists because an app opened from the Finder doesn't get the shell's PATH. The `preview-fetch` spec forbids downloading it, and this change reverses that on macOS.
- **Previews need no ffmpeg:** the format chosen is audio only, so yt-dlp is the only external program.

## Goals / Non-Goals

**Goals:**
- A standard user downloads one file, drags one icon, and opens the app with one standard confirmation.
- Discogs previews work without Homebrew or a terminal.
- A release is reproducible from a tag with one build command and one publish command, and moving it to CI later is easy.

**Non-Goals:**
- Windows or Linux packages.
- Auto-updating the app (Sparkle); users download the next DMG.
- CI release builds.
- The rebrand name. "winamp_rust" stays a placeholder, and renaming moves the config folders (a later migration).
- File associations or opening files dropped on the Dock icon.
- A styled DMG window with a background picture.

## Decisions

### D1. The bundle is assembled by the script, not by `cargo-bundle`
The script builds `winamp-native` twice (`--target aarch64-apple-darwin` and `--target x86_64-apple-darwin`, with `MACOSX_DEPLOYMENT_TARGET=11.0` and `CARGO_PROFILE_RELEASE_STRIP=symbols`), joins them with `lipo -create`, and writes the bundle:

```
dist/‹App›.app/Contents/
  Info.plist          from packaging/macos/Info.plist.in (name, id, version filled in)
  MacOS/winamp-native universal, stripped
  Resources/AppIcon.icns
```
- **`Info.plist` keys:** `CFBundleName` / `CFBundleDisplayName` (‹App›), `CFBundleIdentifier` (`io.github.toast3r.winamp-rust` until the rebrand), `CFBundleShortVersionString` and `CFBundleVersion` (from `apps/native/Cargo.toml`), `CFBundleExecutable`, `CFBundleIconFile`, `LSMinimumSystemVersion` 11.0, `NSHighResolutionCapable`, `LSApplicationCategoryType` (`public.app-category.music`).
- **No usage descriptions:** the player opens no input stream and needs no protected resource.
- **Why not `cargo-bundle`:** it builds one target at a time, and making a universal bundle means post-processing its output anyway. A 20-line template is easier to read and review than tool configuration, and it's one tool fewer to install.

### D2. The DMG is made with `hdiutil`
The script copies the app into a staging folder next to a symlink to `/Applications`, then runs `hdiutil create -format UDZO -volname ‹App›`. Opening the image shows both icons.
- **Why not `create-dmg`:** it adds a styled window with a background and icon positions, but it's an extra Homebrew dependency. That's worth adding later, not needed for "drag one onto the other".

### D3. Signing and notarization when credentials exist
- **Credentials:** two environment variables. `DEVELOPER_ID` holds the certificate name ("Developer ID Application: … (TEAMID)"), and `NOTARY_PROFILE` names a keychain profile created once with `xcrun notarytool store-credentials`. No secret is ever written in the repository.
- **Signed build:** `codesign --force --options runtime --timestamp --sign "$DEVELOPER_ID"` on the binary, then on the app. Then `ditto -c -k` of the app → `notarytool submit --wait` → `stapler staple` of the app. Then the DMG is built, signed, submitted and stapled. Last, `spctl --assess` checks the app and the DMG, and the build fails if either is rejected.
- **Unsigned build:** the app is signed ad hoc (`codesign --sign -`, which Apple Silicon requires to run at all). The DMG is named `‹App›-‹version›-unsigned.dmg`, and the script prints how testers can open it (System Settings ▸ Privacy & Security ▸ Open Anyway).
- **Entitlements:** none. The hardened runtime with no exceptions suffices: no JIT, no unsigned libraries loaded into the process (yt-dlp runs as a separate process).

### D4. Releasing
`scripts/release.sh` checks, in order:
- the working tree is clean;
- the version in `apps/native/Cargo.toml` has no tag yet;
- `dist/‹App›-‹version›.dmg` exists and passes `spctl` (unless `--unsigned` is given).

Then it writes the SHA-256 file, creates and pushes tag `v‹version›`, and runs `gh release create` with the DMG, its checksum and the notes (`--notes-file`, or an editor). The README's download link is `https://github.com/TOAST3R/winamp_rust/releases/latest`.

### D5. The app's own yt-dlp (`dig::tools`)
```
need previews ─▶ user's yt-dlp found? ── yes ─▶ use it (never updated by the app)
                     │ no (macOS)
                     ▼
        managed copy in <cache>/tools/yt-dlp works? ── yes ─▶ use it
                     │ no
                     ▼
        ask once: "Previews need yt-dlp, a free downloader (37 MB). Download it?"
                     │ yes
                     ▼
        GET releases/latest/download/SHA2-256SUMS  → expected hash of yt-dlp_macos
        GET releases/latest/download/yt-dlp_macos  → <cache>/tools/.yt-dlp.part
        sha256 == expected ? chmod 755, rename → tools/yt-dlp : delete, report
```
- **Order:** the finder's candidate list gets the managed copy after the user's paths, so a Homebrew yt-dlp always wins.
- **Sources:** only `https://github.com/yt-dlp/yt-dlp/releases/latest/download/…`, over HTTPS with rustls (the `ureq` agent `dig` already has). The file's name comes from a constant, never from anything downloaded. `SHA2-256SUMS` is parsed for exactly that name.
- **What the checksum proves:** it catches corruption and a swapped file on the download host, but not a compromised yt-dlp release, since the sums come from the same place. Checking yt-dlp's GPG signature (`SHA2-256SUMS.sig`) would close that gap, at the cost of carrying a key and an OpenPGP verifier. It's left as an open question.
- **Updates:**
  - At most once a day, and only while previews are being fetched, the tools worker resolves the tag of `releases/latest` (from the redirect's `Location`, so there's no API call and no rate limit). It compares that with `yt-dlp --version`.
  - When the release is newer, it downloads it the same way and swaps it in with a rename; downloads already running keep the old file's inode.
  - The date of the last check is kept in `tools/checked`.
  - `yt-dlp -U` isn't used, so every copy passes the same verification path.
- **Quarantine:** files written by the app itself don't get the `com.apple.quarantine` attribute, so Gatekeeper doesn't block running the managed copy.
- **Threads and tests:** it runs on the preview scheduler's low-priority worker, behind a small `ToolSource` trait (`get(url) -> bytes`, `latest_tag()`), with a fake for tests. Nothing runs before the first frame, and the download happens only after the user says yes.
- **UI:**
  - The first `NeedsYtDlp` on macOS opens a small dialog: Download (37 MB) / Not now.
  - Not now remembers the answer for the session.
  - OPT ▸ Discogs… shows "yt-dlp 2026.08.19 (installed by you)" or "(downloaded by the app)", with Download or Update now.
  - Progress shows in the status line.

### D6. The icon
- **Art:** original, made to match the player: a dark panel with a green LCD record. It's drawn by a small generator (`cargo run -p ui --bin icon-gen`, alongside `skin-gen`) into `assets/icon/icon-1024.png`, which is committed.
- **`.icns`:** the build script makes it from the PNG with `sips` and `iconutil` (both part of macOS), so only one image lives in git.
- **Test:** checks that the committed PNG matches the generator, as for the skin.

## Risks / Trade-offs

- **[yt-dlp's release layout changes]** (asset or checksum file renamed) → The download fails with a clear message, the user's own yt-dlp still works, and a test pins the names so a change is noticed in one place.
- **[Running a downloaded executable]** → Only after the user agrees, only from yt-dlp's official releases over HTTPS, only with a matching SHA-256, and invoked as today (no shell, `--ignore-config`, validated clip ids). The GPG signature check is an open question.
- **[No Apple Developer account yet]** → Unsigned builds for testing only; `release.sh` refuses to publish them without `--unsigned`. Testers get the Open Anyway steps.
- **[x86_64 build problems]** (a C dependency that doesn't cross-compile) → All C dependencies are built by `cc` with Apple's clang, which targets both architectures. The first build task proves it early.
- **[Launch time inside a bundle]** → The first open after installing includes Gatekeeper's check (seconds, once). The 300 ms target applies from the second open, and is measured with the bundle's own binary (`--startup-time`).
- **[The placeholder name ships to users]** → Renaming later changes the config folder and the bundle identifier. It's documented as a later migration, and publishing to strangers waits for the name.
- **[Size]** → About 19 MB for the DMG (two architectures of a 20 MB stripped binary, compressed). The spec allows 25 MB.

## Migration Plan

- **First release:** v0.1.0, unsigned, for testing on a few Macs (including an Intel one if possible) with the Open Anyway steps. Signed and notarized releases follow once the Apple account exists.
- **Rollback:** a bad release is deleted or marked as a pre-release on GitHub. The previous DMG stays downloadable from its own release. Users' data folders don't change between versions.

## Open Questions

- **The rebrand name.** It sets the app, DMG and volume names and the bundle identifier. Everything reads it from one place in the script, so it's a one-line change until the first public release.
- **The Apple Developer account** ($99/year): when?
- **yt-dlp GPG signature check:** worth adding before public releases, or is SHA-256 over HTTPS enough?
- **Styled DMG window** (background art, icon positions) with `create-dmg`: later?
