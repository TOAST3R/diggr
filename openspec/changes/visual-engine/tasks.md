## 1. Crate and frame loop

- [ ] 1.1 Create `crates/visuals`; implement `FullscreenHost` integration (device/queue/surface per vsync, egui layer)
- [ ] 1.2 Asset layout `visuals/{prelude,scenes,variants,director.ron}`; loader with serde/RON errors surfaced

## 2. Signal bus

- [ ] 2.1 Musical time from ClockReader + SongScore (beats, phases, beat_confidence)
- [ ] 2.2 Trigger scheduler: fire all score events crossed since last frame with sub-frame offsets
- [ ] 2.3 Continuous signals from score curves; live FFT bands from TapReader aligned to clock
- [ ] 2.4 Live-only fallback and crossfade into score mode on coverage
- [ ] 2.5 Track seed from content hash

## 3. Modulation

- [ ] 3.1 Route model and RON syntax; composition order (base → manual → macros → routes → clamp)
- [ ] 3.2 Shapers: envelope, smooth, curve, range, quantize, sample_hold, random, step_seq, gate
- [ ] 3.3 Unit tests for each shaper and determinism across runs

## 4. Scene system

- [ ] 4.1 Manifest schema; Params WGSL + uniform layout generation
- [ ] 4.2 Prelude library: Music struct, prev_frame, complex, noise/fbm, palettes, rot, SDF, raymarch
- [ ] 4.3 Fragment scene pipeline; compute scene pipeline (storage buffers + resolve pass)
- [ ] 4.4 Validation (naga + error scope) and atomic pipeline swap; error toast
- [ ] 4.5 Hot reload via notify (native) for WGSL, RON, prelude, director

## 5. Compositor

- [ ] 5.1 Offscreen Rgba16Float targets, ping-pong feedback with decay/warp functions
- [ ] 5.2 Transition mixer (cut, crossfade, morph via param interpolation)
- [ ] 5.3 Post FX: bloom, chromatic kick, vignette, grain
- [ ] 5.4 Dynamic resolution governor with crossfade pre-lowering
- [ ] 5.5 Device-loss recovery

## 6. Variants and evolution

- [ ] 6.1 Variant load/save with tolerant params; lineage
- [ ] 6.2 Mutate (M / Shift+M), keep (K), undo (Backspace), rate (1–5)

## 7. Director

- [ ] 7.1 Rules schema, events from SongScore, pick expressions
- [ ] 7.2 Downbeat-quantized scheduling; pre-scheduled drop cuts via drop_in; provisional safety
- [ ] 7.3 Label memory per track; default director.ron

## 8. Overlay and deck

- [ ] 8.1 Overlay: bundled OFL font, artist/title/progress with section ticks/time; auto-fade logic
- [ ] 8.2 Fader deck UI: macros + scene params, base vs live display, scene/variant/rating header
- [ ] 8.3 AUTO/MANUAL with RETURN (global + per-fader), bar-aligned glide back; speed quantization
- [ ] 8.4 Macro mapping per scene manifest

## 9. Starter scenes

- [ ] 9.1 julia_tunnel + 2 variants
- [ ] 9.2 liquid_feedback + 2 variants
- [ ] 9.3 kifs_cathedral + 2 variants
- [ ] 9.4 flame (compute) + 2 variants (one tagged calm)
- [ ] 9.5 Benchmark all four on M1-class hardware at 1440p; tune iteration caps
