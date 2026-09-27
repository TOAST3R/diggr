# AGENTS.md

Guidance for AI coding agents (and humans) changing this repository. The user-facing tour
is in [`README.md`](README.md). The authoritative product context is in `openspec/config.yaml`.

## What this is

A Winamp 2.x–style music player in Rust, with a fullscreen fractal visualizer driven by
analysis of the music's beats and sections. **Zero perceived latency is the top priority**, and
nothing on the playback path may ever wait for the UI, the analysis or the visuals.

Milestones (OpenSpec changes): `audio-core`, `classic-ui`, `music-analysis` and `visual-engine`
are done and archived. `web-target` is proposed but not started (`openspec/changes/web-target/`).

## Workspace

Cargo workspace, edition 2024, **Rust ≥ 1.95** (egui 0.36); built and tested with 1.98.

| Crate | Package | Role | Depends on |
|---|---|---|---|
| `crates/platform` | `platform` | seam traits `AudioSink`, `Spawner`, `FileSource`; native impls (cpal); `testing::ManualSink` | — |
| `crates/audio` | `audio` | decode (symphonia) → rubato → lock-free ring → real-time renderer (EQ, tap, volume) → device; playback clock; `Engine` API | platform |
| `crates/analysis` | `analysis` | streaming analyzer: onsets, tempo/beat grid, sections, tension → immutable `SongScore` snapshots; cache; eval tools; track overview (waveform + spectral + cutoff) and spectrogram detail worker | audio, platform |
| `crates/ui` | `ui` | egui/eframe (wgpu) player: skin, main/EQ/playlist, waveform, spectrogram window, settings, fullscreen host | audio, analysis, platform |
| `crates/visuals` | `visuals` | signal bus, modulation, scenes (WGSL + RON), variants, director, GPU compositor, overlay, fader deck | audio, analysis, ui |
| `apps/native` | `winamp-native` | desktop binary: GUI (default), `--tui`, `--bench [--analysis]`, `--click-test`, `--startup-time` | all |

Data files: the skin is in `assets/skin/default/`. It is generated, so don't hand-edit it; run
`cargo run -p ui --bin skin-gen`, which a test checks. Bundled scenes, variants, prelude and
director rules are in `crates/visuals/assets/`.

## Commands

Linux needs `build-essential pkg-config libasound2-dev`. macOS needs the Xcode CLT.

```sh
cargo test --workspace                                   # ~266 tests; no audio device, display or GPU needed
cargo clippy --workspace --all-targets -- -D warnings    # must be clean
cargo fmt --all --check                                  # must be clean
cargo check -p audio -p platform --target wasm32-unknown-unknown   # core must stay web-portable
```

Narrower runs: `cargo test -p audio --test engine`, `cargo test -p visuals director`.
Long checks are opt-in: `cargo test -p analysis --release -- --ignored timing` and
`-- --ignored two_hour`. GPU tests in `visuals` skip themselves when no adapter is available.

Run all four checks before committing. Dependencies are built with `opt-level = 3` even in
dev (see root `Cargo.toml`), because unoptimized decoders and DSP can't keep up with real time.

## Hard rules

1. **Audio callback is real-time safe.** No allocation, locks, syscalls, logging, or blocking
   in `crates/audio/src/renderer.rs` or anything it calls. `tests/rt_alloc.rs` enforces "no
   allocation" with a counting allocator, and `rt_guard` flags it in debug builds. Cross threads
   with atomics, `rtrb` SPSC rings, the seqlock clock or `TryCell`, never with `Mutex` on the
   audio side.
2. **Nothing waits on the playback path.** Analysis runs at low priority, ahead of the playhead.
   The UI and visuals read the clock and tap without locking. Seeks flush by generation number.
3. **Platform seams only.** `audio` and `platform` must compile for `wasm32-unknown-unknown`.
   Reach devices, threads and files only through the `platform` traits. Put native-only code
   behind `#[cfg(not(target_arch = "wasm32"))]`.
4. **Visuals are deterministic.** All randomness is seeded from the track hash and musical
   time, so the same track always gives the same show. Motion keys off musical time (beats,
   bars) from the playback clock, never off wall-clock frame counts.
5. **Latency targets are tested, so don't regress them.** Start < 30 ms, seek < 50 ms, 0
   underruns, A/V offset < 1 frame, launch < 300 ms. The measurements are
   `winamp-native --bench`, `--click-test` and `--startup-time` (see README).
6. **Tests stay hardware-free.** Use `platform::testing::ManualSink` (fake time) for engine
   tests, and the synthesized tracks in `analysis::synth` for analysis ground truth. Fixtures are
   in `crates/audio/tests/fixtures/`.

## Conventions

- Match the surrounding style: short `//!` module docs that explain *why*, and focused unit tests
  in the same file (`#[cfg(test)] mod tests`). Integration tests go in `crates/*/tests/`.
- Data files are RON (serde). User-editable visuals live in `<config>/winamp_rust/visuals/`, are
  hot-reloaded with `notify`, and must fail soft: keep the last good version and show the error.
- Paths come from `dirs` (config/cache), overridable by `WINAMP_CONFIG_DIR` / `WINAMP_CACHE_DIR`.
  Tests must use temp dirs, never the user's real folders.
- eframe/egui are used with `default-features = false`. On Linux, `crates/ui/Cargo.toml` enables
  the `x11` and `wayland` features, without which winit doesn't compile. Keep that when touching
  UI dependencies.
- Don't add model or tool identifiers to code, comments or commit messages.

## Planning with OpenSpec

Non-trivial features go through OpenSpec (`openspec/`): a change folder has `proposal.md`,
`design.md`, `specs/` and `tasks.md`. Requirements use SHALL/MUST, and each has WHEN/THEN
scenarios with numeric targets where they apply. Tasks should be verifiable in one session.

- Active work: `openspec/changes/<name>/`. Living specs: `openspec/specs/<capability>/`.
  History: `openspec/changes/archive/`.
- Agent commands are in `.claude/` and `.opencode/`: `/opsx:propose`, `/opsx:explore`,
  `/opsx:apply` and `/opsx:archive`.
- When implementing a change, tick its `tasks.md` boxes as you go. Before changing behavior
  that an existing spec in `openspec/specs/` describes, read that spec. Each archived change's
  `design.md` records why the code works the way it does.

## Keeping docs honest

After a change that adds a key, flag, env var, file location or test area, update
`README.md` (and the test count in its Tests section) in the same commit.
