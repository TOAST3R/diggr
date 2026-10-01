## Purpose

Lets anyone with a Mac install the player from one downloaded file, and open it like any other app, without developer tools or security workarounds.

## ADDED Requirements

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
The app's logo SHALL be the record crate: white line art of a wooden crate holding records, on a near-black background (the source is `openspec/changes/macos-release/logo-source.jpeg`). It SHALL be kept in the repository as `assets/icon/logo.png` (square, the crate centred with even padding) and `assets/icon/icon-1024.png` (laid out for macOS: a rounded square with the standard margin), and nowhere else as a separate copy. The logo SHALL be shown:
- as the app bundle's icon, in the Finder, the Dock and the disk image window;
- as the window and Dock icon when the player is run from a development build;
- as the Chrome extension's icons (16, 32, 48 and 128 px), where the 16 px version SHALL still read as a crate.

Showing the logo SHALL NOT delay the first frame: launch SHALL stay within 300 ms.

#### Scenario: Installed app
- **WHEN** the app is installed from the disk image and opened
- **THEN** the Finder, the Dock and the app switcher show the record-crate logo

#### Scenario: Development build
- **WHEN** the player is started with `cargo run -p winamp-native`
- **THEN** its window and Dock icon show the record-crate logo, and the first frame still shows within 300 ms

#### Scenario: Browser extension
- **WHEN** the extension is loaded in Chrome
- **THEN** its toolbar button and its card in `chrome://extensions` show the record-crate logo

### Requirement: Drag to install
The app SHALL be delivered as one disk image named `‹App›-‹version›.dmg`, of at most 25 MB. Opening it SHALL show a window with the app and a shortcut to Applications, so installing is dragging one onto the other. The installed app SHALL work after the disk image is ejected and deleted.

#### Scenario: Install
- **WHEN** a user opens the disk image and drags the app onto the Applications shortcut
- **THEN** the app is in Applications, and opens from there after the disk image is ejected

### Requirement: Trusted first launch
A published release SHALL be signed with a Developer ID certificate, use the hardened runtime, and be notarized by Apple with the ticket stapled to both the app and the disk image. Opening it for the first time SHALL show only macOS's standard confirmation for apps downloaded from the internet, and SHALL work without a network connection. The app SHALL ask for no permission at first launch (no microphone, no firewall prompt).

#### Scenario: First open of a downloaded release
- **WHEN** a user downloads the disk image with a browser, installs the app and opens it
- **THEN** macOS asks only whether to open an app downloaded from the internet, and the player opens after one click

#### Scenario: Offline first open
- **WHEN** the downloaded app is opened for the first time with no network connection
- **THEN** it opens the same way, because its notarization ticket is stapled

#### Scenario: No permission prompts
- **WHEN** the app is opened for the first time and plays a file
- **THEN** macOS asks for no permission

### Requirement: One-command build
A script SHALL build the disk image from a clean checkout in one command, into `dist/`. With signing credentials configured, it SHALL sign, notarize and staple. Without them, it SHALL still build the disk image, name it as unsigned and say so clearly. The release script SHALL refuse to publish an unsigned disk image unless explicitly told to.

#### Scenario: Unsigned test build
- **WHEN** the build script runs on a Mac without a Developer ID certificate
- **THEN** it produces `dist/‹App›-‹version›-unsigned.dmg` and warns that the build is for testing only

#### Scenario: Publishing an unsigned build
- **WHEN** the release script is given an unsigned disk image without the explicit unsigned option
- **THEN** it stops without creating a release

### Requirement: Published on GitHub Releases
Each release SHALL be a git tag `v‹version›` matching the app's version, with a GitHub Release that has the disk image and its SHA-256 attached and short release notes. Disk images SHALL NOT be committed to the repository. The README SHALL link to the latest release for download.

#### Scenario: A new version
- **WHEN** version 0.2.0 is released
- **THEN** tag `v0.2.0` exists, its GitHub Release has `‹App›-0.2.0.dmg` and its SHA-256, and the README's download link leads to it

#### Scenario: Nothing binary in git
- **WHEN** a disk image is built
- **THEN** it is in `dist/`, which git ignores
