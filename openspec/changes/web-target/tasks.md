## 1. Build and serve

- [ ] 1.1 Create `apps/web` crate (cdylib) with wasm-bindgen entry points per role (main, decode, analysis)
- [ ] 1.2 `cargo xtask web`: build, wasm-bindgen, assemble `dist/` (index.html, bootstrap JS, worklet, worker bootstraps, assets)
- [ ] 1.3 `cargo xtask serve`: localhost static server with COOP/COEP headers
- [ ] 1.4 Startup capability check (crossOriginIsolated, navigator.gpu) with user-facing message

## 2. Concurrency seam

- [ ] 2.1 SAB ring primitives (f32 PCM with generation, lossy tap, clock seqlock, command queue) in Rust (js-sys) and JS
- [ ] 2.2 Web `Spawner`: spawn role workers loading the shared wasm module; message protocol

## 3. Audio seam

- [ ] 3.1 AudioWorklet processor JS: pull from SAB ring, gain/balance, underrun silence, counters
- [ ] 3.2 Web `AudioSink`: create context + worklet at load, resume on first gesture, keep running
- [ ] 3.3 Move EQ into the decode worker for web with a short ring; verify EQ response time
- [ ] 3.4 Web clock via getOutputTimestamp + outputLatency behind `ClockReader`

## 4. Storage seam

- [ ] 4.1 Web `FileSource` over File/Blob chunked reads; drag-drop and picker
- [ ] 4.2 Persist File System Access handles in IndexedDB; permission re-request on first gesture
- [ ] 4.3 IndexedDB stores for scores, settings, playlist, variants

## 5. Visuals and UI on web

- [ ] 5.1 eframe web with WebGPU backend; fullscreen via key gesture
- [ ] 5.2 Bundle scenes/prelude/director at build time; scene folder drag-and-drop loader

## 6. Verification

- [ ] 6.1 Play each supported format in Chrome; seek, gapless, EQ, volume
- [ ] 6.2 Click-track A/V sync check (< 1 frame) in Chrome
- [ ] 6.3 Confirm no core crate was modified for web beyond seam impls
