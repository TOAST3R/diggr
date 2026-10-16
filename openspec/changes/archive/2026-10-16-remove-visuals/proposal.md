## Why

Diggr is for digging records: hearing a page's records fast, sorting what to want, pass or buy. The fullscreen visualizer (scenes, director, fader deck, video export) is a second product inside it. It brings its own crate, 12 specs, GPU tests, a hot-reloaded user folder, a hard rule and a latency target, and none of it helps anyone dig. Removing it keeps the code, the docs and the pitch on what the app is for.

## What Changes

- **BREAKING:** the fullscreen visuals are removed. F no longer opens anything, and there is no fullscreen mode, track overlay, fader deck, scene library, variants or director.
- **BREAKING:** `diggr --render-show` (exporting a track's show to video) is removed.
- **The visuals crate goes:** `crates/visuals` (signal bus, modulation, scenes in WGSL and RON, variants, director, compositor, overlay, fader deck) is deleted, with the dependencies only it uses (`naga`, `notify`, `pollster`).
- **The fullscreen host in the player goes:** `ui`'s scene seam (`VisualScene`, `SceneFrame`, `fullscreen.rs`), the analysis debug strip (T) and annotation mode (A), which only exist in fullscreen.
- **The analysis keeps only what the player reads:** BPM (tempo segments), beats and downbeats, sections and the energy curve, the track overview (waveform, lossy-source check) and the spectrogram detail. The score drops the drum events, the bass, mid, treble, brightness and flatness curves, and the tension and drop countdown, which only the visuals read. Drum onsets are still detected, because tempo estimation uses them. Cached scores are recomputed once.
- **The user's visuals folder is deleted:** `<config>/Diggr/visuals/` (hot-reloaded scenes and variants) is removed at launch when it exists.
- **Rules and targets:** AGENTS.md's hard rule 4 ("Visuals are deterministic") and the "A/V offset < 1 video frame" target are dropped. The README, AGENTS.md, `openspec/config.yaml` and the help panel describe a player for digging.
- **Stale changes:** the empty change folders `show-transitions`, `section-matching` and `fractal-library` (visuals ideas never written) are deleted.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `compositor`, `director`, `fader-deck`, `fullscreen-mode`, `fullscreen-overlay`, `modulation`, `scene-system`, `show-render`, `signal-bus`, `starter-scenes`, `variants`: every requirement removed.
- `analysis-debug`: the debug timeline strip and annotation capture are removed. The evaluation command stays.
- `structure-analysis`: "Tension and drop countdown" is removed.
- `rhythm-analysis`: band onsets feed tempo estimation and are no longer stored in the score.
- `streaming-analyzer`: lock-free reads are described for the player, not the visual engine.
- `structure-navigation`: section jumps work in the player window (no fullscreen).
- `player-window`: the shortcut list and the help panel lose F and fullscreen.
- `crate-search`: the "shortcuts don't fire" scenario no longer mentions fullscreen.
- `spectrogram-window`: "Low cost" no longer mentions fullscreen visuals.
- `macos-release`: the bundle holds the skin, and the app opens "with its skin" (no scenes or rules).
- `app-identity`: a leftover visuals folder in the config folder is deleted.

## Impact

- **Deleted:** `crates/visuals/` (sources, WGSL and RON assets, tests), `crates/ui/src/fullscreen.rs`, `crates/ui/src/timeline.rs` (its section colours move to the waveform), `apps/native/src/render.rs`.
- **`crates/ui`:** `AppContext.scene`, the fullscreen state and key routing, `DIGGR_AUTO_FULLSCREEN`, annotation mode, and the help entries.
- **`crates/analysis`:** `SongScore` loses `events`, the five curves and tension (`tension()`, `drop_in()`); `ALGORITHM_VERSION` is bumped; code left dead after that is removed. `analysis-eval` keeps reading annotation files.
- **`apps/native`:** the `visuals` dependency, `--render-show` and its usage line.
- **Workspace:** `Cargo.toml` members and `Cargo.lock`.
- **Docs:** README (pitch, keys, the visuals and render sections, test areas, test count, layout), AGENTS.md (what this is, workspace table, data files, hard rules, targets), `openspec/config.yaml`.
- **User data:** `<config>/Diggr/visuals/` deleted at launch. Settings files from older versions still load (unknown fields are ignored).
- **Other open changes:** `beatmatch-automix` and `web-target` mention the visuals in passing. They are left as they are and trimmed when picked up.
- **No change** to `audio`, `platform`, `dig`, the extension, or anything about digging.
