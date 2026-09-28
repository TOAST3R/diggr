## Context

- **Scene kinds today:** `Fragment` and `Compute { invocations }` (`crates/visuals/src/manifest.rs`). A compute scene's `simulate` splats into an atomic accumulator that `Layer::encode` clears every frame (`crates/visuals/src/gpu.rs`), so nothing a scene computes outlives a frame.
- **`prev()`** samples the *composited* feedback target: both layers, the crossfade, and HDR `Rgba16Float` at the current render scale. It suits trails. It doesn't suit exact state: a crossfade blends another scene into it, and a render-scale change resamples it.
- **Per-instance resources** already live in `gpu::Layer` (its uniforms, its accumulator). Layer A and layer B each have their own, so two instances of the same scene never share state.
- **Musical time:** `Music.beats` comes from `signals.rs`, where it integrates at the tempo and phase-locks to the analyzed grid. It jumps when the error is greater than 2 beats (a seek or the first lock) and otherwise converges smoothly, so it can move backwards by a fraction of a beat.
- **Spectrum:** `Music.bands` holds 19 live bars (`ui/src/spectrum.rs`), and `band(m, i)` reads them.
- **Offline rendering** (`render.rs`) runs the same engine on a fixed timeline, so whatever this change adds has to work there too.

## Goals / Non-Goals

**Goals:**
- Scenes can hold a simulation grid that persists across frames, private to each layer instance.
- The simulation advances on musical time: the same number of steps at the same beats, at any frame rate or in offline render.
- Authors write only the rule (`fn rule`) and the look (`fn scene`). Bindings, ping-pong and dispatch are generated, as for other kinds.
- The `polar_life` scene: spectrum-seeded Generations automaton flowing outward through a log-polar tunnel.

**Non-Goals:**
- Pixel-identical output across frame rates. The injected spectrum is sampled live, so frames differ slightly. The same holds for `liquid_feedback` today.
- Cartesian or 3D automata, or grids larger than 1024 × 1024. The kind is general, but only polar is shipped and tuned.
- More spectrum resolution than the 19 bars.
- CPU-side simulation.

## Decisions

### D1. A new kind, `Automaton`, not an extension of `Compute`
```ron
kind: Automaton(theta: 128, rings: 64, steps_per_beat: 4.0),
```
- **Semantics differ from `Compute`:** `Compute` means a stateless splat every frame, while `Automaton` means a grid stepped on ticks. Mixing the two would make it unclear when state is cleared.
- **Validation:** `theta` and `rings` must be in 8..=1024, and `steps_per_beat` in (0, 16].
- **Size:** the grid is small (128 × 64 = 8 k cells, 64 KiB at `Rgba16Float`), so the cost is a few µs per step.
- **Alternative rejected:** CPU simulation in Rust. It is testable without a GPU, but the rule wouldn't be hot-reloadable WGSL, and each tick would need an upload.

### D2. State is a ping-pong pair of `Rgba16Float` storage textures owned by `gpu::Layer`
- **Cell format:** `vec4f` holds whatever the author wants. By convention in `polar_life`: `x` = alive (1) or dying (0..1 by age), `y` = age in ticks / 255, `z` = hue carried from its birth band, `w` spare.
- **Why `Rgba16Float`:** it is a core WebGPU format both as a readable texture and as a write-only storage texture, so the same code runs on the web target. It also holds fractional ages, which a uint format wouldn't.
- **Size:** fixed by the manifest, independent of render scale.
- **Lifecycle:** created lazily on the first encode and cleared to zero on creation and on reset (D4). When a hot reload changes `theta`/`rings`, they are reallocated.
- **Bindings:**
  - binding 4: `state_in: texture_2d<f32>` (read, `textureLoad`, no filtering).
  - binding 5: `state_out: texture_storage_2d<rgba16float, write>`.
  - binding 6: the automaton uniform (`tick`, `tick_phase`), see D3.
  - The fragment pass binds the *current* state as `state_in`. The storage binding is present only in the step pipeline's layout, so the render pipeline never writes state.

