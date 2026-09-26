## Context

Greenfield Rust project. The reference is Winamp 2.x (streaming chunked decode, input → DSP → output plugin pipeline, visualizer never blocking playback). The product's defining goal is zero perceived latency: instant start, gapless transitions, and visuals locked to what the listener hears. Native (macOS first) is the primary target; web (Chrome, AudioWorklet) must remain possible through narrow seams.

## Goals / Non-Goals

**Goals:**
- Press play → first audible sample < 30 ms on a warm app (device already open).
- Zero audible glitches under normal load; zero allocations/locks/syscalls in the audio callback.
- Gapless track transitions.
- A playback clock accurate enough that a renderer can place the audible sample within ±2 ms.
- EQ/volume changes audible within one device buffer (~10 ms).
- A core that compiles for `wasm32-unknown-unknown` (seams only differ).

**Non-Goals:**
- Network streams / internet radio.
- Crossfade between tracks, ReplayGain, DSP plugins beyond EQ (can come later).
- Exclusive-mode / bit-perfect output.
- UI (see `classic-ui`).

## Decisions

### D1. Thread model
```
 [decode thread, high prio] ──SPSC ring (PCM f32 stereo, ~0.5 s)──▶ [audio callback] ──▶ device
        ▲  commands (mpsc)                                          │  clock (seqlock atomics)
 [Engine API / UI thread] ── EQ coefs (SPSC), volume/balance/state (atomics) ─┤  tap (SPSC, lossy)
        │                                                           ▼
        └─ events (mpsc) ◀── decode thread      [visual / analysis consumers]
 [supervisor thread] owns the output stream; rebuilds it on device loss
```
- Decode thread (`worker.rs`) owns symphonia + rubato; decodes to **stereo** at the source rate, resamples through one continuous stream resampler (D5) and pushes interleaved f32 at the device rate. Internally everything is stereo; mapping to the device's channel count (mono sum, stereo, or L/R + silence for >2) happens last in the callback.
- The callback (`renderer.rs`) only: pops from the ring, applies EQ, writes the tap, applies volume/balance, maps channels, updates the clock.
- The renderer lives in a `TryCell` (an atomic-flag try-lock that never blocks). The callback outputs silence if it cannot take it; the supervisor takes it only while no stream is running, to reconfigure after a device change.
- `DecodeWorker::step()` is a non-blocking state machine; the native thread loops over it (`run()`), and a web worker can drive it the same way.
- Alternatives: decoding inside the callback (rejected: unbounded decode time glitches audio); EQ in the decode thread (rejected: EQ changes would lag by the ring length).

### D2. Device kept open
The output stream starts at app launch and outputs silence when idle. Opening a CoreAudio device costs ~50–200 ms; paying it once up front is the single largest start-latency win. Idle CPU cost is negligible. Device buffer size requested: 256–512 frames.

### D3. Ring size does not affect sync
Ring holds ~0.5 s to absorb decode hiccups. Because the clock is published from the callback (what was handed to the device), ring depth never shows up as A/V offset.

The ring is two SPSC rings: samples, and `Segment { generation, track, start_frame, frames, end_of_queue }` headers. A header is pushed only after its samples, so the callback never sees a header without data. Segments are at most 2048 frames. Play/seek/stop bump a target generation; the callback discards segments older than it. A zero-length `end_of_queue` segment tells the callback that playback has run off the end of the queue (state → Stopped).

### D4. Fast start path
On play: open file (via `FileSource`), probe, decode the first packet, push it, and keep going; the callback plays whatever is there. Tags that symphonia reads during the probe are free; anything needing extra reads (the WAV `LIST/INFO` fallback) happens in `TrackDecoder::complete_info` after the track's first audio is queued, and is then sent as a `TrackInfo` event.

Instrumentation: `Control::mark_start_request` records the request time and generation; the callback stores `host_ns − request` when it first writes audio of that generation (`EngineStats::last_start_latency_ms`, which covers seeks too).

### D5. Gapless and pre-warm
At `remaining < 30 s`, the decode thread opens the next track in the queue and pre-decodes its first ~1 s (source rate) into a side buffer, emitting `EngineEvent::PreWarm { index, track }` so `music-analysis` can start early. When the current track hits EOF, the worker continues into the next one in the same ring; every segment carries its track id, so no separate boundary marker is needed. A manual skip to the pre-warmed track reuses its decoder and buffer.

