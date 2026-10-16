## Context

- **The binary is already self-contained.** The skin (`include_bytes!` in `crates/ui/src/skin`), the scenes, prelude and director rules (`include_str!` in `crates/visuals`) are compiled in. User data lives in the platform folders from `dirs` (`~/Library/Application Support/Diggr/`, `~/Library/Caches/Diggr/`). An app bundle only has to wrap one executable.
- **Size today:** the Apple Silicon release binary is 25.3 MB, 20.1 MB stripped, about 9 MB compressed. yt-dlp's macOS build (`yt-dlp_macos`, universal) is 37.1 MB and barely compresses.
- **The build Mac:** only the `aarch64-apple-darwin` target is installed, there is no signing identity, and there won't be one: the Apple Developer Program costs $99/year, which this project doesn't pay, and `gh` is set up for this public repository, which has no releases yet.
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
- File associations or opening files dropped on the Dock icon.
- A styled DMG window with a background picture.

## Decisions

### D1. The bundle is assembled by the script, not by `cargo-bundle`
The script builds `diggr` twice (`--target aarch64-apple-darwin` and `--target x86_64-apple-darwin`, with `MACOSX_DEPLOYMENT_TARGET=11.0` and `CARGO_PROFILE_RELEASE_STRIP=symbols`), joins them with `lipo -create`, and writes the bundle:

```
dist/Diggr.app/Contents/
  Info.plist          from packaging/macos/Info.plist.in (name, id, version filled in)
  MacOS/diggr universal, stripped
  Resources/AppIcon.icns
```
- **`Info.plist` keys:** `CFBundleName` / `CFBundleDisplayName` (Diggr), `CFBundleIdentifier` (`io.github.toast3r.diggr`), `CFBundleShortVersionString` and `CFBundleVersion` (from `apps/native/Cargo.toml`), `CFBundleExecutable`, `CFBundleIconFile`, `LSMinimumSystemVersion` 11.0, `NSHighResolutionCapable`, `LSApplicationCategoryType` (`public.app-category.music`).
- **No usage descriptions:** the player opens no input stream and needs no protected resource.
- **Why not `cargo-bundle`:** it builds one target at a time, and making a universal bundle means post-processing its output anyway. A 20-line template is easier to read and review than tool configuration, and it's one tool fewer to install.

### D2. The DMG is made with `hdiutil`
The script copies the app into a staging folder next to a symlink to `/Applications`, then runs `hdiutil create -format ULMO -volname Diggr`. Opening the image shows both icons.
- **Why LZMA (`ULMO`):** zlib's `UDZO` made a 24.9 MB image, just under the 25 MB limit; LZMA makes 16.6 MB. It opens on macOS 10.15 and later, and the app needs 11.
- **Why not `create-dmg`:** it adds a styled window with a background and icon positions, but it's an extra Homebrew dependency. That's worth adding later, not needed for "drag one onto the other".

### D3. Signing: ad hoc by default, Developer ID only if it ever exists
- **Credentials:** two environment variables. `DEVELOPER_ID` holds the certificate name ("Developer ID Application: … (TEAMID)"), and `NOTARY_PROFILE` names a keychain profile created once with `xcrun notarytool store-credentials`. No secret is ever written in the repository.
- **Signed build:** `codesign --force --options runtime --timestamp --sign "$DEVELOPER_ID"` on the binary, then on the app. Then `ditto -c -k` of the app → `notarytool submit --wait` → `stapler staple` of the app. Then the DMG is built, signed, submitted and stapled. Last, `spctl --assess` checks the app and the DMG, and the build fails if either is rejected.
- **Unsigned build (the release path):** the app is signed ad hoc (`codesign --sign -`, which Apple Silicon requires to run at all). The DMG is named `Diggr-‹version›-unsigned.dmg`, and the script prints how users open it (System Settings ▸ Privacy & Security ▸ Open Anyway).
- **Why unsigned is enough:** Gatekeeper refuses the first open of a quarantined app it can't verify, once; Open Anyway records the exception and later opens are normal. macOS 15 removed the old right-click ▸ Open bypass, so Privacy & Security is the documented way. Homebrew casks hit the same check, so they don't avoid it. The signed path above stays in the script, untested, because it costs nothing to keep.
- **Entitlements:** none. The hardened runtime with no exceptions suffices: no JIT, no unsigned libraries loaded into the process (yt-dlp runs as a separate process).

### D4. Releasing
`scripts/release.sh` checks, in order:
- the working tree is clean;
- the version in `apps/native/Cargo.toml` has no tag yet;
- with `--unsigned` (the normal case), `dist/Diggr-‹version›-unsigned.dmg` exists; without it, `dist/Diggr-‹version›.dmg` exists and passes `spctl`. The flag stays required so one kind is never published as the other.

Then it writes the SHA-256 file, creates and pushes tag `v‹version›`, and runs `gh release create` with the DMG, its checksum and the notes (`--notes-file`, or an editor). An unsigned release is a normal release, not a pre-release, because `releases/latest` skips pre-releases and the README links there; the script appends the Open Anyway steps to its notes. The README's download link is `https://github.com/TOAST3R/diggr/releases/latest`.

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
- **Why not bundle it in the DMG:**
  - It goes stale fast. yt-dlp ships fixes every few weeks as sites change, so a bundled copy would break previews between app releases, and the updater would be needed anyway.
  - Size. It's 37 MB and barely compresses, which would take the DMG from about 19 MB to about 55 MB, over the spec's 25 MB.
  - Signing. Everything inside the bundle must be signed with our Developer ID and notarized. That would mean re-signing a third-party binary, and swapping in an update would break the bundle's signature. A copy in the cache folder sits outside the bundle.
  - Not everyone uses previews. People who never dig don't pay for it.
