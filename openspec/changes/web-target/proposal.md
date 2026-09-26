## Why

Native is the primary target, but the same player and visualizer running in Chrome is valuable if it costs little. Because the core was built behind three platform seams, a web build should require only web implementations of those seams, a small amount of JavaScript glue, and a local dev server.

## What Changes

- `apps/web`: wasm entry points (main thread UI/visuals, decode worker, analysis worker), built with `wasm-bindgen`.
- Web `AudioSink`: an AudioWorklet (small JS processor) pulling PCM from a SharedArrayBuffer ring, with a clock derived from worklet frame counters, `AudioContext.getOutputTimestamp()`, and `outputLatency`.
- Web `Spawner`: Web Workers, each instantiating the same wasm module with a role-specific entry; rings between them over SharedArrayBuffer + Atomics.
- Web `FileSource`: files from drag-and-drop / file picker; File System Access handles persisted in IndexedDB so playlists survive reloads (Chrome).
- SongScore cache, settings, playlist, and user variants persisted in IndexedDB.
- Visual engine on WebGPU (Chrome only); scenes bundled at build time; scene folder drag-and-drop as a dev convenience.
- A local dev/serve command that serves the build with COOP/COEP headers (cross-origin isolation).
- Graceful message when WebGPU or cross-origin isolation is unavailable.

## Capabilities

### New Capabilities
- `web-build`: build pipeline, entry points, local server with isolation headers, capability checks.
- `web-audio-output`: AudioWorklet sink, SAB ring, clock, autoplay/user-gesture handling.
- `web-concurrency`: worker-based Spawner and SAB rings for decode/analysis.
- `web-storage`: file access, handle persistence, IndexedDB cache/settings/variants.

### Modified Capabilities
<!-- none -->

## Impact

- New `apps/web` (Rust + ~200 lines of JS for the worklet and worker bootstraps).
- Dependencies: wasm-bindgen, web-sys, js-sys, wasm-bindgen-futures; wgpu with the WebGPU backend; a small Rust static server (e.g., axum/tiny_http) as an `xtask serve`.
- No changes to core crates beyond implementing the existing seam traits; any core change needed indicates a seam leak to fix.
