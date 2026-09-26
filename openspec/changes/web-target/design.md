## Context

The core (`audio`, `analysis`, `visuals`, `ui`) depends only on `AudioSink`, `Spawner`, and `FileSource`. The user targets Chrome only, hosting locally. Chrome supports WebGPU (including compute), AudioWorklet, SharedArrayBuffer (when cross-origin isolated), File System Access, and fullscreen on user gesture.

## Goals / Non-Goals

**Goals:**
- Same features as native except scene hot reload from disk.
- Press play → sound < 50 ms after the AudioContext is running.
- A/V offset < 1 frame using the browser's output timestamp.
- Effort: only seam implementations + glue; zero forks of core logic.

**Non-Goals:**
- Safari/Firefox support, WebGL2 fallback.
- Public hosting / deployment pipeline.
- Offline PWA install.

## Decisions

### D1. Process layout
```
 Main thread (wasm: ui + visuals, eframe web, WebGPU)
   │  commands / snapshots (postMessage + SAB)
   ├── Decode worker (wasm: audio decode + resample) ──SAB PCM ring──▶ AudioWorklet (JS)
   │                                                                   │ frames consumed,
   ├── Analysis worker (wasm: analysis, low effective priority)        │ callback times (SAB)
   │        └─ SongScore snapshots → main (postMessage, transferable)  ▼
   └── Clock reader on main: SAB counters + getOutputTimestamp + outputLatency
```
One wasm module, multiple instances (no wasm shared-memory threads, so no nightly `build-std` or atomics target features). Rings are standalone SharedArrayBuffers accessed with `Float32Array` + `Atomics` on both sides; wasm copies in/out (copy cost is negligible at audio rates).
Alternative (wasm threads with shared memory) rejected: requires nightly toolchain and complicates the build for little gain.

### D2. Worklet in JS, EQ in Rust
The AudioWorklet processor is small JS: read ring → apply gain/balance → write output → update counters. The EQ runs in the decode worker instead of the callback (web deviation), with the ring kept short (~50 ms) so EQ changes still land quickly. Alternative (wasm inside the worklet) rejected for MVP: more complex loading; revisit if EQ latency is noticeable.

### D3. Always-open device → resume on first gesture
Chrome's autoplay policy requires a user gesture. The AudioContext and worklet are created at load (suspended) and resumed on the first click/key. Afterwards they remain running with silence, matching native's always-open behavior.

### D4. Clock
The worklet writes `(frames_rendered, currentFrame)` each quantum into SAB. Main thread maps AudioContext time to performance time via `getOutputTimestamp()` and subtracts `outputLatency` to get the audible frame. The result feeds the same `ClockReader` interface.

### D5. Files and persistence
- `FileSource` over `File`/`Blob` slices (chunked reads, no full-file load) for dropped/picked files.
- File System Access handles stored in IndexedDB; on reload, permission is re-requested on the first gesture.
- IndexedDB stores: `scores` (SongScore cache), `settings`, `playlist`, `variants` (user-kept variants; bundled ones compiled in).

### D6. Build and serve
`cargo xtask web` → `cargo build --target wasm32-unknown-unknown -p web` → `wasm-bindgen --target web` → copy JS glue + assets to `dist/`. `cargo xtask serve` serves `dist/` on localhost with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`. At startup, the app checks `crossOriginIsolated` and `navigator.gpu`, showing a clear message if missing.

## Risks / Trade-offs

- [EQ latency higher than native] → Short ring on web; if needed, move EQ into a wasm worklet later.
- [Worker scheduling can starve decode under heavy main-thread load] → Decode worker keeps ~300 ms of lead in a larger pre-ring; worklet ring stays short.
- [outputLatency accuracy varies by audio device] → Same user A/V offset setting as native.
- [Permission prompts for persisted file handles] → Batch re-request on first gesture; fall back to "re-add files" when denied.

## Open Questions

- Whether to embed the default music folder concept on web (a directory handle) for MVP.
