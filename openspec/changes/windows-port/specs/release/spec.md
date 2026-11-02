## ADDED Requirements

### Requirement: Windows: one zip for x86_64
The Windows release SHALL be one archive named `Diggr-‹version›-x86_64-windows.zip`, of at most 25 MB. It SHALL hold one folder `Diggr-‹version›/` with `Diggr.exe` and a `README.txt` that gives the first-open steps, where the app keeps its files and how to install yt-dlp. The exe SHALL run on Windows 10 x86_64 (Windows 11 is expected to work and is not tested), SHALL need nothing installed for it (no Rust, no Visual C++ Redistributable) and SHALL NOT register the `diggr://` scheme. Extracted and run without installing, the player SHALL open with its skin and play local files. The folder it is extracted to SHALL be deletable afterwards without affecting the user's crates and settings, which live in the user's profile.

#### Scenario: A Windows 10 without developer tools
- **WHEN** the zip is downloaded in a browser on a clean Windows 10, extracted, and `Diggr.exe` is opened
- **THEN** after the first-open steps the player opens with its skin and plays a local file

#### Scenario: Contents and size
- **WHEN** the build script has packed the zip
- **THEN** it holds `Diggr-‹version›/Diggr.exe` and `Diggr-‹version›/README.txt`, and is at most 25 MB

### Requirement: Windows: first open without a certificate
A published Windows release SHALL NOT need a code-signing certificate. The exe is unsigned, so Windows SmartScreen shows "Windows protected your PC" on the first open of a downloaded copy. The release notes, the zip's `README.txt` and (once the release is promoted) the README next to the download link SHALL give the steps: More info, then Run anyway, once. The app SHALL ask for no permission at first launch: no Windows Defender Firewall prompt (the browser bridge listens only on 127.0.0.1), no microphone prompt. A prompt that does appear on the Windows 10 test machine SHALL be recorded as a finding.

#### Scenario: First open of a downloaded release
- **WHEN** a user downloads the zip with a browser, extracts it and opens `Diggr.exe`
- **THEN** SmartScreen shows once; after More info ▸ Run anyway the player opens

#### Scenario: No permission prompts
- **WHEN** the app is opened for the first time and plays a file
- **THEN** Windows asks for no permission, or the findings record the prompt that appeared

## MODIFIED Requirements

### Requirement: One-command build
Each platform SHALL have a script that builds its release file from a clean checkout in one command, into `dist/`: `scripts/make-dmg.sh` for macOS, `scripts/make-linux.sh` for Linux and `scripts/make-windows.sh` for Windows. All SHALL run on the release Mac; the Linux one SHALL need only Docker besides what macOS builds need, and SHALL NOT need HTTPS from inside its container; the Windows one SHALL need only `cargo-xwin` and Homebrew's `llvm` (and `nasm`, if the build asks for it) besides what macOS builds need. The Windows script SHALL cross-build the exe for `x86_64-pc-windows-msvc`, fail when the exe imports a DLL that Windows does not provide (`VCRUNTIME*`, `MSVCP*`, `ucrtbase`), pack the zip and check its size and contents; it SHALL accept `--dry-run`, which only says what it would build. Without signing credentials, the normal case, the macOS script SHALL sign the app ad hoc, name the disk image as unsigned and print the Open Anyway steps. With a Developer ID and a notarization profile configured, it SHALL sign, notarize and staple instead. The release script SHALL publish an unsigned disk image only when told explicitly, so an image is never published as the wrong kind.

#### Scenario: Unsigned test build
- **WHEN** the macOS build script runs on a Mac without a Developer ID certificate
- **THEN** it produces `dist/Diggr-‹version›-unsigned.dmg`, says it is unsigned, and prints the Open Anyway steps

#### Scenario: Linux build on the Mac
- **WHEN** `scripts/make-linux.sh` runs on the release Mac with Docker running
- **THEN** it produces `dist/Diggr-‹version›-x86_64-linux.tar.gz` and reports the binary's glibc check as passed

#### Scenario: Windows build on the Mac
- **WHEN** `scripts/make-windows.sh` runs on the release Mac with `cargo-xwin` installed
- **THEN** it produces `dist/Diggr-‹version›-x86_64-windows.zip` and reports the imports check as passed

#### Scenario: A runtime DLL leaks in
- **WHEN** the exe imports `VCRUNTIME140.dll`
- **THEN** the Windows script fails and names the DLL, and no zip is written

#### Scenario: Publishing an unsigned build
- **WHEN** the release script is given an unsigned disk image without the explicit unsigned option
- **THEN** it stops without creating a release

### Requirement: Published on GitHub Releases
Each release SHALL be a git tag `v‹version›` matching the app's version, with a GitHub Release that has the macOS disk image, the Linux tarball, the Windows zip and one `Diggr-‹version›-SHA256SUMS` file (in `sha256sum -c` format, covering all three) attached, and short release notes. The release script SHALL stop before tagging when any platform's file for that version is missing, and SHALL say which script builds it. An unsigned release SHALL be a normal release, not a pre-release, so `releases/latest` leads to it, except a release made with `--prerelease`, which SHALL be marked as a pre-release on GitHub so that `releases/latest` keeps leading to the previous release until it is promoted (`gh release edit v‹version› --prerelease=false --latest`). A pre-release SHALL be used for a version whose Windows (or other platform's) file has not yet been confirmed on a real machine. Its notes SHALL end with the macOS first-open steps (when unsigned), the Linux install steps and the Windows first-open steps, each under its platform's name. Release files SHALL NOT be committed to the repository. The README SHALL link to the latest release for download, once for Mac, once for Linux and, after the first Windows release has been promoted, once for Windows.

#### Scenario: A new version
- **WHEN** version 0.2.0 is released
- **THEN** tag `v0.2.0` exists, and its GitHub Release has `Diggr-0.2.0-unsigned.dmg`, `Diggr-0.2.0-x86_64-linux.tar.gz`, `Diggr-0.2.0-x86_64-windows.zip` and `Diggr-0.2.0-SHA256SUMS`; `sha256sum -c` passes for all three downloads, its notes end with the macOS, Linux and Windows steps, and the README's download links lead to it

#### Scenario: A platform missing
- **WHEN** the release script runs and `dist/` has the disk image and the tarball but no Windows zip for this version
- **THEN** it stops before tagging and says to run `scripts/make-windows.sh`

#### Scenario: A test release
- **WHEN** the release script runs with `--prerelease`
- **THEN** the GitHub Release is marked as a pre-release, it can be downloaded without a GitHub login, and `releases/latest` still leads to the previous release

#### Scenario: Promotion
- **WHEN** the maintainer has confirmed the Windows download works on the Windows 10 laptop and promotes the release
- **THEN** it is no longer a pre-release, `releases/latest` leads to it, and the README gains its Windows download link

#### Scenario: Nothing binary in git
- **WHEN** a disk image, a tarball or a zip is built
- **THEN** it is in `dist/`, which git ignores
