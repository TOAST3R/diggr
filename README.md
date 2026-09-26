# winamp_rust

A Winamp 2.x–inspired music player in Rust, built for **speed and zero perceived latency**,
with (coming) a fullscreen fractal visualizer that follows the rhythm and structure of the music.

Progress:

 **audio-core** (33/35 tasks) → classic-ui → music-analysis → visual-engine → web-target

The first milestone, `audio-core`, is implemented: the audio engine plus a terminal harness.
There is no GUI yet (that's `classic-ui`).

## Quick start

Requires the stable Rust toolchain (the project was built with Rust 1.98):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # if Rust is not installed
cargo build --release
```

### Play music (terminal player)

```sh
cargo run --release -p winamp-native -- ~/Music/*.mp3
```

| Key | Action | | Key | Action |
|---|---|---|---|---|
| `z` | previous | | `←` / `→` | seek −5 s / +5 s |
| `x` | play | | `↑` / `↓` | volume |
| `c` | pause / resume | | `e` | EQ on/off |
| `v` | stop | | `q` / `Esc` | quit |
| `b` | next | | | |

The status line shows the audible position (from the playback clock), track, volume, EQ, device
rate, the last start/seek latency, underruns, and, in debug builds, allocations detected
inside the audio callback (should always be 0).

Supported formats: MP3, FLAC, WAV, OGG Vorbis, AAC/M4A (pure-Rust decoding via symphonia).

### Measure latency on your machine

```sh
cargo run --release -p winamp-native -- --bench --volume 0 file1.mp3 file2.flac …
```

For each file this measures press-play → first audio at the device and three seeks. It then
plays the whole queue gaplessly and counts underruns. It exits non-zero if a target is missed:
start < 30 ms, seek < 50 ms, 0 underruns, 0 callback allocations (the allocation check needs a
debug build: drop `--release`). `--volume 0` runs the full pipeline silently.

Last measured on an M-series Mac (CoreAudio, 44.1 kHz, 512-frame buffer), 60 s files:
start 7.9–20.8 ms, seek 12.1–20.0 ms.

### Check A/V clock accuracy acoustically

```sh
cargo run --release -p winamp-native -- --click-test
```

Plays 16 clicks and records them with the default microphone. It reports how far the heard
clicks are from when the playback clock said they'd be audible (target ±2 ms). This needs your
speakers audible to the mic, and macOS will ask for microphone permission.

## Tests

```sh
cargo test --workspace            # 64 tests, ~5 s; no audio hardware needed
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo check -p audio -p platform --target wasm32-unknown-unknown   # core stays web-portable
```

What's covered:

- **Unit tests** per module: lock-free ring, seqlock clock (interpolation, latency, gapless
  boundary, monotonicity, concurrency), EQ (±0.5 dB at band centers, bit-identical when off, no
  clicks when sweeping, Nyquist bypass), tap, renderer, resampler.
- **`crates/audio/tests/decode_formats.rs`**: every format at 44.1 and 48 kHz (pitch, length,
  tags), accurate seeking, corrupt and garbage files. Fixtures are in
  `crates/audio/tests/fixtures/`.
- **`crates/audio/tests/engine.rs`**: the full engine against `platform::testing::ManualSink`, a
  sink the test drives by hand with fake time. Covers:
  - bit-exact playback, pause and seek;
  - bit-exact gapless playback, and gapless playback with resampling;
  - the clock hitting a click within ±2 ms with 12 ms of output latency;
  - device loss that comes back at a new sample rate;
  - the tap, EQ, volume and balance.
- **`crates/audio/tests/rt_alloc.rs`**: a counting global allocator proves the audio callback
  never allocates on any path.

## How it works

```
 decode thread ──lock-free ring (~0.5 s)──▶ audio callback ──▶ device (opened once, always running)
   symphonia → stereo → rubato             EQ → tap → volume/balance
   gapless + pre-warm next track           publishes the playback clock
                                                  │
                   visuals / analysis ◀── clock + tap (lock-free, never block audio)
```

- **Instant start:** the output device opens at launch and stays open (emitting silence), so
  pressing play only has to decode one packet.
- **Zero-offset visuals:** the callback publishes *which frame is at the speaker right now*
  (callback time + output latency). Consumers interpolate it at any frame rate.
- **Gapless:** the next track is opened and pre-decoded 30 s before the end. One continuous
  resampler carries across track boundaries, so even 44.1 kHz files on a 48 kHz device join
  seamlessly.
- **Real-time safe:** no allocations, locks, or waiting in the callback. Seeks flush by
  generation number instead of locking.
- **Resilient:** corrupt frames and unplayable files are skipped. If the output device
  disappears, the stream is rebuilt at the same position, even at a new sample rate.

Full details and the reasoning behind each decision:
[`openspec/changes/archive/2026-09-26-audio-core/design.md`](openspec/changes/archive/2026-09-26-audio-core/design.md).

## Layout

```
crates/platform   seam traits (AudioSink, Spawner, FileSource) + native impls + ManualSink for tests
crates/audio      decode, ring, renderer (callback), clock, EQ, tap, decode worker, Engine API
apps/native       terminal player, --bench, --click-test
openspec/         specs and plans for every milestone (see below)
```

## Known limitations

- **AAC/M4A is not sample-exact gapless:** symphonia 0.6.1 ignores MP4 edit lists, so about
  43 ms of encoder priming and padding plays. MP3, FLAC, Vorbis and WAV are exact.
- **WAV tags** (RIFF `INFO`) are read by our own small parser, because symphonia 0.6.1 drops them.
- **Still to verify:** a 1-hour playlist with zero underruns, and the acoustic clock check.
- Surround files are played as their front left/right channels.

## Roadmap (OpenSpec)

Each milestone is an OpenSpec change with a proposal, design, specs and tasks in
`openspec/changes/`:

1. `audio-core`: this engine (done apart from the two checks above)
2. `classic-ui`: the Winamp-style skinned player, EQ and playlist (egui/wgpu), and the
   fullscreen key
3. `music-analysis`: beat grid, phrases, build/drop/breakdown detection that analyzes ahead of
   the playhead
4. `visual-engine`: fractal scenes (WGSL), a modulation matrix, a director, a fader deck, and an
   auto-fading track overlay
5. `web-target`: the same app in Chrome via WebAssembly, AudioWorklet and WebGPU

```sh
openspec list                      # progress per change
openspec show audio-core           # read a change
```
