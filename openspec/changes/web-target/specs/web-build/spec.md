## ADDED Requirements

### Requirement: Web build command
The project SHALL provide a single command that builds the wasm app and its JS glue into a `dist/` folder, reusing the core crates unmodified.

#### Scenario: Build
- **WHEN** the web build command is run
- **THEN** `dist/` contains the wasm module, JS bootstrap, worklet processor, worker bootstraps, and assets

### Requirement: Local server with isolation headers
The project SHALL provide a local serve command that serves `dist/` with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`.

#### Scenario: Isolation active
- **WHEN** the app is opened in Chrome from the local server
- **THEN** `crossOriginIsolated` is true

### Requirement: Capability check
On startup the app SHALL verify WebGPU availability and cross-origin isolation and SHALL show a clear message naming what is missing instead of failing silently.

#### Scenario: Opened from file://
- **WHEN** `index.html` is opened directly from disk
- **THEN** a message explains that the local server must be used
