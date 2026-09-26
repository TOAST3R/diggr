## Context

Hosted inside `classic-ui`'s fullscreen mode (wgpu device/queue/surface per vsync, egui layer on top). Inputs: `ClockReader` (audible position), `TapReader` (post-EQ samples), `SongScore` snapshots (beats, events, sections, tension, drop_in). Targets: native (Metal on macOS) and Chrome WebGPU — both support compute. The user will write new fractal shapes as WGSL occasionally and will shape everything else live via faders and keys.

## Goals / Non-Goals

**Goals:**
- Visual events land on the audible beat (< 1 frame offset).
- New look = one `.wgsl` + one `.ron`; edits visible while music plays (hot reload, native).
- The show never goes black: invalid shaders/manifests keep the last good version running.
- 60 fps at native resolution on an M-series Mac for the starter pack (dynamic resolution as safety net).
- Same track → same show (deterministic).

**Non-Goals:**
- MilkDrop `.milk` preset compatibility.
- No-code shape editor, MIDI input, reshuffle key, multi-monitor output (designed-for, not built).
- WebGL2 fallback.

## Decisions

### D1. Layered architecture
```
 Signal bus → Modulation → Scene(+Variant) → Director → Compositor → screen
   (engine)     (engine)      (files)          (file)      (engine)
```
Per frame on the render thread: read clock → compute musical time → sample score & live FFT → build signal snapshot → director update → evaluate routes → write uniforms → render layers → egui overlay/deck.

### D2. Musical time as the master clock
`beats: f64` = beat index + phase from the score's tempo segment at the audible position. Shaders never see wall time. Pause freezes; seek jumps. Beatless regions: `beats` advances at a nominal 120 BPM equivalent but `beat/bar/phrase` phases are marked unreliable (routes can gate on `beat_confidence`). Before the score covers the playhead: live-only mode (energy and live onsets) with a crossfade into score mode when coverage arrives.

### D3. Signal kinds
| Kind | Signals |
|---|---|
| Phase 0→1 | `beat`, `bar`, `phrase` (8 bars), `section_progress` |
| Trigger | `kick`, `snare`, `hat`, `downbeat`, `drop`, `section_change` (fired from score timestamps as the clock crosses them — zero detection latency) |
| Continuous | `energy`, `bass`, `mid`, `treble`, `brightness`, `tension`, `drop_in` (beats), live FFT bands (texture only) |
| Discrete | `section_kind`, `section_label`, `beat_confidence` |
Triggers crossed between frames are all delivered (with sub-frame offset), so fast hats aren't lost at 60 fps.

### D4. Modulation
Route = `source → [shaper…] → target` with mode `set | add | mul` and a depth. Evaluated on CPU each frame (tens to hundreds of routes — trivial). Shapers:
`envelope(attack, release)`, `smooth(beats)`, `curve(linear|ease_in|ease_out|ease_in_out|exp|step|sine)`, `range(lo, hi)`, `quantize(1/4|1/2|beat|bar)`, `sample_hold(on: trigger)`, `random(per: beat|bar|phrase)`, `step_seq([..16], rate)`, `gate(when: section_kind in [...])`.
Times are in beats unless suffixed `ms`. `random` = hash(track_seed, scene id, route id, period index) → deterministic.
Order of application per param: variant base value → manual fader override → macros → routes (add/mul) → clamp to range.

### D5. Scene format
```
visuals/scenes/<id>/scene.wgsl   fn scene(uv: vec2f, m: Music, p: Params) -> vec4f   (fragment)
                                 or @compute entry + fn resolve(...) for compute scenes
visuals/scenes/<id>/scene.ron    name, tags, kind: fragment|compute, params{name: (type, default,
                                 range, mutable)}, macro_map{macro: [(param, amount)]}, routes[...],
                                 feedback: Option<(decay, warp)>
```
The engine generates WGSL for `struct Params` from the manifest (f32, vec2, vec3, color, int) and prepends `prelude/*.wgsl`: `struct Music` uniform, `prev_frame` texture+sampler, helpers (complex math, noise/fbm, palettes, rotations, SDFs, raymarch utils). Uniforms are packed into one buffer per scene (std140-safe layout generated from the manifest).

### D6. Validation and hot reload
On change (notify, native): parse RON (serde errors with line numbers) → assemble WGSL → validate with naga → create pipeline inside a wgpu error scope. Success → atomic swap at the next frame; failure → keep the old pipeline and show an error toast (file, line, message) for 8 s. Variants referring to removed params keep working (unknown keys ignored, missing keys default).

