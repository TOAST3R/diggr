# winamp_rust

A Winamp 2.x–inspired music player in Rust, built for **speed and zero perceived latency**,
with (coming) a fullscreen fractal visualizer that follows the rhythm and structure of the music.

Progress:

 **audio-core** ✅ → **classic-ui** ✅ → music-analysis → visual-engine → web-target

Implemented so far: the audio engine (`audio-core`) and the classic Winamp-style player window
(`classic-ui`), including a fullscreen mode that the fractal visualizer will plug into.

## Launch the app

Starting from nothing on macOS, run these from the project folder:

```sh
xcode-select --install                                           # 1. once: the C linker (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # 2. once: install Rust
source "$HOME/.cargo/env"                                        # 3. put cargo on PATH in this terminal
cargo run --release -p winamp-native                             # 4. build (a few minutes the first time) and launch
```

The player window opens. Drag music files or folders onto it, or start with the bundled
test tones:

```sh
cargo run --release -p winamp-native -- crates/audio/tests/fixtures/tone.*
cargo run --release -p winamp-native -- ~/Music/album/*.flac    # your own files
```

After the first build you can also start the program directly, without `cargo`:

```sh
./target/release/winamp-native
```

Press `F` for fullscreen visuals, `Esc` to leave, and Cmd+Q to quit. Your playlist and settings
are kept for next time.

