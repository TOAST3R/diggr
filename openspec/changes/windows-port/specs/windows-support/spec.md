## ADDED Requirements

### Requirement: Windows 10 x86_64 build from macOS
The app SHALL be buildable for `x86_64-pc-windows-msvc` on a Mac, with `cargo-xwin` and Homebrew's `llvm` and nothing that runs only on Windows, producing one `diggr.exe`. The C runtime SHALL be linked statically, so the exe SHALL import only DLLs that Windows itself provides and SHALL NOT need the Visual C++ Redistributable. Everything the player needs (its skin and icon) SHALL be inside the exe. The macOS, Linux and `wasm32-unknown-unknown` builds SHALL be unchanged by it, and `audio` and `platform` SHALL still compile for `wasm32-unknown-unknown`. The tested system SHALL be Windows 10; Windows 11 SHALL be expected to work and SHALL be described as untested.

#### Scenario: Cross-build
- **WHEN** the release build for the Windows target is run on the Mac
- **THEN** it produces `diggr.exe`, and `cargo xwin check --workspace --all-targets` and clippy with `-D warnings` pass for that target

#### Scenario: Only system DLLs
- **WHEN** the exe's imports are listed with `llvm-objdump -p`
- **THEN** none is `VCRUNTIME*`, `MSVCP*` or `ucrtbase`, and each is a DLL that a clean Windows 10 has

#### Scenario: A Windows 10 without developer tools
- **WHEN** the release zip is downloaded from GitHub in a browser on a Windows 10 machine with no Rust, Visual Studio or Visual C++ Redistributable, extracted, and `Diggr.exe` is run
- **THEN** it opens its window with the skin and plays a local file

#### Scenario: Other targets unchanged
- **WHEN** the macOS build, the Linux build and `cargo check -p audio -p platform --target wasm32-unknown-unknown` run after the change
- **THEN** they succeed, and `cargo test --workspace` passes on the Mac

### Requirement: Playback targets on Windows
On Windows 10 the player SHALL meet the same targets as elsewhere: first non-silent sample within 30 ms of play on a warm app, seek audible within 50 ms, 0 underruns in a `--bench` run, and the first frame within 300 ms of launch. They SHALL be measured with `diggr --bench` and `--startup-time` on the Windows 10 laptop, and the numbers SHALL be recorded in the change's findings. A target the Windows build misses SHALL NOT be relaxed without a recorded decision that names the new number.

#### Scenario: Bench on the laptop
- **WHEN** `diggr.exe --bench` runs on the Windows 10 laptop on a local file
- **THEN** it reports start under 30 ms, seek under 50 ms and 0 underruns, or the findings record each number it missed

#### Scenario: Launch time
- **WHEN** `diggr.exe --startup-time` runs the second time after a copy to the laptop
- **THEN** it reports a first frame within 300 ms, or the findings record the number and the graphics backend wgpu chose

### Requirement: Console and command-line modes
The GUI SHALL open without a console window in a release build. The command-line modes (`--help`, `--tui`, `--bench`, `--click-test`, `--startup-time`) SHALL still print to the terminal they were started from, with the same usage line as on other systems. A development build SHALL keep its console.

#### Scenario: Double-click
- **WHEN** the exe is opened from Explorer
- **THEN** the player's window opens and no console window appears

#### Scenario: Usage from a terminal
- **WHEN** `diggr.exe --help` is run in Command Prompt or PowerShell
- **THEN** the usage line starting "usage: diggr" is printed there

### Requirement: Quiet helper processes
Every process the app starts on Windows (yt-dlp for previews and for its version check) SHALL be started without a console window, and its output and exit status SHALL be read as on other systems. All such starts SHALL go through one function, so none can omit that.

#### Scenario: Previews
- **WHEN** a record is sent and its previews download with yt-dlp on Windows
- **THEN** no console window opens or flashes, and the previews play

#### Scenario: One place
- **WHEN** the code is searched for process starts in `crates/dig`
- **THEN** each goes through the same helper

### Requirement: yt-dlp is found on Windows
On Windows the app SHALL find a yt-dlp that the user installed: at the path set in OPT ▸ Discogs… (with or without `.exe`), or on the PATH as `yt-dlp.exe`. While none is found, entries SHALL say "needs yt-dlp" and the main window SHALL say how to install it, and the app SHALL look again within 30 s of one being installed, as the preview spec requires. It SHALL NOT offer to download one on Windows in this release.

#### Scenario: On the PATH
- **WHEN** `yt-dlp.exe` is in a PATH folder and a record is sent
- **THEN** its previews start downloading and OPT ▸ Discogs… shows its version as "installed by you"

#### Scenario: Not installed
- **WHEN** a record is sent on Windows with no yt-dlp
- **THEN** its entries say "needs yt-dlp", no download is offered, and installing yt-dlp makes them start within 30 s

### Requirement: Windows data folders
On Windows the config SHALL be in `%APPDATA%\Diggr\` and the cache in `%LOCALAPPDATA%\Diggr\`, with `DIGGR_CONFIG_DIR` and `DIGGR_CACHE_DIR` overriding them. Settings, crates and the Discogs token SHALL be saved by writing a temporary file and replacing the old one, and saving over an existing file SHALL succeed. A user name or folder with spaces or non-ASCII characters SHALL NOT stop settings, previews or local files from working. A broken data file SHALL still never block startup.

#### Scenario: Saving twice
- **WHEN** settings are saved, the app is closed and opened, and settings are saved again
- **THEN** both saves succeed and the second launch shows the first save's values

#### Scenario: Awkward paths
- **WHEN** the app runs with `DIGGR_CONFIG_DIR` and `DIGGR_CACHE_DIR` set to folders whose names contain spaces and an accented letter, and plays a file from a folder like that
- **THEN** settings, crates, the cache and playback all work

### Requirement: The exe's icon
`diggr.exe` SHALL carry the digging-D mark as its icon, so Explorer, the taskbar and the Start menu show it. The icon SHALL be an `.ico` in `assets/icon/` generated from `logo-mark.svg` by `icon-gen` (16, 32, 48 and 256 px), and a test SHALL fail when the committed file differs from what the generator makes. Embedding it SHALL NOT change the macOS or Linux builds, nor the first frame's 300 ms.

#### Scenario: In Explorer
- **WHEN** the exe is shown in Explorer on the laptop
- **THEN** its icon is the digging-D mark, not the generic application icon

#### Scenario: Generated icon in sync
- **WHEN** the tests run after the mark changes without regenerating
- **THEN** the icon test fails and says to run `cargo run -p ui --bin icon-gen`

### Requirement: Findings are recorded
The first run on the Windows 10 laptop SHALL produce `openspec/changes/windows-port/findings.md`, listing for each check what was done, what happened, and, for each failure, whether it was fixed in this change or left as a known gap with the reason. The change SHALL NOT be archived with an unrecorded failure.

#### Scenario: A failing check
- **WHEN** a check on the laptop fails and cannot be fixed from the Mac
- **THEN** findings.md names it as a known gap and the README's Windows notes say so

#### Scenario: Complete spike
- **WHEN** the spike is finished
- **THEN** findings.md has a line for each of: launch, GUI, local playback, `--bench`, a preview with yt-dlp, a settings save, the bridge and firewall prompt, the exe's imports
