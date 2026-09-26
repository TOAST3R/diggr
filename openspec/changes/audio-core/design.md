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
 [decode thread, high prio] ──SPSC ring (PCM f32, ~0.5 s)──▶ [audio callback] ──▶ device
        ▲  commands (SPSC)                                        │  clock (atomics)
 [control / UI thread] ───────────────────────────────────────────┤  tap (SPSC, lossy)
                                                                  ▼
                                                      [visual / analysis consumers]
```
- Decode thread owns symphonia + rubato; converts to the device's rate/channel layout and pushes interleaved f32.
- The callback only: pops from the ring, applies EQ + volume/balance, writes the device buffer, pushes to the tap, updates the clock.
- Alternatives: decoding inside the callback (rejected: unbounded decode time glitches audio); EQ in the decode thread (rejected: EQ changes would lag by the ring length).

### D2. Device kept open
The output stream starts at app launch and outputs silence when idle. Opening a CoreAudio device costs ~50–200 ms; paying it once up front is the single largest start-latency win. Idle CPU cost is negligible. Device buffer size requested: 256–512 frames.

### D3. Ring size does not affect sync
Ring holds ~0.5 s to absorb decode hiccups. Because the clock is published from the callback (what was handed to the device), ring depth never shows up as A/V offset. Seek/stop flush the ring via a generation counter: each ring chunk carries a generation; the callback drops chunks whose generation is stale.

### D4. Fast start path
On play: open file (via `FileSource`), probe, decode the first packet(s), push, and signal the callback. The callback begins as soon as the ring holds ≥ 1 device buffer. Everything else (full metadata read, duration estimate refinement) happens after audio has started.

### D5. Gapless and pre-warm
At `remaining < 30 s`, the decode thread opens the next track in the queue and pre-decodes its first ~1 s into a side buffer. When the current track's decoder hits EOF it continues pushing from the next track into the same ring with a `TrackBoundary { track_id, frame }` marker, so the clock switches track id at the exact audible frame. A manual skip uses the D4 fast path (and the pre-warmed buffer if the target is the pre-warmed track). An `on_prewarm(track)` hook lets `music-analysis` start analyzing the next track early.

### D6. Playback clock
Published by the callback as a seqlock over plain atomics:
`{ track_id, frames_played (at callback start), host_instant (callback timestamp), output_latency_frames, sample_rate, state }`.
Readers compute `audible_frame = frames_played + (now − host_instant)·rate − output_latency_frames`, clamped and monotonic per track. On native, the timestamp/latency come from cpal's `OutputCallbackInfo` timestamps. Alternative (atomic "position" only) rejected: doesn't let readers interpolate between callbacks at 60–240 Hz.

### D7. Equalizer
10 peaking biquads (RBJ cookbook, Q≈1.41) + preamp, per channel, in Direct Form II Transposed, f32. Band centers: 60, 170, 310, 600, 1k, 3k, 6k, 12k, 14k, 16k Hz (classic Winamp), ±12 dB. The control thread computes coefficients and sends them via SPSC; the callback swaps them in with a short (≈5 ms) linear gain ramp to avoid zipper noise. Bands above Nyquist are bypassed.

### D8. Audio tap
The callback pushes post-EQ, pre-volume stereo frames with their starting frame index into a separate SPSC ring (~1 s). If full, the callback drops the push (consumers lose data, audio never waits). Consumers correlate samples with the clock via frame index.

### D9. Resilience
- Decode errors: skip the bad packet, log once per track; if a track yields no audio within 1 s of trying, mark it failed and advance.
- Device loss/change (cpal error callback): control thread rebuilds the stream on the new default device; the decode position is kept; clock publishes a discontinuity flag.
- Ring underrun: callback emits silence, increments an underrun counter (exposed for diagnostics), never blocks.
- Callback stays allocation-free: verified by a test allocator in debug builds (`assert_no_alloc`-style).

### D10. Platform seams
```rust
trait AudioSink   { fn start(cfg, render: Box<dyn FnMut(&mut [f32], CallbackInfo) + Send>) -> Result<SinkHandle>; }
trait Spawner     { fn spawn(name, priority: Priority, f: impl FnOnce() + Send + 'static); }
trait FileSource  { fn open(&self, id: &TrackRef) -> Result<Box<dyn MediaSource>>; fn size(..); }
```
Native: cpal / std::thread (+ audio_thread_priority) / std::fs. The `audio` crate depends only on these traits.

## Risks / Trade-offs

- [cpal timestamp accuracy varies by backend] → Validate on CoreAudio first; fall back to latency estimate from buffer size; expose a user A/V offset nudge (±50 ms) in settings.
- [Always-open device keeps some systems from sleeping audio hardware] → Negligible on macOS; add a "close device after N minutes idle" setting later if needed.
- [Resampling cost for mismatched rates] → rubato FFT-based resampler in the decode thread, well within budget.
- [symphonia AAC/M4A coverage gaps] → Treat as decode failure path (skip track, surface error); revisit if user library hits it.
- [Seqlock readers may spin] → Writes are tiny and ≤ once per callback; readers retry at most a few times.

## Open Questions

- EQ labels: classic 60/170/310 vs the screenshot's 70/180/320. Decision here uses classic values; the label set is a UI concern and trivially changeable.
- Default device buffer size on macOS: 256 vs 512 frames — decide after measuring underrun rate.