### D3. Ticks are counted in musical time, per layer, at a rate that can change per beat
The step rate is `steps_per_beat × rate`. `rate` is the value of an optional, reserved scene param `step_rate` (routable like any param, e.g. faster on drops), **sampled once per beat** and held for that beat:
```
at each beat boundary b crossed:   rate_b = snap(step_rate, rate_(b-1)) // {0.5, 1, 2, 4}, with hysteresis
                                   pos_b  = pos_(b-1) + steps_per_beat × rate_(b-1)
pos   = pos_b + fract(beats) × steps_per_beat × rate_b                  // f64 position in ticks
due   = floor(pos)
steps = clamp(due - last_tick, 0, MAX_CATCHUP)                          // MAX_CATCHUP = 8
run `steps` step passes (each: state_in ← cur, state_out ← other, swap), last_tick = due
```
- **Why per beat, and why powers of two:** a rate that changed continuously would make the step count depend on the frames that sampled it, which breaks frame-rate independence. Holding it per beat keeps the steps evenly spaced within each beat. The powers of two keep them on the beat grid (4 steps/beat are 16th notes; ×2 gives 32nds). Section kinds and the per-beat energy curve come from the analysis, so the choice for each beat is deterministic.
- **Several speed changes in one drop:** the rate can change on any beat, as often as the modulated value moves. `polar_life` routes `(source: Energy, shapers: [Gate([Drop]), Range(1.0, 4.0)], mode: Set, target: "step_rate")`. In a drop the speed follows the energy: ×2 for a normal drop, ×4 when it peaks, and back to ×2 when it eases. Outside drops the gate is closed and the rate stays at its default, ×1.
- **Snapping with hysteresis:** the snap works in octaves: `level = log2(rate)` in {−1, 0, 1, 2}. It moves to the level nearest `log2(step_rate)` only when that is more than 0.6 from the current level (0.1 past the midpoint). Energy hovering at a boundary then doesn't flip the speed on every beat. A move can cross several levels at once (×1 → ×4 on a massive drop). With `Range(1, 4)` on normalized energy (0..1):
  - ×1 → ×2 above energy 0.17;
  - ×2 → ×4 above 0.84, and ×4 → ×2 below 0.55.
- **Without `step_rate`:** a scene that doesn't declare it runs at rate 1, which is `floor(beats × steps_per_beat)` as before.
- **Where:** a pure `AutomatonClock` in the engine (Rust), per layer. It hands `Layer::encode` a step count, the tick index and `tick_phase`, so it can be tested without a GPU.
- **Uniform:** a small per-layer automaton uniform carries `tick` (for each pass) and `tick_phase = fract(pos)` (for the fragment pass). A rule can seed deterministically with `hash(tick, cell, seed)`.
- **Small backwards moves:** phase-lock corrections of less than one tick give `due < last_tick`, so the step count is 0 and the automaton holds. It never runs backwards. A beat boundary crossed backwards doesn't resample the rate.
- **Paused:** `beats` is still, so there are no steps.
- **Glide:** `fn scene` calls the generated `tick_phase()` and shifts its radial lookup by it, so the discrete ring shift reads as continuous motion. When the rate doubles, the glide speeds up with it.

### D4. Reset and pre-roll on discontinuities
- **Triggers:** the layer's state resets (cleared, `last_tick = due - PREROLL`) when:
  - `due` jumps by more than `max(2 beats worth of ticks, MAX_CATCHUP)` in either direction (a seek or the first lock), mirroring the 2-beat rule in `signals.rs`;
  - the track changes (the seed changes);
  - the program is rebuilt (hot reload), or the layer is created, e.g. when the director loads the scene.
- **Pre-roll:** `PREROLL = 16` ticks, spread over the next frames at the `MAX_CATCHUP` rate, so a freshly loaded scene isn't black but no frame gets a burst of work. On the first frames after a reset, the effective catch-up budget is 8 steps per frame until the pre-roll is consumed.
- **Offline render:** `render.rs` gets this for free, because it drives the same engine with its own `beats`.

### D5. Generated WGSL API for authors
```wgsl
// author implements:
fn rule(c: vec2<i32>, m: Music, p: Params) -> vec4f   // c = (theta, ring); return next state. Not `step`: that would shadow the WGSL builtin.
fn scene(uv: vec2f, m: Music, p: Params) -> vec4f

// generated helpers:
fn grid() -> vec2<i32>                  // (theta, rings)
fn cell(c: vec2<i32>) -> vec4f          // previous state; theta wraps (rem_euclid); rings outside the grid → vec4f(0)
fn tick() -> i32                        // index of the tick being computed (step only)
fn tick_phase() -> f32                  // fraction of the current step elapsed (from the automaton uniform)
fn inject_level(m: Music, theta: f32) -> f32   // theta in 0..1 → 19 bands, mirror symmetric, linear interp
fn state_at(depth: f32, theta: f32) -> vec4f   // depth in rings (float), theta 0..1; nearest in theta,
                                               // linear in depth, 0 outside the grid
```
- **Mirror symmetry:** `theta` 0..0.5 maps bands 0..18, and 0.5..1 mirrors them. Bass sits at the bottom (θ = 0.75 turn after rotation in `scene`), which makes the tunnel read balanced.
- **Shift:** the ring shift is part of the rule the author writes (`cell(c - vec2(0, 1))` is the ring below). The engine doesn't enforce it, so a scene can collapse inward if it wants.

