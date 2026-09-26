# platform-seams Specification

## Purpose
TBD - created by archiving change audio-core. Update Purpose after archive.
## Requirements
### Requirement: Platform seam traits
The system SHALL define `AudioSink`, `Spawner`, and `FileSource` traits in `crates/platform`, and the `audio` crate SHALL access audio output, threads, and files exclusively through these traits.

#### Scenario: Core has no direct platform dependency
- **WHEN** the `audio` crate's dependency tree is inspected
- **THEN** it does not depend on cpal, std::fs, or std::thread spawning directly outside the native implementations in `crates/platform`

#### Scenario: Core compiles for the web target
- **WHEN** `cargo check -p audio --target wasm32-unknown-unknown` is run
- **THEN** it succeeds

### Requirement: Native seam implementations
The system SHALL provide native implementations: `AudioSink` over cpal, `Spawner` over std threads with priority hints (high for decode, real-time for audio where the OS allows, low for analysis), and `FileSource` over the local file system.

#### Scenario: Priority hints honored
- **WHEN** the decode thread is spawned with `Priority::High` on macOS
- **THEN** the thread runs with elevated scheduling priority, and the analysis thread spawned with `Priority::Low` does not

#### Scenario: File opened through the seam
- **WHEN** a track path is played
- **THEN** the file is opened via `FileSource` and streamed in chunks without loading the entire file into memory