### D7. Variants and evolution
`visuals/variants/<scene>/<name>.ron` = `{ scene, parent: Option, seed, params: {..}, route_overrides, tags, rating }`.
- `M` mutate: Gaussian step σ=0.08 in normalized [0,1] param space on mutable params (seeded).
- `Shift+M` big mutate: σ=0.25, and may toggle/replace one route from the scene's optional-route pool.
- `K` keep: save current state (including fader positions) as a new variant, auto-named `<adjective>-<hex4>`, parent recorded.
- `Backspace` undo: back through a session history stack.
- `1–5` rate: stored in the variant; director weights picks by rating.

### D8. Director
`visuals/director.ron`: ordered rules `{ on: event|condition, do: action }`. Events: `build_start`, `drop`, `breakdown_start`, `section_change`, `track_change`, `idle(bars)`. Actions: `cut(pick)`, `crossfade(pick, bars)`, `morph(pick_sibling, bars)`, `set_macro(name, value|signal, glide)`, `flash(beats)`. Pick expressions: `tag(...)`, `same_as_label`, `highest_rated_unused`, `random_weighted`. All transitions start/land on downbeats (a `drop` cut lands exactly on the drop beat, using `drop_in` to pre-schedule). Default rules:
```
build      → set_macro(stretch, tension, glide 1 beat); no scene change
drop       → cut(highest_rated_unused) + flash(1)
breakdown  → crossfade(tag("calm"), 1 bar)
label seen → return to the variant last used for that label
idle 16 bars → morph(sibling, 4 bars)
track_change → crossfade(random_weighted, 2 bars)
```
Show state (which variant per label) is per track and deterministic from the seed.

### D9. Compositor
```
[scene A] ─┐
           ├─ transition mix ─▶ [feedback warp (prev frame, decay, warp fn)] ─▶ [post: bloom,
[scene B] ─┘  (only during xfade)                                                chroma kick,
                                                                                 vignette, grain]
            render targets at scale s ∈ [0.5, 1.0]  ──upscale──▶ surface  ─▶ egui overlay/deck (full res)
```
- Offscreen targets in Rgba16Float; ping-pong for feedback.
- Dynamic resolution: governor on smoothed frame time; if > 90% of the vsync budget for 30 frames, reduce s by 0.1; raise when < 60%. Crossfades temporarily double scene cost, so the governor pre-lowers s at transition start.
- Device loss: recreate device resources, reload pipelines, resume from musical time.

### D10. Overlay
Bottom-left: artist (small caps), title (large), progress line with section ticks (kinds colored, provisional ticks faint), `m:ss / m:ss`. Readable on any background via soft dark gradient + text shadow. Visible 5 s on fullscreen entry and on track change, then fades out over 1 s; reappears on mouse move or any key. Font: bundled OFL font (e.g., Space Grotesk/Inter) + the skin's LCD green as accent option.

### D11. Fader deck
Toggle with `D` (fullscreen only). Two groups: MACROS (6) and SCENE PARAMS (up to 12 mutable params of the current scene). Each fader shows the base (thin line) and the live post-modulation value (knob). Modes: **AUTO** (follows director/automation) or **MANUAL** (user value wins). On release, a MANUAL fader glides back to AUTO after its RETURN time; RETURN ∈ {1 beat, 1 bar, 4 bars, phrase, ∞}; a global RETURN selector plus per-fader override (right-click cycles). Return glides land on a bar line. `speed` snaps to ¼, ½, 1, 2, 4×. Deck also shows current scene/variant name, rating, and M/K/undo buttons. `K` captures current fader positions into the new variant.

### D12. Starter scenes
1. `julia_tunnel` (fragment): Julia set with orbit-trap coloring; `c` sample-held per bar, kick → zoom punch.
2. `liquid_feedback` (fragment, feedback-heavy): simple emitter shapes + strong warp field; the MilkDrop feel.
3. `kifs_cathedral` (fragment, raymarched): kaleidoscopic IFS; folds on beat, camera flight speed on tempo; tagged `3d`.
4. `flame` (compute): fractal flame chaos-game accumulation into a storage buffer, log-density tone map; transform weights from bands; tagged `calm`/`silky`.

## Risks / Trade-offs

- [Raymarched KIFS too heavy at 4K] → Dynamic resolution + iteration caps as mutable=false params; scene-level `max_scale`.
- [Compute flame on WebGPU limits (atomics on storage buffers)] → Use u32 atomic add accumulation (supported in WebGPU); fixed-point density.
- [Provisional sections cause wrong cuts] → Director acts on `drop` only when final or when `drop_in` has been stable for ≥ 8 beats.
- [Too many routes make scenes unreadable] → Manifest supports route groups with names; deck shows route depths.
- [egui overlay cost] → Minimal widgets; overlay hidden (no layout) when faded out.

## Open Questions

- Exact visual style of overlay typography (tune after first prototype).
- Whether the director should also react to energy within long `groove` sections beyond idle morphs.