If you get `zsh: command not found: cargo`, repeat step 3. To make it permanent, run
`echo 'source "$HOME/.cargo/env"' >> ~/.zshrc`. Other platforms and details are under
[Setup](#setup), and everything the window can do is under [Try it](#try-it).

## Setup

### 1. Prerequisites

| Platform | Needs |
|---|---|
| macOS | Xcode command-line tools (the linker): `xcode-select --install` |
| Linux | a C toolchain plus ALSA headers: `sudo apt install build-essential pkg-config libasound2-dev` |
| Windows | Visual Studio Build Tools (C++); rustup offers to install them |

### 2. Rust

The project uses stable Rust (built with 1.98, edition 2024):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"                     # puts cargo on PATH in this shell
rustup component add clippy rustfmt           # for the lint/format commands below
rustup target add wasm32-unknown-unknown      # only for the web-portability check
```

> **`zsh: command not found: cargo`?** Cargo is installed in `~/.cargo/bin`, but that folder
> isn't on your `PATH`. Fix it for the current shell with `source "$HOME/.cargo/env"`, or for
> every new shell with:
>
> ```sh
> echo 'source "$HOME/.cargo/env"' >> ~/.zshrc    # bash: ~/.bashrc
> ```
>
> Check with `cargo --version`. Conda environments (`(base)`) don't matter here.

### 3. Build

```sh
cargo build --release
```

The first build downloads and compiles all dependencies, which takes a few minutes; later
builds are incremental. Dependencies are compiled with optimizations even in debug builds
(see `Cargo.toml`), because unoptimized decoders can't keep up with real-time playback.

## Try it

### The player

```sh
cargo run --release -p winamp-native                          # opens with your last playlist
cargo run --release -p winamp-native -- ~/Music/album/*.flac  # replaces the playlist and plays
```

No music handy? The repo includes short test tones:
`cargo run --release -p winamp-native -- crates/audio/tests/fixtures/tone.*`

The window is the classic three-part Winamp layout: main player, equalizer and playlist. It's
drawn from an original pixel-art skin, at double size by default (switch with **OPT** in the
playlist).

- **Add music:** drag files or folders onto the window (folders are scanned recursively; `.m3u`
  playlists are expanded), use **ADD**, or press Cmd+O. **Eject** opens files and plays them.
- **Playlist:**
  - double-click an entry to play it;
  - Shift/Cmd-click to select several;
  - drag to reorder;
  - Delete removes the selection;
  - drag the bottom-right grip to show more rows;
  - **MISC** imports and exports M3U/M3U8.
- **Main window:**
  - click the time to switch between elapsed and remaining;
  - click the mini visualizer to cycle spectrum → oscilloscope → off;
  - **SHUFFLE**;
  - **REP** cycles off → all → one (all gapless);
  - **EQ** and **PL** show or hide the other sections.
- **Equalizer:**
  - **ON** enables it;
  - drag the sliders, or double-click one to reset it to 0 dB;
  - **PRESETS** loads the built-in presets, and can save or delete your own.
- **Move the window** by dragging any title bar. Your playlist, settings and presets are saved
  in `~/Library/Application Support/winamp_rust/`. Set `WINAMP_CONFIG_DIR=/some/dir` to use
  another folder, for example for testing.

| Key | Action | | Key | Action |
|---|---|---|---|---|
| `Z` | previous | | `←` / `→` | seek −5 s / +5 s |
| `X` | play | | `↑` / `↓` | volume |
| `C` | pause / resume | | `F` | fullscreen visuals (`F`/`Esc` to leave) |
| `V` | stop | | `Delete` | remove selected entries |
| `B` | next | | `Cmd+O` / `Cmd+A` | add files / select all |

**Fullscreen (`F`)** currently shows a placeholder visual: the screen flashes on every beat of a
fixed 120 BPM grid, timed from the playback clock, so you can judge audio/visual sync by eye.
The fractal visual engine replaces it in a later milestone. Transport keys keep working in
fullscreen.

### Terminal player

The earlier terminal harness is still available:

```sh
cargo run --release -p winamp-native -- --tui ~/Music/*.mp3
```

Run it in a real terminal (Terminal, iTerm, or the VS Code terminal): it reads keys directly,
so it won't work with piped input or in a non-interactive shell. By default it plays at 80%
volume; add `--volume 0.3` to start quieter.

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
cargo run --release -p winamp-native -- --bench crates/audio/tests/fixtures/tone.*   # quick check
```

For each file this measures press-play → first audio at the device and three seeks. It then
plays the whole queue gaplessly. Underruns are counted over the whole session, including the
seeks. It exits non-zero if a target is missed:
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
cargo test --workspace            # 108 tests, a few seconds after the first build; no audio hardware or display needed
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo check -p audio -p platform --target wasm32-unknown-unknown   # core stays web-portable
```

Run a single test file or test with `cargo test -p audio --test engine` or
`cargo test -p audio gapless`.

The test fixtures in `crates/audio/tests/fixtures/` were generated with ffmpeg (2 s, 440 Hz, tagged
`M83 / Midnight_City`). You only need ffmpeg if you want to regenerate them.

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
- **`crates/ui`** (unit tests):
  - playlist selection, reordering and totals;
  - shuffle order;
  - folder scanning and M3U round trip;
  - settings, playlist and preset persistence;
  - spectrum bars following the *audible* frame;
  - EQ curve;
  - skin validation, and that the committed skin matches its generator;
  - repaint policy;
  - headless egui click/drag tests of the skinned widgets.
- **`crates/ui/tests/large_add.rs`**: 2,000 files get their metadata read while the engine plays
  in real time, with zero underruns.

Measure the player's launch time (the target is under 300 ms):

```sh
cargo run --release -p winamp-native -- --startup-time
```

On an M-series Mac it takes 142–171 ms, including with a 500-entry saved playlist. The very
first launch after a build takes about 0.7 s while macOS compiles and caches GPU shaders.

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
crates/ui         the player window: skin, main/EQ/playlist sections, fullscreen host, playlist model
assets/skin       the bundled original skin (atlas.png + skin.ron), generated by `cargo run -p ui --bin skin-gen`
apps/native       the desktop app: GUI (default), --tui, --bench, --click-test, --startup-time
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

1. `audio-core`: the audio engine ✅ (done and archived, apart from the two checks above)
2. `classic-ui`: the Winamp-style skinned player, EQ and playlist (egui/wgpu), and the
   fullscreen key ✅ (done and archived)
3. `music-analysis`: beat grid, phrases, build/drop/breakdown detection that analyzes ahead of
   the playhead
4. `visual-engine`: fractal scenes (WGSL), a modulation matrix, a director, a fader deck, and an
   auto-fading track overlay
5. `web-target`: the same app in Chrome via WebAssembly, AudioWorklet and WebGPU

Finished changes move to `openspec/changes/archive/`, and their requirements become the living
specs in `openspec/specs/`.

The `openspec` CLI is optional; it's only needed to browse or advance the plans:

```sh
npm install -g @fission-ai/openspec   # requires Node.js
openspec list                         # active changes and task progress
openspec list --specs                 # specs of what's already built
openspec show classic-ui              # read the next milestone
openspec show audio-playback --type spec
```
