# app-identity Specification

## Purpose
TBD - created by archiving change rebrand-diggr. Update Purpose after archive.
## Requirements
### Requirement: Product name
The app SHALL be called Diggr in everything a user sees: the window title, the name it reports to the browser extension (`dig::APP_NAME`), the extension's name and toolbar title, the help panel and the docs. The skin's title bar, whose font is uppercase, SHALL show it as DIGGR.

#### Scenario: Window and extension
- **WHEN** the app starts and the extension says hello to it
- **THEN** the window title is "Diggr", and the extension's buttons read "Play in Diggr" and "Enqueue in Diggr"

#### Scenario: Before the first hello
- **WHEN** the extension is installed and the player has never answered
- **THEN** its buttons and toolbar title already say Diggr

### Requirement: Folders
The app SHALL keep its config and cache in a folder named `Diggr` on macOS and Windows and `diggr` on other systems, inside the platform's config and cache directories, named in one place. `DIGGR_CONFIG_DIR` and `DIGGR_CACHE_DIR` SHALL override them. The app SHALL NOT read or move folders of any earlier name.

#### Scenario: macOS
- **WHEN** the app saves its settings and a score on macOS without overrides
- **THEN** they are in `~/Library/Application Support/Diggr/settings.ron` and under `~/Library/Caches/Diggr/scores/`

#### Scenario: Override
- **WHEN** the app runs with `DIGGR_CONFIG_DIR=/tmp/d`
- **THEN** settings and crates are read from and written to `/tmp/d`

### Requirement: Binary and environment
The desktop package and binary SHALL be `diggr`, with the same command-line modes as before (`--tui`, `--bench`, `--click-test`, `--startup-time`, `--render-show`). Every environment variable the app or its tests read SHALL start with `DIGGR_`.

#### Scenario: Usage
- **WHEN** the user runs `diggr --help`
- **THEN** the usage line starts with "usage: diggr" and lists every mode

#### Scenario: Env vars
- **WHEN** the repo's code is searched for `std::env::var` calls
- **THEN** every variable name read starts with `DIGGR_`

### Requirement: Outward identity
Requests to Discogs SHALL send the User-Agent `Diggr/‹version› +https://github.com/TOAST3R/diggr`.

#### Scenario: User-Agent
- **WHEN** the app reads a Discogs release
- **THEN** the request's User-Agent starts with "Diggr/0.1.0 +https://github.com/TOAST3R/diggr"

### Requirement: No other product's name
No tracked file in the repository SHALL contain the name of the player that inspired the app, in any letter case. A test SHALL check every tracked text file, and SHALL skip when git is unavailable.

#### Scenario: Clean repo
- **WHEN** `cargo test --workspace` runs in a git checkout
- **THEN** the clean-repo test passes, and it fails naming the file and line if a tracked file contains that name

