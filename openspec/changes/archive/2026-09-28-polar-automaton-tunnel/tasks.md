## 1. Manifest and tick bookkeeping

- [x] 1.1 `SceneKind::Automaton { theta, rings, steps_per_beat }` in `manifest.rs`, with validation (8..=1024, 0 < steps ≤ 16) and errors that name the field; parse/reject unit tests
- [x] 1.2 A pure tick clock (`AutomatonClock`): a position in ticks that advances `k × rate` per beat, with the rate taken from the optional `step_rate` param once per beat and snapped to {0.5, 1, 2, 4} with octave hysteresis (0.6), multi-level jumps allowed; at most 8 steps per frame with carry-over, holds on small backwards moves, resets on jumps > 2 beats, track change or rebuild, and a 16-step pre-roll
- [x] 1.3 Clock unit tests: 30 vs 144 fps over 8 beats at 120 BPM → 32 steps at the same beats; groove→drop with rate 2 → 16 + 32 steps at both frame rates; a mid-beat rate change applies from the next beat; 2→4→2 within a drop gives 8/16/8 steps per beat; 1.35↔1.5 alternation doesn't flicker; paused; −0.1 beat correction; seek → reset + pre-roll over 2 frames

## 2. Codegen

- [x] 2.1 Automaton bindings: `state_in` (texture_2d), `state_out` (storage rgba16float, step layout only), and the automaton uniform (`tick`, `tick_phase`)
- [x] 2.2 Generated `cs_step` entry point (8×8 workgroups over the grid) calling the author's `fn rule(c, m, p)`
- [x] 2.3 Helpers: `grid`, `cell` (θ wraps, rings outside the grid are empty), `tick`, `tick_phase`, `inject_level` (19 bars → θ, mirror, linear), `state_at` (nearest θ, linear depth)
- [x] 2.4 Codegen tests: an automaton module validates with naga; a missing `fn rule` points at the author's file and line; fragment and compute scenes are unchanged

## 3. GPU

- [x] 3.1 The `SceneProgram` for automata: a step pipeline plus a render pipeline with separate layouts
- [x] 3.2 `Layer` owns the ping-pong state textures (lazy, reallocated on a grid-size change, cleared on reset), runs N step passes before the fragment pass, and swaps
- [x] 3.3 Wire `AutomatonClock` per layer in `engine.rs` (step count and reset signals from the seed, program rebuild and beats jumps) and in the offline path of `render.rs`
- [x] 3.4 GPU tests (skip without an adapter): state persists across frames; a copy-neighbour rule moves a cell one ring per step and wraps θ; a reset clears; the two layers keep separate state; a render-scale change leaves the grid intact

## 4. polar_life scene

- [x] 4.1 `assets/scenes/polar_life/scene.ron`:
  - `Automaton(theta: 128, rings: 64, steps_per_beat: 4.0)`;
  - params (`threshold`, `kick_seed`, `birth`/`survive` masks, `decay_states`, `twist`, `spin`, `r_min`/`r_max`, colours);
  - `step_rate` param: Energy → `Gate([Drop])` → `Range(1, 4)`, `mode: Set` (×2 for a normal drop, ×4 at the peak);
  - routes (Kick → kick_seed, Energy → threshold, Treble → twist) and macros;
  - tags `2d`, `tunnel`, `organic`, `automaton`.
- [x] 4.2 `scene.wgsl` `fn rule`:
  - ring 0: spectrum and kick injection, with the hue taken from the band;
  - rings ≥ 1: shift outward plus a Generations rule on the 5 upstream neighbours of the shifted cell (see design D6).
- [x] 4.3 `scene.wgsl` `fn scene`: log-polar depth shifted by `tick_phase`, θ twist and spin, colour by age and birth band, glow on alive cells, a darkened centre
- [x] 4.4 At least two variants in `assets/variants/polar_life/` (one tagged `calm`, one intense), with director tags checked in `director.ron`
- [x] 4.5 Scene tests (GPU, skip without an adapter), driven by a synthetic Music uniform (a kick every beat, then silence) rather than a full `analysis::synth` track: kicks seed ring 0 and the cells reach the outer rings; silence empties the grid after rings + decay steps

