## MODIFIED Requirements

### Requirement: A universal app
The release SHALL be a macOS app bundle named after the app, with the app's logo as its icon, that runs natively on both Apple Silicon and Intel Macs with macOS 11 or later. Its binary SHALL contain both architectures. Everything the player needs (its skin) SHALL be inside the bundle, and it SHALL NOT need Rust, Homebrew or any other developer tool. Opened from the Finder, it SHALL show its first frame within 300 ms on an M-series Mac once macOS has checked it the first time.

#### Scenario: Both kinds of Mac
- **WHEN** the same app is opened on an Apple Silicon Mac and on an Intel Mac
- **THEN** it runs natively on each (not under Rosetta), and plays a local file

#### Scenario: A Mac without developer tools
- **WHEN** the app is opened on a Mac with no Xcode, Rust or Homebrew
- **THEN** the player opens with its skin, and plays local files

#### Scenario: Launch time
- **WHEN** the installed app is opened from the Finder for the second time
- **THEN** its first frame shows within 300 ms