- **Quarantine:** files written by the app itself don't get the `com.apple.quarantine` attribute, so Gatekeeper doesn't block running the managed copy.
- **Threads and tests:** it runs on the preview scheduler's low-priority worker, behind a small `ToolSource` trait (`get(url) -> bytes`, `latest_tag()`), with a fake for tests. Nothing runs before the first frame, and the download happens only after the user says yes.
- **UI:**
  - The first `NeedsYtDlp` on macOS opens a small dialog: Download (37 MB) / Not now.
  - Not now remembers the answer for the session.
  - OPT ▸ Discogs… shows "yt-dlp 2026.08.19 (installed by you)" or "(downloaded by the app)", with Download or Update now.
  - Progress shows in the status line.

### D6. The icon
- **Art:** a D whose bowl is a record, with a hand digging through it, in near-black (`#111`) on cream (`#F5F5EC`). It comes in two forms, both traced to SVG from the supplied images: the **mark** (`openspec/changes/macos-release/logo-mark.svg`) and the **lockup**, the mark over the "Diggr" wordmark (`logo-lockup.svg`). They move to `assets/icon/`, and the SVGs are the only hand-edited sources.
- **Which form where:**
  - The mark goes wherever the logo is small or square: the bundle's `.icns`, the window and Dock icon in development builds, and the Chrome extension's four icon sizes. Text in an icon is unreadable below about 64 px, and Apple's guidelines advise against it.
  - The lockup goes wherever there's room for the name: the README header (and the landing page).
- **macOS layout:** `icon-1024.png` is a cream rounded square on Apple's icon grid (824 px body centred in 1024, corner radius about 185 px), with the mark centred in it. The badge is the icon's shape, so there's no frame inside a frame.
- **Generated PNGs:** a small `icon-gen` binary (next to `skin-gen` in `crates/ui`) renders the mark with `resvg` (pure Rust) into `icon-1024.png`, `icon-256.png` and the extension's `icon{16,32,48,128}.png`. The PNGs are committed so builds and the app need no SVG renderer, and `resvg` is only a dependency of the generator. At 16 and 32 px the record grooves merge, but the D's silhouette still reads.
- **Dock icon at launch:** eframe sets the Dock icon only from `ViewportBuilder::with_icon` at window creation, and puts egui's own icon there when given none, even inside the bundle. So it can't wait for the first frame. Decoding the 1024 px PNG there cost about 140 ms of launch, so:
  - inside `Diggr.app` (the executable's path contains `.app/Contents/MacOS/`), the app passes an empty icon, which leaves the bundle's `.icns` in place at no cost;
  - a development build decodes `icon-256.png` (the Dock's largest size), which costs nothing measurable.
- **`.icns`:** the build script makes it from `icon-1024.png` with `sips` and `iconutil` (both part of macOS).
- **Test:** checks that the committed PNGs match the generator, as for the skin.
- **Source quality:** the SVGs are automatic traces of small images, so the D's outer arc and straight edge wobble slightly at large sizes. That's invisible at icon sizes; redrawing those two curves by hand is a polish task, not a blocker.

### D7. Coming forward with a `diggr://` URL
- **Why:** after the extension's Add seller, the app asks to come to the front, but macOS refuses a background app's own focus request (the Dock icon bounces). It always honours opening a URL: LaunchServices activates the app that registered the scheme.
- **How:** `Info.plist` registers `diggr` (`CFBundleURLTypes`). The bridge's seller answer carries `"front": "diggr://show"` only when the player runs inside `Diggr.app` (the executable's path contains `.app/Contents/MacOS/`), and the extension then opens that URL. A development build never sends it, so it can't launch an installed copy next to itself. The app needs no Apple Event handler: activation is the whole effect. Chrome asks once whether to open Diggr, with "Always allow".

## Risks / Trade-offs

- **[yt-dlp's release layout changes]** (asset or checksum file renamed) → The download fails with a clear message, the user's own yt-dlp still works, and a test pins the names so a change is noticed in one place.
- **[Running a downloaded executable]** → Only after the user agrees, only from yt-dlp's official releases over HTTPS, only with a matching SHA-256, and invoked as today (no shell, `--ignore-config`, validated clip ids). The GPG signature check is an open question.
- **[No Apple Developer account, by choice]** → Every user does Open Anyway once, and some will give up there. The README and release notes give the steps next to the download. If a future macOS removes Open Anyway too, the signed path in the script is the way out.
- **[x86_64 build problems]** (a C dependency that doesn't cross-compile) → All C dependencies are built by `cc` with Apple's clang, which targets both architectures. The first build task proves it early.
- **[Launch time inside a bundle]** → The first open after installing includes Gatekeeper's check (seconds, once). The 300 ms target applies from the second open, and is measured with the bundle's own binary (`--startup-time`).
- **[Size]** → About 17 MB for the DMG (two architectures, 50 MB stripped together, LZMA-compressed). The spec allows 25 MB.

## Migration Plan

- **First release:** v0.1.0, unsigned, as a normal release with the Open Anyway steps; tried first on a few Macs (including an Intel one if possible).
- **Rollback:** a bad release is deleted or marked as a pre-release on GitHub. The previous DMG stays downloadable from its own release. Users' data folders don't change between versions.

## Open Questions

- **yt-dlp GPG signature check:** worth adding before public releases, or is SHA-256 over HTTPS enough?
- **Styled DMG window** (background art, icon positions) with `create-dmg`: later?
