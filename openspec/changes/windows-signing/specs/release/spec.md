## ADDED Requirements

### Requirement: Windows: first open of a signed build
A Windows release SHALL be signed by SignPath Foundation unless the release is made with `--windows-unsigned`. A signed `Diggr.exe` SHALL open on Windows 11 with Smart App Control on. Windows SmartScreen may still show "Windows protected your PC" (publisher SignPath Foundation) until the certificate has a download history, so the release notes, the zip's `README.txt` and the README SHALL keep the steps (More info, then Run anyway) and say they may be needed once. For an unsigned release, the text SHALL say it is unsigned and that Smart App Control blocks it. The app SHALL ask for no permission at first launch: no Windows Defender Firewall prompt (the browser bridge listens only on 127.0.0.1), no microphone prompt.

#### Scenario: Smart App Control on
- **WHEN** a signed release's zip is downloaded in a browser on Windows 11 with Smart App Control on, extracted, and `Diggr.exe` is opened
- **THEN** the player opens, after at most one SmartScreen "Run anyway"

#### Scenario: Signature shown
- **WHEN** a user opens Properties ▸ Digital Signatures on the extracted `Diggr.exe`
- **THEN** the signer is SignPath Foundation and the signature is valid

#### Scenario: Unsigned fallback
- **WHEN** a release is made with `--windows-unsigned`
- **THEN** its notes say the Windows build is unsigned and blocked by Smart App Control, and give the Run anyway steps

## MODIFIED Requirements

### Requirement: One-command build
Each platform SHALL have a script that builds its release file from a clean checkout in one command, into `dist/`: `scripts/make-dmg.sh` for macOS, `scripts/make-linux.sh` for Linux and `scripts/make-windows.sh` for Windows. All SHALL run on the release Mac; the Linux one SHALL need only Docker besides what macOS builds need, and SHALL NOT need HTTPS from inside its container; the Windows one SHALL need only `cargo-xwin` and Homebrew's `llvm` (and `nasm`, if the build asks for it) besides what macOS builds need, and with `--signed` only `gh` and `osslsigncode`. The Windows script SHALL cross-build the exe for `x86_64-pc-windows-msvc`, check it with `scripts/check-windows-exe.sh` (failing when the exe imports a DLL that Windows does not provide: `VCRUNTIME*`, `MSVCP*`, `ucrtbase`), pack the zip with `scripts/pack-windows-zip.sh` and check its size and contents; with `--signed` it SHALL instead fetch the signed zip built by the CI workflow for `HEAD`. It SHALL accept `--dry-run`, which only says what it would build. Without signing credentials, the normal case, the macOS script SHALL sign the app ad hoc, name the disk image as unsigned and print the Open Anyway steps. With a Developer ID and a notarization profile configured, it SHALL sign, notarize and staple instead. The release script SHALL publish an unsigned disk image only when told explicitly, so an image is never published as the wrong kind.

#### Scenario: Unsigned test build
- **WHEN** the macOS build script runs on a Mac without a Developer ID certificate
- **THEN** it produces `dist/Diggr-‹version›-unsigned.dmg`, says it is unsigned, and prints the Open Anyway steps

#### Scenario: Linux build on the Mac
- **WHEN** `scripts/make-linux.sh` runs on the release Mac with Docker running
- **THEN** it produces `dist/Diggr-‹version›-x86_64-linux.tar.gz` and reports the binary's glibc check as passed

#### Scenario: Windows build on the Mac
- **WHEN** `scripts/make-windows.sh` runs on the release Mac with `cargo-xwin` installed
- **THEN** it produces `dist/Diggr-‹version›-x86_64-windows.zip` and reports the imports check as passed

#### Scenario: Signed Windows build
- **WHEN** `scripts/make-windows.sh --signed` runs on a pushed `HEAD`
- **THEN** it produces `dist/Diggr-‹version›-x86_64-windows.zip` from the CI run for `HEAD`, with `Diggr.exe` signed by SignPath Foundation

#### Scenario: A runtime DLL leaks in
- **WHEN** the exe imports `VCRUNTIME140.dll`
- **THEN** the Windows script fails and names the DLL, and no zip is written

#### Scenario: Publishing an unsigned build
- **WHEN** the release script is given an unsigned disk image without the explicit unsigned option
- **THEN** it stops without creating a release

### Requirement: Published on GitHub Releases
Each release SHALL be a git tag `v‹version›` matching the app's version, with a GitHub Release that has the macOS disk image, the Linux tarball, the Windows zip and one `Diggr-‹version›-SHA256SUMS` file (in `sha256sum -c` format, covering all three) attached, and short release notes. The release script SHALL stop before tagging when any platform's file for that version is missing, and SHALL say which script builds it. It SHALL extract `Diggr.exe` from the Windows zip and stop before tagging when `osslsigncode verify` fails on it or its signer isn't SignPath Foundation, unless it is run with `--windows-unsigned`. An unsigned release SHALL be a normal release, not a pre-release, so `releases/latest` leads to it, except a release made with `--prerelease`, which SHALL be marked as a pre-release on GitHub so that `releases/latest` keeps leading to the previous release until it is promoted (`gh release edit v‹version› --prerelease=false --latest`). A pre-release SHALL be used for a version whose Windows (or other platform's) file has not yet been confirmed on a real machine, and for the first signed Windows release. Its notes SHALL end with the macOS first-open steps (when unsigned), the Linux install steps and the Windows first-open steps matching whether the zip is signed, each under its platform's name. Release files SHALL NOT be committed to the repository. The README SHALL link to the latest release for download, once for Mac, once for Linux and, after the first Windows release has been promoted, once for Windows.

#### Scenario: A new version
- **WHEN** version 0.2.0 is released
- **THEN** tag `v0.2.0` exists, and its GitHub Release has `Diggr-0.2.0-unsigned.dmg`, `Diggr-0.2.0-x86_64-linux.tar.gz`, `Diggr-0.2.0-x86_64-windows.zip` and `Diggr-0.2.0-SHA256SUMS`; `sha256sum -c` passes for all three downloads, its notes end with the macOS, Linux and Windows steps, and the README's download links lead to it

#### Scenario: A platform missing
- **WHEN** the release script runs and `dist/` has the disk image and the tarball but no Windows zip for this version
- **THEN** it stops before tagging and says to run `scripts/make-windows.sh`

#### Scenario: An unsigned Windows zip
- **WHEN** the release script runs without `--windows-unsigned` and the zip's `Diggr.exe` is unsigned
- **THEN** it stops before tagging and says to run `scripts/make-windows.sh --signed`

#### Scenario: A test release
- **WHEN** the release script runs with `--prerelease`
- **THEN** the GitHub Release is marked as a pre-release, it can be downloaded without a GitHub login, and `releases/latest` still leads to the previous release

#### Scenario: Promotion
- **WHEN** the maintainer has confirmed the Windows download works on a Windows machine and promotes the release
- **THEN** it is no longer a pre-release, `releases/latest` leads to it, and the README gains its Windows download link

#### Scenario: Nothing binary in git
- **WHEN** a disk image, a tarball or a zip is built
- **THEN** it is in `dist/`, which git ignores

## REMOVED Requirements

### Requirement: Windows: first open without a certificate
**Reason**: Replaced by "Windows: first open of a signed build": releases are signed by SignPath Foundation, and the unsigned case is now the explicit `--windows-unsigned` fallback described there.
**Migration**: None for users. Unsigned releases need `scripts/release.sh --windows-unsigned`. This change is archived after `windows-port`, which adds the removed requirement.
