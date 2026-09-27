# winamp_rust

A Winamp 2.x–inspired music player in Rust, built for **speed and zero perceived latency**,
with a fullscreen fractal visualizer that follows the rhythm and structure of the music.

Progress:

 **audio-core** ✅ → **classic-ui** ✅ → **music-analysis** ✅ → **visual-engine** ✅ → web-target

Implemented so far: the audio engine (`audio-core`), the classic Winamp-style player window
(`classic-ui`), music analysis ahead of the playhead (`music-analysis`), and the fullscreen
visual engine (`visual-engine`), which holds a steady 60 fps on an M2 MacBook.

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

**Fullscreen (`F`)** shows the fractal visuals (see [Visuals](#visuals)). Transport keys keep
working in fullscreen.

### Visuals

Fullscreen visuals run on musical time: beats, bars and phrases from the analysis, with kicks,
snares and hats fired as the playhead crosses them, so motion lands on the beat you hear. At
each section change the **director** picks what to show. A big rise in energy cuts to your
highest-rated look with a flash. A drop in energy crossfades to something calm. A section that
comes back returns to the look it had before. Other changes morph to a sibling look, and the
`stretch` macro follows the track's tension. The same track always gets the same show.

Four scenes ship with it, each with two variants: **Julia Tunnel** (2D fractal), **Liquid
Feedback** (the MilkDrop feel), **KIFS Cathedral** (raymarched 3D) and **Flame** (a compute-shader
fractal flame). The artist, title and progress (with section ticks) show for 5 s on entering
fullscreen, on each new track, and when you move the mouse or press a key.

| Key | Action |
|---|---|
| `D` | show/hide the fader deck: 6 macros (intensity, chaos, stretch, speed, hue, feedback) and the scene's parameters |
| `M` / `Shift+M` | mutate the current look (small / big step) |
| `K` | keep: save the current look, fader positions included, as a new variant |
| `Backspace` | undo back through the looks you had |
| `1`–`5` | rate the current look (the director prefers higher ratings) |

On the deck, dragging a fader switches it to MANUAL. When you let go, it holds, then glides back
to automation after the RETURN time (1 beat, 1 bar, 4 bars, a phrase, or ∞), landing on a bar
line. Click RETURN in the deck header to change the global setting. Right-click a fader to give it
its own RETURN, and double-click it to hand it back to automation now. Speed snaps to ¼, ½, 1, 2
and 4×.

**Make it yours:** on first use the scenes are copied to
`~/Library/Application Support/winamp_rust/visuals/`, and any file you save there is picked up
while the music plays:

```
visuals/
  director.ron              the rules for what happens at section changes (commented)
  prelude/*.wgsl            helpers every scene can call: complex math, noise, palettes, SDFs
  scenes/<id>/scene.ron     name, tags, parameters (type, default, range), macro mappings, routes
  scenes/<id>/scene.wgsl    fn scene(uv: vec2f, m: Music, p: Params) -> vec4f
  variants/<id>/<name>.ron  saved looks (K writes these; ratings live here too)
```

A new look is one `.wgsl` and one `.ron` file in a new `scenes/<id>/` folder. You write only
`fn scene`. The engine generates `p.<param>` from your manifest and passes the music as `m`: for
example `m.beat` (the phase within the beat), `m.motion` (beats, scaled by the speed macro), `m.kick`
(beats since the last kick, so use `pulse(m.kick, 4.0)` for a punch), `m.energy`, `m.tension`,
and `band(m, i)` for the 19 spectrum bars. Routes in the manifest connect signals to parameters
without writing code, for example
`(source: Kick, shapers: [Envelope(attack: Ms(5), release: Ms(120)), Range(0, 0.35)], target: "zoom")`.
If you save a broken shader or manifest, the previous version keeps running, and a message
shows the file, line and error for 8 seconds. To reset a file, delete it and it is restored from
the bundled copy the next time you enter fullscreen.

Resolution adapts to hold the frame rate. You can measure the scenes on your GPU:

```sh
cargo run -p visuals --example visual_bench --release        # offscreen, native Retina size
WINAMP_VISUAL_BENCH=1 cargo run -p winamp-native --release   # in the app: press F; 15 s with no visuals,
                                                             # then 15 s per scene; results in .../visuals/bench.txt
WINAMP_FRAME_STATS=1 cargo run -p winamp-native --release    # per-second frame timings (app, visuals, present)
cargo run -p ui --example fullscreen_probe --release         # what a blank eframe window can present
```

### Music analysis

While a track plays, the app analyzes it about 2 minutes ahead of what you hear: tempo, the beat
grid, bars, and sections (intro, build, drop, breakdown, groove, outro), with a countdown to the
next drop. The first 32 bars are ready about half a second after you press play, and results are
cached in `~/Library/Caches/winamp_rust/`, so a second play is instant. Set `WINAMP_CACHE_DIR`
to use another folder. It runs at low priority and never delays playback.

In fullscreen:

| Key | Action |
|---|---|
| `T` | show/hide the analysis strip: BPM, current section, drop countdown, section bands, beats (taller on bar starts), the tension curve and the playhead |
| `A` | annotation mode, for teaching the analyzer your music |
| `Space` *(annotating)* | tap along with the beat |
| `1`–`6` *(annotating)* | mark where a section starts: 1 intro, 2 build, 3 drop, 4 breakdown, 5 groove, 6 outro |

Annotations are saved immediately to `~/Library/Caches/winamp_rust/annotations/`. To see how the
analyzer scores against them:

```sh
cargo run --release -p analysis --bin analysis-eval
```

This prints each track's beat accuracy and how many of your section marks it found. The targets
are ≥ 90% and ≥ 70%.

No music annotated yet? You can try it on generated tracks:

```sh
cargo run --release -p analysis --example make_annotated -- /tmp/eval
cargo run --release -p analysis --bin analysis-eval -- /tmp/eval/annotations
```

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
cargo test --workspace            # 216 tests, under a minute after the first build; no audio hardware or display needed
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
- **`crates/analysis`**: generated electronica with exact ground truth, testing:
  - kicks within 20 ms;
  - tempo at 124/128/140/174 BPM, and tempo changes;
  - beatless intros;
  - downbeats and section boundaries on the right bar;
  - section kinds, and repeated drops sharing a label;
  - streaming, seek, cache and pre-warm.

  Two long checks run on request:
  - `-- --ignored timing`: speed targets;
  - `-- --ignored two_hour`: flat memory on a 2-hour mix.
- **`crates/ui/tests/analysis_playback.rs`**: real-time playback while two tracks are analyzed,
  with zero underruns.
- **`crates/visuals`** (unit tests, plus GPU tests on a headless device that skip when there is
  no GPU):
  - musical time and triggers locked to the analyzed beats, including pause and seek;
  - every shaper, route determinism, and the base → manual → macros → routes → clamp order;
  - manifests, and WGSL generation checked with naga, with errors mapped to the author's line;
  - every bundled scene compiling and rendering;
  - crossfade, feedback trails, compute accumulation, hue, and broken shaders not panicking;
  - variants loading when params change, mutation, lineage and undo;
  - the director's default show (rise, fall, repeat, idle, track change, provisional boundaries);
  - fader RETURN glides landing on bar lines;
  - the overlay fade;
  - hot reload keeping the last good scene, and the file watcher;
  - the engine end to end (keys, crossfades, re-init).

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
crates/analysis   music analysis: beat grid, tempo segments, sections, tension, cache, eval tools
crates/ui         the player window: skin, main/EQ/playlist sections, fullscreen host, playlist model
crates/visuals    the visual engine: signals, modulation, scenes (WGSL + RON), variants, director, GPU compositor, overlay, deck
crates/visuals/assets  the bundled scenes, variants, prelude and director rules
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
   the playhead ✅ (done and archived)
4. `visual-engine`: fractal scenes (WGSL), a modulation matrix, a director, a fader deck, and an
   auto-fading track overlay ✅
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
