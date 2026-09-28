## Why

Every current scene is either a pure function of the frame (`julia_tunnel`, `kifs_cathedral`), feeds back the composited image (`liquid_feedback`), or accumulates points that are cleared every frame (`flame`). None of them can hold a *simulation* that lives across frames, so organic, emergent visuals are out of reach. A cellular automaton on a polar grid does need that. The music injects life at the centre of a tunnel, and the living cells spread, collapse and stream outward on the beat. It would be the first scene whose shape comes from rules and history rather than a formula.

## What Changes

- **New scene kind `Automaton`:** the manifest declares a polar grid (`theta` × `rings`) and `steps_per_beat`. The engine keeps a private, persistent state grid for the scene (ping-pong on the GPU). Crossfades, render scale, post FX and the other layer never touch it.
- **Authored step function:**
  - The author writes `fn rule(cell, m, p) -> vec4f`, which computes a cell's next state from the neighbours in the previous state.
  - `θ` wraps around; rings outside the grid read as empty.
  - Helpers read neighbours, map the 19 spectrum bands onto angles (mirror symmetric), and let `fn scene` sample the grid with a sub-tick glide.
- **Steps on musical ticks:**
  - The automaton advances `steps_per_beat` steps per beat, not one per frame, so the show is the same at any frame rate.
  - The speed can be modulated with a reserved `step_rate` param. It is sampled once per beat and snapped to ×0.5, ×1, ×2 or ×4 with hysteresis. Drops can run faster, and change speed several times as their energy rises and falls, without breaking determinism.
  - Missed ticks catch up, up to a cap per frame.
  - A seek, a backwards jump, a track change or a scene reload resets the grid and pre-rolls it.
- **New bundled scene `polar_life`:**
  - Spectrum energy births cells on the inner ring, and kicks seed whole rings.
  - A Generations-style rule (alive → dying for k steps → dead) evolves the grid, and the state shifts one ring outward per tick.
  - It's drawn as a log-polar tunnel with the vanishing point at the centre, coloured by cell age.
  - It speeds up in drops following their energy: ×2 for a normal drop and ×4 at the peak of a strong one, changing on the beat.
  - It ships with at least two variants and tags for the director.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `scene-system`: the manifest gains the `automaton` kind. New requirements cover persistent per-scene state, tick-based stepping (determinism, catch-up, reset on discontinuities) and the automaton helpers in the prelude.
- `starter-scenes`: the bundled set grows from four to five with `polar_life`, which is held to the same variant and performance requirements.

## Impact

- `crates/visuals/src/manifest.rs`: `SceneKind::Automaton { theta, rings, steps_per_beat }` and its validation.
- `crates/visuals/src/codegen.rs`: state bindings, the `cs_step` entry point, and the `cell` / `state_at` / `inject_level` helpers.
- `crates/visuals/src/gpu.rs`: allocating the state textures, the step dispatches before the fragment pass, and the swap.
- `crates/visuals/src/engine.rs` / `render.rs`: the tick bookkeeping per layer and the reset on seek, track change or reload.
- `crates/visuals/assets/scenes/polar_life/`, `assets/variants/polar_life/`, `assets/director.ron` (tags).
- `README.md`: the scene kind, the bundled scene list and the test count.
- No new dependencies. Nothing is added to the audio path, and the web path (WebGPU compute) works as it does for `flame`.
