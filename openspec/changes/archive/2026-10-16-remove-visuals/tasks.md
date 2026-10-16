## 1. Remove the visuals and the fullscreen host

- [x] 1.1 Delete `crates/visuals/` and remove it from the workspace members and `apps/native/Cargo.toml`; delete `apps/native/src/render.rs`, `--render-show` and its usage text; drop `scene: VisualEngine::new()` from `main.rs`; verify `cargo tree -p diggr` no longer lists `visuals`, `naga` as a direct dependency of ours, `notify` or `pollster`
- [x] 1.2 In `ui`, remove the scene seam (D2): `fullscreen.rs`, `VisualScene`, `SceneFrame`, `AppContext.scene`, the `fullscreen` state, `toggle_fullscreen`, `host_action`, cursor hiding and `DIGGR_AUTO_FULLSCREEN`; F and Esc-to-leave do nothing; update or delete the tests that entered fullscreen (`app.rs`, `dig_tests.rs`); verify with a headless test that F in the player window changes nothing
- [x] 1.3 Remove the debug strip and annotation mode (D4): `timeline.rs` (moving `kind_color` into `waveform.rs`), T and A, `annotating`, `AppContext.annotations_dir`; verify the waveform still tints sections (its existing tests)
- [x] 1.4 Help panel: drop the "Fullscreen visuals" group, F, T and A; verify with the help's existing tests that every listed key still works and F isn't listed

## 2. Trim the analysis (D3)

- [x] 2.1 Remove `events`, the bass, mid, treble, brightness, flatness and tension curves, `tension()` and `drop_in()` from `SongScore` and their assembly; keep onset detection for tempo; bump `ALGORITHM_VERSION` to 2; delete what the compiler then reports unused; verify the rhythm tests (kick within ±20 ms from the onset detector, tempo on the synth set), the structure tests and the cache test (a version-1 entry is ignored)
- [x] 2.2 Check `--bench --analysis` and `analysis-eval` still build and report beats, BPM and sections; run `analysis-eval` on the synth set once

## 3. The user's visuals folder (D5)

- [x] 3.1 After the first frame, on the background spawner, delete `<config>/Diggr/visuals/` (honouring `DIGGR_CONFIG_DIR`) when it exists; verify with a test using `TestDir`: a `visuals/` folder with files is gone after startup while `settings.ron` and a sibling file remain, and without one nothing is created

## 4. Docs, specs housekeeping and checks

- [x] 4.1 README: pitch and hero without the visualizer; remove the fullscreen, scenes, variants, director, fader deck and render sections, F/T/A keys, `DIGGR_AUTO_FULLSCREEN` and `--render-show`; the analysis described for BPM, waveform and structure keys; test areas, test count and layout updated
- [x] 4.2 AGENTS.md: "What this is", the workspace table without `visuals`, the data files line, hard rule 4 removed, hard rule 5 without the A/V target; `openspec/config.yaml`: the product line and the constraints that mention visuals
- [x] 4.3 Delete the empty change folders `openspec/changes/show-transitions`, `section-matching` and `fractal-library`
- [x] 4.4 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check; confirm `--startup-time` is no worse than before, and note the release binary's size before and after
- [x] 4.5 Run the app: F does nothing, the help has no fullscreen group, BPM column and filter, waveform, spectrogram, `[`/`]` and previews all work, and an existing `visuals/` config folder is gone after launch
