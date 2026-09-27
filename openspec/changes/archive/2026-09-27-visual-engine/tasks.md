## 1. Crate and frame loop

- [x] 1.1 Create `crates/visuals`; implement `ui::fullscreen::VisualScene` (init with eframe's render state, paint callback with GPU work in prepare/paint, egui layer, keys)
- [x] 1.2 Asset layout `visuals/{prelude,scenes,variants,director.ron}`, bundled and copied to the config dir for editing; loader with serde/RON errors surfaced

## 2. Signal bus

- [x] 2.1 Musical time from ClockReader + SongScore (beats, phases, beat_confidence)
- [x] 2.2 Trigger scheduler: fire all score events crossed since last frame with sub-frame offsets
- [x] 2.3 Continuous signals from score curves; live FFT bands from TapReader aligned to clock
- [x] 2.4 Live-only fallback and crossfade into score mode on coverage
- [x] 2.5 Track seed from content hash

## 3. Modulation

- [x] 3.1 Route model and RON syntax; composition order (base → manual → macros → routes → clamp)
- [x] 3.2 Shapers: envelope, smooth, curve, range, quantize, sample_hold, random, step_seq, gate
- [x] 3.3 Unit tests for each shaper and determinism across runs

## 4. Scene system

- [x] 4.1 Manifest schema; Params WGSL + uniform layout generation
- [x] 4.2 Prelude library: Music struct, prev_frame, complex, noise/fbm, palettes, rot, SDF, raymarch
- [x] 4.3 Fragment scene pipeline; compute scene pipeline (storage buffers + resolve pass)
- [x] 4.4 Validation (naga + error scope) and atomic pipeline swap; error toast
- [x] 4.5 Hot reload via notify (native) for WGSL, RON, prelude, director

## 5. Compositor

- [x] 5.1 Offscreen Rgba16Float targets, ping-pong feedback with decay/warp functions
- [x] 5.2 Transition mixer (cut, crossfade, morph via param interpolation)
- [x] 5.3 Post FX: bloom, chromatic kick, vignette, grain
- [x] 5.4 Dynamic resolution governor with crossfade pre-lowering
- [x] 5.5 Re-create all GPU resources when `init` is called with a new render state

## 6. Variants and evolution

- [x] 6.1 Variant load/save with tolerant params; lineage
- [x] 6.2 Mutate (M / Shift+M), keep (K), undo (Backspace), rate (1–5)

## 7. Director

- [x] 7.1 Rules schema; events from SongScore boundaries (rise/fall by energy, change, repeat, optional kind), track change, idle; pick expressions
- [x] 7.2 Downbeat-quantized scheduling; boundaries pre-scheduled from the score; provisional safety (final or stable ≥ 8 beats)
- [x] 7.3 Label memory per track; default director.ron keyed on changes and energy

## 8. Overlay and deck

- [x] 8.1 Overlay: artist/title/progress with section ticks/time (egui font), above the analysis strip when shown; auto-fade logic
- [x] 8.2 Fader deck UI: macros + scene params, base vs live display, scene/variant/rating header
- [x] 8.3 AUTO/MANUAL with RETURN (global + per-fader), bar-aligned glide back; speed quantization
- [x] 8.4 Macro mapping per scene manifest

## 9. Starter scenes

- [x] 9.1 julia_tunnel + 2 variants
- [x] 9.2 liquid_feedback + 2 variants
- [x] 9.3 kifs_cathedral + 2 variants
- [x] 9.4 flame (compute) + 2 variants (one tagged calm)
- [x] 9.5 Visual benchmark mode (each scene 15 s fullscreen, frame times + render scale); run on this Mac; tune iteration caps
