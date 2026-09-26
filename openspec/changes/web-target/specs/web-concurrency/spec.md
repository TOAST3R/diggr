## ADDED Requirements

### Requirement: Worker-based spawner
The web `Spawner` SHALL run decode and analysis roles in separate Web Workers, each instantiating the same wasm module with a role-specific entry point.

#### Scenario: Roles
- **WHEN** the app starts playback of an uncached track
- **THEN** decoding runs in the decode worker and analysis in the analysis worker, and the main thread stays responsive (no long tasks > 50 ms from decode/analysis)

### Requirement: SharedArrayBuffer rings
PCM, tap, clock, and command channels between workers, main thread, and worklet SHALL use SharedArrayBuffer rings with Atomics, compatible with the native ring semantics (generation flush, lossy tap).

#### Scenario: Seek flush
- **WHEN** the user seeks in the browser
- **THEN** no pre-seek audio is heard after the seek