**Continuous stream resampler (changed during implementation).** The first version resampled per track. Flushing one resampler against zero padding and starting a fresh one caused a measurable click (a step of 0.05 on a 0.4-amplitude signal) at every boundary whenever the file rate differed from the device rate, which is the common case on macOS (44.1 kHz files, 48 kHz device). Now:
- `TrackDecoder` outputs source-rate stereo; the worker owns a `Stream` with one `StereoResampler` (rubato FFT, fixed input).
- Consecutive tracks with the same source rate share the resampler, so the filter history is continuous across the boundary.
- Output frames are attributed back to tracks exactly: a track whose first input frame is `i` begins at output frame `round(i · ratio)`. The clock therefore switches track at the right frame.
- When source rates differ, the old resampler's tail is flushed (exact length `round(n · ratio)`) and a new one continues the same output timeline.

### D6. Playback clock
Published by the callback once per buffer as a seqlock over plain atomics (`ClockSnapshot`):
`{ track, frame (track frame of the buffer's first frame), host_ns, latency_ns, sample_rate, state, boundary: Option<(track, buffer_offset, track_frame)>, starved, buffer_frames, epoch }`.
- Readers compute `off = (now − host_ns − latency_ns)·rate`; the audible position is `frame + off`, or `boundary.track_frame + (off − buffer_offset)` once past a boundary. `boundary` covers both a gapless track change and a seek that lands mid-buffer.
- Extrapolation is capped at one buffer + 50 ms (late callbacks). Each `ClockReader` is monotonic within a track and epoch.
- `starved` (no audio in the buffer, e.g. while loading) and paused/stopped states freeze the position.
- `epoch = device_epoch << 32 | generation`; a change sets `Position::discontinuity` (seek, device change).
- User A/V offset (±50 ms) is shared by all readers.
- Time base: `AudioSink::now_ns()`. Native callbacks stamp `host_ns` with that clock and take latency from cpal's `playback − callback` timestamps.

Alternative (atomic "position" only) rejected: doesn't let readers interpolate between callbacks at 60–240 Hz.

### D7. Equalizer
10 peaking biquads (RBJ cookbook, Q≈1.41) + preamp, per channel, in Direct Form II Transposed, f32. Band centers: 60, 170, 310, 600, 1k, 3k, 6k, 12k, 14k, 16k Hz (classic Winamp), ±12 dB. The control thread computes `EqCoefs` (a `Copy` struct) and sends them via SPSC.
- The callback interpolates coefficients and preamp per sample over 5 ms. A new change mid-ramp starts from the current interpolated point, so fast slider sweeps don't click.
- Switching EQ on/off (or a band-layout change after a rate change) jumps immediately with cleared filter state. With EQ off, samples are untouched (bit-identical).
- Bands at or above 0.45 · sample rate are bypassed.
- Presets: `EqPresets` (RON-serializable, case-insensitive names) with the 7 built-ins; persisting to disk is left to the UI.

### D8. Audio tap
The callback pushes post-EQ, pre-volume stereo frames into a separate SPSC ring of 256 × 256-frame `TapChunk`s (~1.4 s at 48 kHz). Each chunk is stamped with track id and start frame, and a chunk never spans a track change or discontinuity; partial chunks are flushed at the end of every callback. If the ring is full, the chunk is dropped and counted (`TapReader::take_dropped`); audio never waits. Consumers correlate samples with the clock via frame index. There is exactly one reader (`Engine::take_tap`).

### D9. Resilience
- **Decode errors:** undecodable packets are skipped. After 64 consecutive errors, or if a track produces no audio at all, the track fails (`EngineEvent::TrackFailed`) and the worker continues with the next playable one without a gap. A truncated file ends the track normally.
- **Device loss/change:** the cpal error callback sends to the **supervisor thread**, which owns the stream. It:
  - drops the stream;
  - retries every 250 ms until a default device is available;
  - reconfigures the renderer (channel count, EQ coefficients for the new rate, epoch bump);
  - if the sample rate changed, bumps the generation and sends `Reload { out_rate, track, secs }` so the worker restarts at the audible position at the new rate;
  - starts the new stream.
- **Ring underrun:** the callback emits silence, never blocks, and increments `underruns`. Only a gap in audio of the *current* generation counts. Waiting for the first audio after a play/seek is not an underrun (a bug in the first version, found by the device benchmark).
- **Allocation-free callback:** `rt_guard::GuardAlloc` is a counting global allocator. `Renderer::render` marks its thread as real-time. Tests (and debug builds of `apps/native`) assert zero allocations there.

