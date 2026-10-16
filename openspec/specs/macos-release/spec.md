# macos-release Specification

## Purpose
Lets anyone with a Mac install the player from one downloaded file, and open it like any other app, without developer tools, and with one documented step the first time.
## Requirements
### Requirement: A universal app
The release SHALL be a macOS app bundle named after the app, with the app's logo as its icon, that runs natively on both Apple Silicon and Intel Macs with macOS 11 or later. Its binary SHALL contain both architectures. Everything the player needs (skin, scenes, rules) SHALL be inside the bundle, and it SHALL NOT need Rust, Homebrew or any other developer tool. Opened from the Finder, it SHALL show its first frame within 300 ms on an M-series Mac once macOS has checked it the first time.

#### Scenario: Both kinds of Mac
- **WHEN** the same app is opened on an Apple Silicon Mac and on an Intel Mac
- **THEN** it runs natively on each (not under Rosetta), and plays a local file

#### Scenario: A Mac without developer tools
- **WHEN** the app is opened on a Mac with no Xcode, Rust or Homebrew
- **THEN** the player opens with its skin and visuals, and plays local files

#### Scenario: Launch time
- **WHEN** the installed app is opened from the Finder for the second time
- **THEN** its first frame shows within 300 ms

### Requirement: The app's logo
The app's logo SHALL be the digging D: a D whose bowl is a record with a hand digging through it, in near-black on cream. It SHALL exist in two forms, kept as SVG sources in `assets/icon/`: the mark (`logo-mark.svg`, the D alone) and the lockup (`logo-lockup.svg`, the D over the "Diggr" wordmark). Every PNG of the logo SHALL be generated from the mark, and a test SHALL fail when a committed PNG differs from what the generator makes. The mark SHALL be shown, without the wordmark:
- as the app bundle's icon, in the Finder, the Dock and the disk image window, laid out on a cream rounded square with macOS's standard margin;
- as the window and Dock icon when the player is run from a development build;
- as the Chrome extension's icons (16, 32, 48 and 128 px), where the 16 px version SHALL still read as a D.

The lockup SHALL head the README.

Showing the logo SHALL NOT delay the first frame: launch SHALL stay within 300 ms.

#### Scenario: Installed app
- **WHEN** the app is installed from the disk image and opened
- **THEN** the Finder, the Dock and the app switcher show the digging-D mark

#### Scenario: Development build
- **WHEN** the player is started with `cargo run -p diggr`
- **THEN** its window and Dock icon show the digging-D mark, and the first frame still shows within 300 ms

#### Scenario: Browser extension
- **WHEN** the extension is loaded in Chrome
- **THEN** its toolbar button and its card in `chrome://extensions` show the digging-D mark

### Requirement: Drag to install
The app SHALL be delivered as one disk image named `Diggr-‹version›-unsigned.dmg` (`Diggr-‹version›.dmg` if it is ever signed with a Developer ID), of at most 25 MB. Opening it SHALL show a window with the app and a shortcut to Applications, so installing is dragging one onto the other. The installed app SHALL work after the disk image is ejected and deleted.

#### Scenario: Install
- **WHEN** a user opens the disk image and drags the app onto the Applications shortcut
- **THEN** the app is in Applications, and opens from there after the disk image is ejected

### Requirement: First launch without a paid Apple account
A published release SHALL NOT need an Apple Developer Program membership. The app SHALL be signed ad hoc (which Apple Silicon needs to run it) and is not notarized, so macOS refuses its first open. The README, next to the download link, and the notes of every unsigned release SHALL give the steps to open it: System Settings ▸ Privacy & Security ▸ Open Anyway, once. After that the app SHALL open like any other, also offline. The app SHALL ask for no permission at first launch (no microphone, no firewall prompt).

#### Scenario: First open of a downloaded release
- **WHEN** a user downloads the disk image with a browser, installs the app and opens it
- **THEN** macOS refuses it once; after Open Anyway in Privacy & Security and one confirmation, the player opens

#### Scenario: Later opens
- **WHEN** the app is opened again, with or without a network connection
- **THEN** it opens without any prompt

#### Scenario: No permission prompts
- **WHEN** the app is opened for the first time and plays a file
- **THEN** macOS asks for no permission

### Requirement: One-command build
A script SHALL build the disk image from a clean checkout in one command, into `dist/`. Without signing credentials, the normal case, it SHALL sign the app ad hoc, name the disk image as unsigned and print the Open Anyway steps. With a Developer ID and a notarization profile configured, it SHALL sign, notarize and staple instead. The release script SHALL publish an unsigned disk image only when told explicitly, so an image is never published as the wrong kind.

#### Scenario: Unsigned test build
- **WHEN** the build script runs on a Mac without a Developer ID certificate
- **THEN** it produces `dist/Diggr-‹version›-unsigned.dmg`, says it is unsigned, and prints the Open Anyway steps

#### Scenario: Publishing an unsigned build
- **WHEN** the release script is given an unsigned disk image without the explicit unsigned option
- **THEN** it stops without creating a release

### Requirement: Published on GitHub Releases
Each release SHALL be a git tag `v‹version›` matching the app's version, with a GitHub Release that has the disk image and its SHA-256 attached and short release notes. An unsigned release SHALL be a normal release, not a pre-release, so `releases/latest` leads to it, and its notes SHALL end with the Open Anyway steps. Disk images SHALL NOT be committed to the repository. The README SHALL link to the latest release for download.

#### Scenario: A new version
- **WHEN** version 0.2.0 is released
- **THEN** tag `v0.2.0` exists, its GitHub Release has `Diggr-0.2.0-unsigned.dmg` and its SHA-256, its notes end with the Open Anyway steps, and the README's download link leads to it

#### Scenario: Nothing binary in git
- **WHEN** a disk image is built
- **THEN** it is in `dist/`, which git ignores

