## 1. Workspace and seams

- [ ] 1.1 Create cargo workspace with `crates/platform`, `crates/audio`, `apps/native`; set up fmt/clippy config and CI-ready `cargo test` / `cargo clippy` commands
- [ ] 1.2 Define `AudioSink`, `Spawner` (with `Priority`), `FileSource`/`MediaSource` traits in `crates/platform`
- [ ] 1.3 Implement native `Spawner` (std::thread + audio_thread_priority) and `FileSource` (std::fs)
- [ ] 1.4 Implement native `AudioSink` over cpal, exposing callback timestamps and output latency in `CallbackInfo`
- [ ] 1.5 Add a `cargo check -p audio --target wasm32-unknown-unknown` check to verify the core stays portable

## 2. Rings and real-time guard

- [ ] 2.1 Add rtrb-based PCM ring with generation-tagged chunks and flush-by-generation
- [ ] 2.2 Add debug-build allocation guard for the audio callback and a test that exercises it
- [ ] 2.3 Add command SPSC (control → callback) for EQ coefficients, volume, balance, state

## 3. Decode pipeline

- [ ] 3.1 Implement decode thread: probe via symphonia, decode packets, convert to interleaved f32
- [ ] 3.2 Add rubato resampling and channel mapping to the device format
- [ ] 3.3 Implement fast start path (first packet → ring → callback start) with latency instrumentation
- [ ] 3.4 Implement seek with ring flush and clock discontinuity
- [ ] 3.5 Skip corrupt packets; mark undecodable tracks failed and advance
- [ ] 3.6 Read metadata (tags, duration, bitrate) asynchronously after audio start; fall back to file name

## 4. Engine and transport

- [ ] 4.1 Implement `Engine` API: play/pause/resume/stop/seek/next/prev/volume/balance and a track queue
- [ ] 4.2 Keep the output stream open from startup, emitting silence when idle
- [ ] 4.3 Apply volume and balance in the callback with ramping
- [ ] 4.4 Handle device loss/change: rebuild stream on the default device, preserve position
- [ ] 4.5 Track underrun counter and expose diagnostics

## 5. Gapless and pre-warm

- [ ] 5.1 Pre-open and pre-decode the next track at 30 s remaining; emit pre-warm event
- [ ] 5.2 Continue into next track in the same ring with `TrackBoundary` markers
- [ ] 5.3 Use pre-warmed buffer on manual skip to the pre-warmed track
- [ ] 5.4 Test: concatenated sine across two files has no discontinuity at the boundary

## 6. Playback clock

- [ ] 6.1 Implement seqlock clock publication in the callback and `ClockReader` with interpolation and latency compensation
- [ ] 6.2 Switch track id at the audible boundary frame; flag discontinuities on seek/device change
- [ ] 6.3 Add user A/V offset setting (±50 ms)
- [ ] 6.4 Build a loopback click-track test tool to measure clock accuracy (target ±2 ms) on macOS

## 7. Equalizer

- [ ] 7.1 Implement RBJ peaking biquad (DF2T, f32) and a 10-band + preamp chain per channel
- [ ] 7.2 Compute coefficients on the control thread; swap in callback with a ~5 ms ramp; bypass bands above Nyquist
- [ ] 7.3 Unit tests: +12 dB at band center within ±0.5 dB; EQ off is bit-identical
- [ ] 7.4 Presets: load/save named presets; ship Flat, Rock, Pop, Dance, Techno, Full Bass, Full Treble

## 8. Audio tap

- [ ] 8.1 Implement lossy position-stamped tap SPSC (post-EQ, pre-volume) and `TapReader`
- [ ] 8.2 Test: stalled consumer causes dropped-chunk count, never an underrun

## 9. Native harness

- [ ] 9.1 `apps/native` CLI: play files/queue from arguments, keyboard transport, print clock and latency stats
- [ ] 9.2 Verify targets on macOS: start < 30 ms, seek < 50 ms, zero underruns over a 1-hour playlist