### D10. Platform seams
As built (`crates/platform/src/lib.rs`):
```rust
trait AudioSink: Send + Sync {
    fn now_ns(&self) -> u64;                                   // clock time base (added)
    fn default_format(&self, cfg: &SinkConfig) -> Result<StreamFormat>;
    fn start(&self, format: &StreamFormat, render: RenderFn, on_error: ErrorFn) -> Result<Box<dyn SinkHandle>>;
}
trait Spawner: Send + Sync { fn spawn(&self, name: &str, priority: Priority, f: Box<dyn FnOnce() + Send>) -> Result<()>; }
trait FileSource: Send + Sync { fn open(&self, track: &TrackRef) -> Result<Box<dyn MediaSource>>; }
trait MediaSource: Read + Seek + Send + Sync { fn byte_len(&self) -> Option<u64>; }
```
- `default_format` is separate from `start` because the renderer must know the rate and channel count before the stream starts.
- `now_ns` was added so the engine and clock readers share the sink's time base (fakeable in tests).
- Native: `CpalSink` (always f32), `NativeSpawner` (std::thread + `thread-priority`: High → max, Low → min), `NativeFileSource` (std::fs), `native::now_ns()` (monotonic `Instant`).
- `platform::testing::ManualSink` is a test-driven sink with fake time, format changes and simulated device loss; it powers the engine integration tests.
- The `audio` crate depends only on these traits and checks cleanly for `wasm32-unknown-unknown`.

## Risks / Trade-offs

- [cpal timestamp accuracy varies by backend] → Validate on CoreAudio first; fall back to latency estimate from buffer size; expose a user A/V offset nudge (±50 ms) in settings.
- [Always-open device keeps some systems from sleeping audio hardware] → Negligible on macOS; add a "close device after N minutes idle" setting later if needed.
- [Resampling cost for mismatched rates] → rubato FFT-based resampler in the decode thread, well within budget. Dependencies are built with `opt-level = 3` even in dev builds (workspace `Cargo.toml`), since unoptimized symphonia/rubato cannot keep up with real time.
- [symphonia 0.6.1: WAV reader parses RIFF `INFO` tags but discards them] → Own minimal `LIST/INFO` parser (`IART`, `INAM`, `IPRD`), run after audio start.
- [symphonia 0.6.1: MP4 edit lists ignored] → AAC/M4A plays 1024 priming + padding frames (~43 ms at 44.1 kHz): **not sample-exact gapless for AAC**. MP3 (LAME/Xing delay+padding), FLAC, Vorbis and WAV are exact. Fix later by parsing `elst` or upgrading symphonia.
- [Seqlock readers may spin] → Writes are tiny and ≤ once per callback; readers retry at most a few times.
- [Timing-sensitive tests] → Engine tests pull from `ManualSink` at 2× real time and assert zero underruns. 4× real time flaked under parallel test load (1 in 10 runs).

## Verification status

- 64 automated tests (`cargo test --workspace`), 0 failures in 15 consecutive runs:
  - unit tests per module;
  - `tests/decode_formats.rs`: every format, both rates, tags, seek accuracy, corrupt/garbage files;
  - `tests/engine.rs`: bit-exact playback, pause, seek, bit-exact gapless, resampled gapless, clock at a click within ±2 ms with 12 ms latency, device loss at a new rate, tap, EQ/volume;
  - `tests/rt_alloc.rs`: zero callback allocations across all paths, and the guard is shown to detect allocations.
- Real CoreAudio device (M-series Mac, 44.1 kHz, 512-frame buffer, muted), 60 s files in MP3/FLAC/OGG/M4A/WAV:
  - start 7.9–20.8 ms (target < 30);
  - seek 12.1–20.0 ms (target < 50);
  - device open 448 ms, paid once at launch.
- Dropped from this change's tasks at archive time (not performed; the tooling exists):
  - former 9.2: a 1-hour, underrun-free run on the real device. The 5-minute run's single underrun was the accounting bug above, and it hasn't been re-run since the fix. Use `winamp-native --bench`.
  - former 6.4: the acoustic clock check (`winamp-native --click-test`, needs a speaker → mic path).

## Open Questions

- EQ labels: classic 60/170/310 vs the screenshot's 70/180/320. Decision here uses classic values; the label set is a UI concern and trivially changeable.
- Default device buffer size on macOS: 512 frames is the current default and meets the start target; 256 has not been measured yet.