## 5. Verify and document

- [x] 5.1 Performance: `polar_life` holds render scale ≥ 0.75 in the visual benchmark (15 s fullscreen), with step passes < 0.2 ms at 128×64
- [x] 5.2 Visual check: `cargo run -p diggr`, fullscreen, force `polar_life` from the deck; check the smooth glide at 144 fps, the reaction to kicks, the speed-up in drops (and its escalation on a strong drop), seek and hot reload
- [x] 5.3 README: the automaton scene kind and helpers, the bundled scene list, the test count
- [x] 5.4 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, and the wasm check for audio/platform, all clean

## 6. Rework after the first visual check

- [x] 6.1 Reproduce the report (mostly black, no visible automaton) with an offline render of the user's track, and read the frames
- [x] 6.2 `polar_life` rule: Life-like over 8 neighbours every step, push one ring every `shift_every` steps, onset injection with a per-angle running level on ring 0, `max_age` turnover
- [x] 6.3 Engine: optional `preroll` in the `Automaton` manifest (≤ 1024), and the `since_reset()` helper (steps since the last reset, in the automaton uniform)
- [x] 6.4 `polar_life` look: tiles with gaps, glide between pushes, faint lattice, `r_max` covering widescreen corners; soup start with a 64-step pre-roll
- [x] 6.5 Variants retuned (`drift` = Life, `bloom` = B3/S2345), tests updated (soup density, kicks reaching the rim at one ring per beat, silence emptying the grid, a long pre-roll spread over frames)
- [x] 6.6 Speed: 8 steps per beat and a push every 2 steps (4 rings per beat), and a kick lunge
- [x] 6.7 Interference on hits (a kick or, with `beat_hits`, every beat): row jumps, torn ring bands, colour split, scanlines and static, plus flipped cells in a random band of rings

## 7. Richer look and a second automaton (see design D7)

- [x] 7.1 `polar_life` look: bevelled tiles lit from the vanishing point with a specular top (`bevel`), a halo from living neighbours into the gaps (`halo`), comet tails behind outward-flying cells (`tails`); colour split only while glitching
- [x] 7.2 `polar_life` rules per section (`section_rules`): build → Brian's Brain (B2/S/3), drop → Star Wars (B2/S345/4), breakdown → Day & Night (B3678/S34678, 8 states); other sections run the variant's rule
- [x] 7.3 Finer grid: 160 × 96 (square cells in log-polar), 8 rings per beat (a push every step), 32-step pre-roll (the soup moves a third of the way out); kick band and punch scaled to the grid; the treble twist route and twist range scaled down; tests updated
- [x] 7.4 Engine: optional `substeps` in the `Automaton` kind (1..=32, `fn rule` passes per step, same tick), the `substeps()` helper; manifest and codegen tests
- [x] 7.5 `prelude/tunnel.wgsl`: `tube_hit` (a tube whose far end sways, solved in 3 fixed-point iterations) and `tube_facing`
- [x] 7.6 3D tube in `polar_life` (`tube`, `sway`: the far end sways over 8 bars, headlight and fog) and the `hyperdrive` variant
- [x] 7.7 New scene `coral_tunnel`: Gray-Scott reaction-diffusion (12 passes per step, the substrate stored as its depletion 1 − A for half-float precision, upwind outward flow, fresh substrate at the centre and a closed outer edge), onset, kick and spore seeding in blocks, glossy relief shading, tube option; variants `reef` (calm) and `mitosis` (tube)
- [x] 7.8 A GPU test for `coral_tunnel`: a kick splash grows and the chemistry stays within 0..1 over 400 passes
- [x] 7.9 Verify: all four checks clean (303 tests); offline renders of each look (`--look polar_life/{drift,bloom,hyperdrive}`, `coral_tunnel/{reef,mitosis}`) read frame by frame; the visual benchmark (M2, 3024 × 1964): `polar_life` 12.9 ms at full scale, 7.8 ms at 0.75; `coral_tunnel` 12.3 ms, 9.3 ms at 0.75
- [x] 7.10 README: the new look params, `substeps`, the tunnel prelude, six bundled scenes, the test count
