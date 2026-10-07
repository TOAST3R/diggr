## 1. App bundle and DMG

- [ ] 1.1 Install the `x86_64-apple-darwin` target and build `winamp-native` for both architectures with `MACOSX_DEPLOYMENT_TARGET=11.0` and stripped symbols; join them with `lipo`; verify with `lipo -info` (both architectures) and by running each slice (`arch -x86_64` under Rosetta for the Intel one) with `--startup-time`
- [ ] 1.2 Replace the app's logo with the record-crate logo (white line art of a crate of records on near-black; source: `openspec/changes/macos-release/logo-source.jpeg`): commit it cleaned up as `assets/icon/logo.png` (square, near-black background, the crate centred with even padding, JPEG artifacts removed) and `assets/icon/icon-1024.png` (macOS rounded-square layout with the standard margin); use it as the window and Dock icon when run from `cargo run` too (`ViewportBuilder::with_icon`, embedded with `include_bytes!`, decoded after the first frame so launch stays under 300 ms); replace the Chrome extension's `icons/icon{16,32,48,128}.png` with it, checking the 16 px version still reads as a crate; verify the Dock and window show it, and the extension's toolbar button and `chrome://extensions` card do
- [ ] 1.3 Write `packaging/macos/Info.plist.in` and the bundle step of `scripts/make-dmg.sh` (template filled from one name constant and the version in `apps/native/Cargo.toml`, `.icns` made with `sips` and `iconutil`, ad-hoc signature); verify with `plutil -lint`, and that `dist/‹App›.app` opens from the Finder, shows its icon and name in the Dock, and plays a local file
- [ ] 1.4 Add the DMG step (`hdiutil`, app plus an Applications symlink, named `‹App›-‹version›-unsigned.dmg` when unsigned, with the "testing only" warning and the Open Anyway steps); add `dist/` to `.gitignore`; verify the image opens with both icons, installs by dragging, is at most 25 MB, and the app runs after the image is ejected and deleted
- [ ] 1.5 Measure launch time of the installed bundle (`‹App›.app/Contents/MacOS/winamp-native --startup-time`, second open) and confirm it stays under 300 ms, and that no permission prompt appears when playing a file
- [ ] 1.6 Register a `winamp-rust://` URL scheme in `Info.plist` (`CFBundleURLTypes`) and handle it in the app, so the extension's Add seller (and later sends) can bring the player to the front, which macOS honours for a URL but not for a background app's own focus request (follow-up from `seller-crates`); verify that opening `winamp-rust://show` from Safari brings the bundle forward

## 2. Signing and notarization

- [ ] 2.1 Add the signed path to `make-dmg.sh` (`DEVELOPER_ID` and `NOTARY_PROFILE` from the environment; hardened runtime; notarize and staple the app, then the DMG; `spctl --assess` must pass or the script fails); verify with a dry run that, without the variables, the script takes the unsigned path and says why
- [ ] 2.2 With an Apple Developer account and a `notarytool` profile: build a signed release and confirm on another Mac (or a fresh user account) that a browser-downloaded DMG opens with only the standard confirmation, also offline. Blocked until the account exists

## 3. Releasing

- [ ] 3.1 Write `scripts/release.sh` (clean tree, version not yet tagged, DMG present and passing `spctl` unless `--unsigned`, SHA-256 file, tag `v‹version›`, push, `gh release create` with DMG, checksum and notes); verify that it refuses an unsigned DMG without `--unsigned` and a dirty tree, using a dry-run option that stops before tagging
- [ ] 3.2 README: "Download for Mac" link to `releases/latest` near the top, a "Build a release" section (both scripts, the two environment variables, unsigned builds and Open Anyway), and a note that disk images are never committed

## 4. yt-dlp managed by the app (`dig::tools`)

- [ ] 4.1 Implement the `ToolSource` trait (`get(url)`, `latest_tag()` from the `releases/latest` redirect) with a `ureq` implementation and a fake; parse `SHA2-256SUMS` for exactly `yt-dlp_macos`; verify with unit tests (parsing, missing entry, wrong hash length)
- [ ] 4.2 Implement install: download to `tools/.yt-dlp.part`, check SHA-256, `chmod 755`, rename to `tools/yt-dlp`; a mismatch deletes the file and never runs it; verify with tests against the fake (good file installed and runs `--version` via a stand-in script, bad hash leaves nothing behind, an interrupted download leaves the old copy intact)
- [ ] 4.3 Add the managed copy to the finder after the user's candidates (macOS only), and the daily update check while previews are fetched (`tools/checked`, compare the tag with `--version`, swap by rename); verify with tests: a user's yt-dlp wins and is never updated, no check within 24 h, a newer tag is installed, an older or equal one isn't
- [ ] 4.4 Wire it into the preview scheduler's worker (new commands and events: download requested, progress, installed with version, failed with reason), keeping every tool request off the UI thread and after the first frame; verify with a scheduler test where entries waiting with "needs yt-dlp" start downloading once the fake install finishes

## 5. yt-dlp in the player (`ui`)

- [ ] 5.1 On macOS, the first `NeedsYtDlp` opens "Previews need yt-dlp, a free downloader (37 MB). Download it?" with Download and Not now (remembered for the session); download progress in the status line; failures in the main window; verify with headless tests (accept → previews start, decline → asked no more, checksum failure → message and entries keep waiting)
- [ ] 5.2 OPT ▸ Discogs… shows the version and "(installed by you)" or "(downloaded by the app)", with Download or Update now; other platforms keep the install hint; verify with headless tests, and update the README (managed yt-dlp, its folder `tools/` in the cache table, the new test areas)

## 6. Checks

- [ ] 6.1 Build a full unsigned DMG from a clean clone with one command, install it on a Mac without Homebrew's yt-dlp (or with it hidden), send a Discogs label, accept the download, and confirm previews play
- [ ] 6.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check; confirm the README's test count matches
