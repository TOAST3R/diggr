## Why

The signature feature is a fullscreen fractal visualizer that is locked to the rhythm and changes its world when the music's structure changes — and that the user can customize and evolve over time without rewriting code. MilkDrop reacted to loudness; this engine reads musical time, beat phase, sections, and upcoming drops.

## What Changes

- New `crates/visuals`: a data-driven visual engine rendered with wgpu/WGSL (fragment and compute), hosted by `classic-ui`'s fullscreen mode.
- **Signal bus**: typed musical signals (phases, triggers, continuous, discrete) and musical time, sourced from the playback clock + `SongScore`, with live-FFT texture and a live-only fallback.
- **Modulation matrix**: routes `signal → shaper chain → param` with shapers (envelope, smooth, curve, range, quantize, sample-hold, random, step sequencer, gate).
- **Scenes**: `scene.wgsl` + `scene.ron` manifest; engine-injected prelude (music uniforms, generated params, helper library, previous frame); hot reload with validation and safe fallback.
- **Variants**: saved parameter genomes per scene with lineage; mutate / keep / undo / rate from the keyboard.
- **Global macros**: intensity, chaos, stretch, speed (quantized), hue, feedback — mapped per scene.
- **Director**: RON rules file reacting to section changes and their energy (plus optional build/drop/breakdown kinds and repeated labels); cut / crossfade / morph transitions landing on downbeats. Section kinds proved heuristic on real music (see the archived `music-analysis` design), so the default show keys on boundaries, energy and tension.
- **Compositor**: scene → feedback warp → post FX → overlay; dynamic resolution to hold frame rate.
- **Fullscreen overlay**: artist, title, progress bar with section ticks, elapsed/total; auto-fades.
- **Fader deck** (fullscreen only): macros + scene params; AUTO vs MANUAL with a musical RETURN time (global + per-fader).
- Deterministic show per track (seed = track hash).
- Starter pack: Julia 2D, feedback liquid, KIFS 3D raymarch, compute fractal flame.

## Capabilities

### New Capabilities
- `signal-bus`: musical time and typed signals from clock, score, and live FFT.
- `modulation`: routes and shapers mapping signals to parameters.
- `scene-system`: scene format, prelude, parameter binding, hot reload, validation.
- `variants`: genomes, mutation, keep/undo, ratings, lineage.
- `director`: rules-driven scene selection and musically quantized transitions.
- `compositor`: layer pipeline, feedback, post FX, transitions, dynamic resolution, GPU-loss recovery.
- `fullscreen-overlay`: auto-fading track info and progress.
- `fader-deck`: live faders for macros and params with automation/return behavior.
- `starter-scenes`: the four bundled scenes proving 2D, feedback, 3D, and compute paths.

### Modified Capabilities
<!-- none -->

## Impact

- New crate `visuals` and asset folder `visuals/` (prelude, scenes, variants, director.ron).
- Dependencies: wgpu (the version eframe uses), naga (validation with line numbers), serde/ron, notify (native hot reload). Text uses egui's bundled font.
- Implements classic-ui's `ui::fullscreen::VisualScene` (wgpu paint callback + egui layer), receiving `SceneFrame { position, bars, artist, title, score }` each frame; the host keeps `T`/`A` (and annotation keys) for itself.
- Depends on `audio-core` (Position), `music-analysis` (`SongScore`), `classic-ui` (`VisualScene`, key routing).
- GPU requirement: WebGPU-class features (compute shaders); no WebGL2 fallback.
