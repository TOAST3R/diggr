## 1. Spike: can it be cross-built from the Mac?

- [x] 1.1 Install the maintainer tooling (`cargo install cargo-xwin --locked`, `brew install llvm`, `rustup target add x86_64-pc-windows-msvc`), accept Microsoft's SDK license on first use, and note the exact commands in `findings.md`
- [x] 1.2 Add `.cargo/config.toml` with `+crt-static` for `x86_64-pc-windows-msvc` only, then run `cargo xwin build --release --locked -p diggr --target x86_64-pc-windows-msvc`; if `ring` asks for `nasm`, install it and record it
- [x] 1.3 List the exe's imports with `llvm-objdump -p`; confirm no `VCRUNTIME*`, `MSVCP*` or `ucrtbase`, and record the DLL list and the exe's size
- [x] 1.4 Run `cargo xwin check --workspace --all-targets` and `cargo xwin clippy --workspace --all-targets -- -D warnings` for the Windows target; fix every compile or `cfg` error, and record each in `findings.md`
- [x] 1.5 Create `findings.md` in the change folder with one line per check named in the spec (launch, GUI, local playback, `--bench`, a preview with yt-dlp, a settings save, the bridge and firewall prompt, the exe's imports), filled in as far as the Mac can; the laptop's rows stay open until section 7

## 2. Port fixes

- [x] 2.1 Add one helper in `crates/dig` that builds a `Command` for a helper process and, on Windows only, sets `CREATE_NO_WINDOW` through `std::os::windows::process::CommandExt`; switch the three spawns in `preview/fetcher.rs` and the two in `tools.rs` to it; add a test that fails if a `Command::new` appears elsewhere in `crates/dig/src`
- [x] 2.2 Check the yt-dlp lookup for Windows: a path in OPT ▸ Discogs… with or without `.exe`, and `yt-dlp.exe` on the PATH; fix the lookup if it assumes Unix folders, and add unit tests with path strings in Windows form that run on every platform
- [x] 2.3 Check the code that is Unix-only by `cfg` (`tools.rs` permissions, `config.rs` and `bridge/pairing.rs` file modes) compiles and behaves on Windows (the Discogs token is protected by the user profile's folder permissions there); add a test that saving over an existing file with `write_atomic` succeeds
- [x] 2.4 Extend `icon-gen` to write `assets/icon/diggr.ico` (16, 32, 48 and 256 px) from `logo-mark.svg`, with a test that fails when the committed file differs, in the same style as the PNG test; run `cargo run -p ui --bin icon-gen`
- [x] 2.5 Add `apps/native/build.rs` that embeds the `.ico` (and the product name and version) with `embed-resource` using `llvm-rc`, for the Windows target only; confirm macOS, Linux and the wasm check are unchanged; if `llvm-rc` is missing, record it and ship the generic icon as a known gap
- [x] 2.6 Add `#![windows_subsystem = "windows"]` for release builds on Windows and, at the start of `main`, a parent-console attach so `--help`, `--bench`, `--tui`, `--startup-time` still print from a terminal; keep a development build's console; re-run 1.3 and 1.4

## 3. Packaging and the release script

- [x] 3.1 Add `packaging/windows/README.txt`: the SmartScreen steps (More info ▸ Run anyway), where the app keeps its files (`%APPDATA%\Diggr\`, `%LOCALAPPDATA%\Diggr\`), and how to install yt-dlp
- [x] 3.2 Write `scripts/make-windows.sh` after `make-linux.sh`: version from `apps/native/Cargo.toml`, `--dry-run`, the cross-build, the imports check that fails on `VCRUNTIME*`/`MSVCP*`/`ucrtbase` and names the DLL, the pack into `dist/Diggr-<version>-x86_64-windows.zip` (folder `Diggr-<version>/` with `Diggr.exe` and `README.txt`), and a size check at 25 MB
- [x] 3.3 Update `scripts/release.sh`: require the zip (stop before tagging and name `scripts/make-windows.sh` when it is missing), one `SHA256SUMS` over all three files, a Windows 10 section in the notes, and a `--prerelease` option that passes the flag to `gh release create`; update the script's header comment and `--dry-run` output
- [x] 3.4 Run `scripts/make-windows.sh --dry-run`, then a real run; check the zip's contents and size, and that the imports check fails when a `VCRUNTIME` import is simulated

## 4. Version, docs and changelog

- [x] 4.1 Bump the version to 0.1.4 in `apps/native/Cargo.toml` and update `Cargo.lock`
- [x] 4.2 Add the Windows entry to `CHANGELOG.md` (written as landing-page copy, following the `changelog` skill), rename `## Unreleased` to `## 0.1.4 — <date>` and start a fresh empty `## Unreleased`
- [x] 4.3 Update `README.md`: the Windows build requirements (cross-build from a Mac with `cargo-xwin`), the Windows notes for yt-dlp and folders, how to build the Windows zip, and the test count in its Tests section if it changed; do NOT add the Windows download link yet
- [x] 4.4 Update `AGENTS.md` where it lists scripts and release builds (`scripts/make-windows.sh`) so the next agent knows

## 5. Checks before anything is published

- [x] 5.1 Run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check` and `cargo check -p audio -p platform --target wasm32-unknown-unknown`; all clean
- [x] 5.2 Run the Windows-target `cargo xwin check` and clippy again, and `cargo run -p ui --bin skin-gen` and `icon-gen` to confirm the generated files are in sync
- [ ] 5.3 Build all three platforms for 0.1.4 from the committed tree (`scripts/make-dmg.sh`, `scripts/make-linux.sh`, `scripts/make-windows.sh`) and run `scripts/release.sh --unsigned --prerelease --dry-run`; read its summary (files, sizes, sums, notes)
- [x] 5.4 Commit on a branch and open a pull request for the user to merge (the release script requires a clean tree on the commit being released)

## 6. Publish the test release (ask the maintainer first)

- [ ] 6.1 Tell the maintainer exactly what will be published (tag `v0.1.4`, three files plus sums, as a pre-release) and wait for a go; do not push the tag before that
- [ ] 6.2 Run `scripts/release.sh --unsigned --prerelease --notes-file <notes>`; confirm on GitHub that it is marked pre-release, that `releases/latest` still leads to 0.1.3, and that the three files and the sums are attached
- [ ] 6.3 Give the maintainer the laptop checklist (section 7) and the pre-release's URL

## 7. Confirm on the Windows 10 laptop (the maintainer runs these)

- [ ] 7.1 Download the zip from the release page in the laptop's browser, extract it, open `Diggr.exe`; note what SmartScreen shows and whether More info ▸ Run anyway is offered
- [ ] 7.2 Check the player opens with its skin and the exe shows the digging-D icon in Explorer and the taskbar; note which graphics backend is in use if the window does not open (try `set WGPU_BACKEND=dx12`, then `vulkan`, in PowerShell before running)
- [ ] 7.3 Play a local file; from PowerShell run `.\Diggr.exe --startup-time` twice and `.\Diggr.exe --bench <file>`; write the launch, start, seek and underrun numbers into `findings.md`
- [ ] 7.4 Install yt-dlp (the `yt-dlp.exe` from its releases, on the PATH), paste a Discogs release address and play a preview; note that no console window flashes, how long the first preview takes, and that OPT ▸ Discogs… shows the yt-dlp version
- [ ] 7.5 Save a setting, close and reopen the app, and check it is kept; run once more with `DIGGR_CONFIG_DIR` and `DIGGR_CACHE_DIR` pointing to a folder with a space and an accented letter, playing a file from such a folder
- [ ] 7.6 Note whether Windows Defender Firewall or any other prompt appeared at any point
- [ ] 7.7 Report back to this session with the results; record every failure in `findings.md` as fixed in this change or a known gap with its reason

## 8. Fixes from the laptop, then promote

- [ ] 8.1 For each failure from section 7 that can be fixed from the Mac, fix it with a test where one is possible, and ship it as the next patch version (a new tag, never a reused one) through sections 4 to 7 again
- [ ] 8.2 When the maintainer confirms the Windows download works, promote the release (`gh release edit v<version> --prerelease=false --latest`) after asking, and check that `releases/latest` leads to it
- [ ] 8.3 Add the "Download for Windows" link and its first-open steps to `README.md` next to the Mac and Linux links, and tick the README/CHANGELOG items that apply
- [ ] 8.4 Confirm `findings.md` has no unrecorded failure, then archive the change (`/opsx:archive`)
