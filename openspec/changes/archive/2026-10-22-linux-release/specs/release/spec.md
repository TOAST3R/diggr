## ADDED Requirements

### Requirement: macOS: a universal app
The macOS release SHALL be a macOS app bundle named after the app, with the app's logo as its icon, that runs natively on both Apple Silicon and Intel Macs with macOS 11 or later. Its binary SHALL contain both architectures. Everything the player needs (its skin) SHALL be inside the bundle, and it SHALL NOT need Rust, Homebrew or any other developer tool. Opened from the Finder, it SHALL show its first frame within 300 ms on an M-series Mac once macOS has checked it the first time.

#### Scenario: Both kinds of Mac
- **WHEN** the same app is opened on an Apple Silicon Mac and on an Intel Mac
- **THEN** it runs natively on each (not under Rosetta), and plays a local file

#### Scenario: A Mac without developer tools
- **WHEN** the app is opened on a Mac with no Xcode, Rust or Homebrew
- **THEN** the player opens with its skin, and plays local files

#### Scenario: Launch time
- **WHEN** the installed app is opened from the Finder for the second time
- **THEN** its first frame shows within 300 ms

### Requirement: The app's logo
The app's logo SHALL be the digging D: a D whose bowl is a record with a hand digging through it, in near-black on cream. It SHALL exist in two forms, kept as SVG sources in `assets/icon/`: the mark (`logo-mark.svg`, the D alone) and the lockup (`logo-lockup.svg`, the D over the "Diggr" wordmark). Every PNG of the logo SHALL be generated from the mark, and a test SHALL fail when a committed PNG differs from what the generator makes. The mark SHALL be shown, without the wordmark:
- as the app bundle's icon, in the Finder, the Dock and the disk image window, laid out on a cream rounded square with macOS's standard margin;
- as the icon of the Linux menu entry that `install.sh` adds, from the 256 px PNG;
- as the window and Dock icon when the player is run from a development build;
- as the Chrome extension's icons (16, 32, 48 and 128 px), where the 16 px version SHALL still read as a D.

The lockup SHALL head the README.

Showing the logo SHALL NOT delay the first frame: launch SHALL stay within 300 ms.

#### Scenario: Installed app
- **WHEN** the app is installed from the disk image and opened
- **THEN** the Finder, the Dock and the app switcher show the digging-D mark

#### Scenario: Linux menu entry
- **WHEN** `install.sh` has run on a GNOME or KDE desktop
- **THEN** the applications menu lists Diggr with the digging-D mark

#### Scenario: Development build
- **WHEN** the player is started with `cargo run -p diggr`
- **THEN** its window and Dock icon show the digging-D mark, and the first frame still shows within 300 ms

#### Scenario: Browser extension
- **WHEN** the extension is loaded in Chrome
- **THEN** its toolbar button and its card in `chrome://extensions` show the digging-D mark

### Requirement: macOS: drag to install
The app SHALL be delivered as one disk image named `Diggr-‹version›-unsigned.dmg` (`Diggr-‹version›.dmg` if it is ever signed with a Developer ID), of at most 25 MB. Opening it SHALL show a window with the app and a shortcut to Applications, so installing is dragging one onto the other. The installed app SHALL work after the disk image is ejected and deleted.

#### Scenario: Install
- **WHEN** a user opens the disk image and drags the app onto the Applications shortcut
- **THEN** the app is in Applications, and opens from there after the disk image is ejected

### Requirement: macOS: first launch without a paid Apple account
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

### Requirement: Linux: one tarball for x86_64
The Linux release SHALL be one archive named `Diggr-‹version›-x86_64-linux.tar.gz`, of at most 25 MB. It SHALL hold one folder `Diggr-‹version›/` with the stripped `diggr` binary, a `diggr.desktop` menu entry, the 256 px icon and `install.sh`. The binary SHALL run on any x86_64 distribution with glibc 2.31 or later (Debian 11, Ubuntu 20.04, Fedora 32 and later), under X11 or Wayland. It SHALL need only glibc, ALSA (`libasound.so.2`) and a Vulkan or OpenGL driver, and SHALL NOT need Rust or any package installed for it. Run from the extracted folder without installing, the player SHALL open with its skin and play local files. The release SHALL NOT register the `diggr://` scheme on Linux.