### D6. `polar_life` rule and look (revised after the first visual check)
**First attempt, rejected on screen:** each step pushed the grid one ring outward, and a Generations rule saw only the 5 upstream neighbours. Cells never got to interact, so the result was advection (arcs and streaks sliding out) rather than life. A fixed spectrum threshold, tuned against a louder synthetic spectrum, also left real tracks mostly black. Offline renders of the user's track (`--render-show … --look polar_life/bloom`) confirmed it: black for most of 10 s, with smooth arcs near the centre.

**Revised dynamics:**
- **Life-like evolution every step** over the 8 neighbours of the cell. Birth and survive masks go over 0..=8, and the default is B3/S23 (Conway's Life). The calm variant `drift` uses the default. The intense variant `bloom` uses B3/S2345 (a maze-like, dense rule).
- **Rhythmic push:** every `shift_every` steps, the cell is taken from the ring below and the rule is evaluated around that source. That makes it Life in a frame that moves with the tunnel, so patterns stay intact at any push speed. The first tuning (4 steps per beat, a push every 4 steps, so one ring per beat) felt far too slow against the music. The defaults are now 8 steps per beat and a push every 2 steps: 4 rings per beat, crossing the tunnel in about 16 beats. Kicks add a lunge (`kick_punch` rings × `pulse(m.kick, 8)`) to the drawn depth.
- **Onset injection on ring 0:** each ring-0 cell keeps its angle's running spectrum level in `w` (an exponential average, 0.15 per step). A cell is born when `inject_level` rises `sensitivity` above that average. Loud and quiet passages both feed the tunnel, independently of absolute bar levels. Kicks scatter seeds with probability `kick_seed × pulse(m.kick, 6)`.
- **Soup start:** on `since_reset() == 0` (a new engine helper), the grid is seeded with a random soup (`soup`, 35% by default), and a 64-step pre-roll (`preroll: 64` in the manifest, a new optional field) evolves it into patterns. The tunnel is full on entry. Without this, one ring per beat takes about 30 s to fill 64 rings, and the director's resets made the scene look black.
- **Turnover:** cells older than `max_age` steps (240) die, so structures renew. The GPU test measures silence emptying the grid completely: 0 cells after `rings × shift_every + max_age` steps.

**Look:**
- Each cell is drawn as a tile with a thin gap (nearest cell, no interpolation), so the grid of the automaton reads clearly.
- Glide: sampling at `depth + 1 − (since_push + tick_phase) / shift_every` slides the tunnel continuously between pushes.
- Log-polar depth from `r_min` (0.03) to `r_max` (2.0, which covers widescreen corners). A faint lattice every 8 rings and angles keeps the tunnel visible when little lives. The centre is darkened and the rim fades out.
- Feedback is light (0.3), so the tiles stay crisp.

**Hits and interference:** a *hit* is a kick or, with `beat_hits` (on by default), a beat as well while `m.energy` is above 0.15, since the beat grid keeps ticking through silence. On the user's track the analyzer found few kicks (one between 1:00 and 1:04) although a kick is heard on every beat. The beat grid is dependable, so it backs the kick up. Each hit:
- makes the tunnel lunge (`kick_punch`) and scatters seeds on ring 0 (`kick_seed`);
- flips cells at random in a band of 2–7 rings of the grid (`kick_noise`), chosen per hit, so the noise evolves and flies outward;
- adds interference to the picture for about 0.2 beats (`glitch`): screen rows jump sideways, bands of rings tear, the colour channels split along the angle, scanlines dim and static flickers.

A hit is identified by its time in eighths of a beat (`hit_id`), so every step and pixel makes the same random choices for it.

**Routes:** Kick → `glow`, Treble → `twist`, and Energy gated to drops → `step_rate` 1..4 (a normal drop runs ×2, a peaking drop ×4, changing on the beat). Optional routes: a phrase-random spin, and Hat → palette shift.

**Macros:** Chaos → `kick_seed`, Stretch → `twist`, Feedback → decay states. The B/S masks are bit patterns, and a macro sweeping them would jump between unrelated rules, so variants choose them instead.

**Tags:** `["2d", "tunnel", "organic", "automaton"]`.

### D7. A richer look, rules per section, and a continuous automaton
After the rework, the tunnel moved well but read as flat pixels: small patterns on a coarse grid, lit on or off.

- **Cells as objects:** each tile has a bevelled rim whose slope tilts its normal, lit from the vanishing point, with a specular top on living cells. Living cells bleed a halo into the gaps, and the two cells further out drag tails back towards the centre, so motion reads as speed. The first version read 13 cells per pixel (a 3 × 3 halo, 3 tails) and tripled them for the colour split: 42 ms per frame at native Retina resolution. It now reads 6: the halo only from the three neighbours on the pixel's side of its tile (farther ones add under 1%), 2 tails, and the split channels skip halo and tails. That's 12.9 ms.
- **The song's structure changes the life, not just its speed:** with `section_rules`, builds run Brian's Brain (restless sparks), drops Star Wars (exploding lattices) and breakdowns Day & Night (calm blobs with long trails). Coral (B3/S45678) was tried for breakdowns and rejected: it kills every cell with fewer than 4 neighbours, so a breakdown blanked the tunnel. The rest of the song runs the variant's rule. The section comes from the analysis, which the Music uniform already carries, so the choice is deterministic. It is sampled from the frame that runs the step, the same trade-off as the spectrum.
- **Finer grid:** 160 × 96 keeps the cells square in log-polar (2π/160 ≈ ln(r_max/r_min)/96). 256 × 160 was tried first, but its tiles were about 4 px at 720p, too small for the bevel and halo to read. The shear from twist grows with the ring count, so the treble twist route and the twist range were scaled down with it. One push per step at 8 steps per beat crosses the tunnel in 12 beats. The pre-roll is 32 steps: with a push every step, a pre-roll as long as the tunnel (tried first) pushes the whole soup out before the first frame.
- **The 3D tube (`prelude/tunnel.wgsl`):** a tube of radius 1 seen from its axis. Its far end sways to `bend`, and its axis on screen at depth z is `bend · z / (z + 2)`. `tube_hit` solves for the wall point with three fixed-point iterations. With `bend` = 0 it is exactly the flat mapping, so `tube` blends the sway, a headlight (`tube_facing`) and distance fog into the same log-polar lookup. No raymarching is needed.
- **Continuous automata need many passes per tick:** `substeps` (1..=32) runs `fn rule` that many times per step, all with the same `tick()`. Tick accounting, catch-up and reset stay on musical time, which Gray-Scott chemistry needs because it moves in small increments. `coral_tunnel` runs 12 passes at 8 steps per beat, so at 120 BPM 192 passes per second. A 256-step pre-roll grows the seeds into coral before the first frame shows.
- **Why reaction-diffusion rather than Lenia:** Gray-Scott has well-known stable settings (the Karl Sims 3×3 Laplacian, dt = 1), a small kernel, and four named regimes (coral, cells, worms, mazes) that variants can pick. Lenia needs a large kernel and careful tuning per creature.
- **`Rgba16Float` precision:** substrate A rests near 1, where half floats step by about 5 × 10⁻⁴, and the feed increments there are smaller. In the mitosis regime that starved every spot. The grid therefore stores the depletion 1 − A, which rests near 0 where half floats are fine. Values are clamped to 0..1 on every pass.
- **Boundaries and seeding:** the flow carries fresh substrate in at the centre. The outer edge is closed (it mirrors the last ring): an open edge fed the last rings and grew a bright static ring there. Onsets, kicks and a trickle of spores (scaled with energy) seed growth in blocks, because single-cell seeds dissolve before the coral regime takes hold. Without spores, quiet passages let the flow flush the tunnel empty.

## Risks / Trade-offs

- **[Live spectrum sampling makes runs differ slightly across frame rates]** → Tick timing is exact, and injection is sampled from the frame that processes the tick. Accepted: the show's structure is determined by musical time, and the texture varies subtly. Tests assert tick counts and timing, not pixels.
- **[Chaotic rules blow up (the grid fills) or die out]** → The defaults were checked on offline renders of a real track. The push flushes old cells off the rim, `max_age` renews structures, and onsets keep re-seeding. A glider moving inward at exactly the push speed could in theory circle forever. `max_age` doesn't stop it, because its cells are reborn. The silence test found none with the defaults.
- **[Aliasing at the centre (many θ per pixel) and moiré at the rim]** → The centre is darkened, and `max_scale` isn't needed because the fragment is cheap. Nearest-in-θ sampling is intentional (visible cells). A later variant could smooth it.
- **[Storage texture format support on web]** → `rgba16float` write storage is core WebGPU. The same compute path as `flame` is required on Chrome.
- **[The energy that drives `step_rate` blends the analyzed per-beat curve with the live spectrum]** → Once a score covers the playhead, the analyzed curve dominates, so the per-beat speed is effectively deterministic, and the hysteresis absorbs the small live differences. Before the score arrives, drops aren't known and the gate stays closed (×1).
- **[Hot reload resets the show]** → Accepted. It's a rebuilt program, and the pre-roll hides it within a few frames.

## Migration Plan

This is additive: existing scenes, manifests and variants are unchanged. The director only picks `polar_life` through tags. Rolling back means removing the scene folder, and the kind stays harmless.

## Open Questions

- Resolved: the speed is modulated through the reserved `step_rate` param (D3). It is sampled per beat (confirmed: beat, not bar), snapped to {0.5, 1, 2, 4} with hysteresis, and can change several times within a drop following its energy.
- The 0.6-octave hysteresis and the `Range(1, 4)` mapping are starting points, to tune by eye on real tracks.