#### Scenario: A distribution without developer tools
- **WHEN** the archive is extracted on a fresh Ubuntu 22.04 desktop and `./diggr` is run
- **THEN** the player opens with its skin and plays a local file

#### Scenario: Oldest supported glibc
- **WHEN** the build script checks the binary
- **THEN** it requires no `GLIBC_` symbol version above 2.31, and `diggr --help` succeeds in a bare Debian 11 container that has only `libasound2` installed

### Requirement: Linux: per-user install
`install.sh` SHALL install the player for the current user only, without root: the binary in `~/.local/bin/`, the icon in the user's `hicolor` icon theme, and the menu entry in the user's `applications` folder (under `XDG_DATA_HOME` when set), with the binary's absolute path in its `Exec` line. Running it again SHALL replace the installed copy (an upgrade). It SHALL print the paths it wrote, which are all an uninstall has to delete, and the README SHALL list them.

#### Scenario: Install
- **WHEN** a user runs `./install.sh` from the extracted folder
- **THEN** Diggr appears in the applications menu and opens from there, also after the extracted folder is deleted

#### Scenario: Upgrade
- **WHEN** `install.sh` of a newer version runs over an installed one
- **THEN** the menu entry opens the newer version, and the user's crates and settings are kept

### Requirement: One-command build
Each platform SHALL have a script that builds its release file from a clean checkout in one command, into `dist/`: `scripts/make-dmg.sh` for macOS and `scripts/make-linux.sh` for Linux. Both SHALL run on the release Mac; the Linux one SHALL need only Docker besides what macOS builds need, and SHALL NOT need HTTPS from inside its container. Without signing credentials, the normal case, the macOS script SHALL sign the app ad hoc, name the disk image as unsigned and print the Open Anyway steps. With a Developer ID and a notarization profile configured, it SHALL sign, notarize and staple instead. The release script SHALL publish an unsigned disk image only when told explicitly, so an image is never published as the wrong kind.

#### Scenario: Unsigned test build
- **WHEN** the macOS build script runs on a Mac without a Developer ID certificate
- **THEN** it produces `dist/Diggr-‹version›-unsigned.dmg`, says it is unsigned, and prints the Open Anyway steps

#### Scenario: Linux build on the Mac
- **WHEN** `scripts/make-linux.sh` runs on the release Mac with Docker running
- **THEN** it produces `dist/Diggr-‹version›-x86_64-linux.tar.gz` and reports the binary's glibc check as passed

#### Scenario: Publishing an unsigned build
- **WHEN** the release script is given an unsigned disk image without the explicit unsigned option
- **THEN** it stops without creating a release

### Requirement: Published on GitHub Releases
Each release SHALL be a git tag `v‹version›` matching the app's version, with a GitHub Release that has the macOS disk image, the Linux tarball and one `Diggr-‹version›-SHA256SUMS` file (in `sha256sum -c` format, covering both) attached, and short release notes. The release script SHALL stop before tagging when either platform's file for that version is missing, and SHALL say which script builds it. An unsigned release SHALL be a normal release, not a pre-release, so `releases/latest` leads to it. Its notes SHALL end with the macOS first-open steps (when unsigned) and the Linux install steps, each under its platform's name. Release files SHALL NOT be committed to the repository. The README SHALL link to the latest release for download, once for Mac and once for Linux.

#### Scenario: A new version
- **WHEN** version 0.2.0 is released
- **THEN** tag `v0.2.0` exists, and its GitHub Release has `Diggr-0.2.0-unsigned.dmg`, `Diggr-0.2.0-x86_64-linux.tar.gz` and `Diggr-0.2.0-SHA256SUMS`; `sha256sum -c` passes for both downloads, its notes end with the macOS and Linux steps, and the README's download links lead to it

#### Scenario: A platform missing
- **WHEN** the release script runs and `dist/` has the disk image but no Linux tarball for this version
- **THEN** it stops before tagging and says to run `scripts/make-linux.sh`

#### Scenario: Nothing binary in git
- **WHEN** a disk image or a tarball is built
- **THEN** it is in `dist/`, which git ignores
