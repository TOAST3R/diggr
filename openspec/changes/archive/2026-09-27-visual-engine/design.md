## Context

Hosted inside `classic-ui`'s fullscreen mode through `ui::fullscreen::VisualScene`: `init(render_state)` once with eframe's wgpu device, then per frame `paint(rect, frame) -> PaintCallback` (GPU work in the callback's `prepare`/`paint`) and `ui(ui, frame)` for the egui layer; `key(key, mods)` for keys the host does not keep (it keeps `T`, `A`, and while annotating `Space`/`1`–`6`). Inputs per frame (`SceneFrame`): the audible `Position`, the 19 live spectrum bars, artist/title, and the audible track's `SongScore` when analyzed (beats, events, sections, tension, drop_in). Section *kinds* are heuristic on real music; boundaries, energy and beats are reliable. Targets: native (Metal on macOS) and Chrome WebGPU — both support compute. The user will write new fractal shapes as WGSL occasionally and will shape everything else live via faders and keys.

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

As built: the engine encodes and submits all offscreen work itself during the host's `paint` call (it keeps clones of eframe's device and queue); the egui paint callback only draws the final post-FX/upscale pass inside egui's render pass. wgpu 30 resources are refcounted, so the callback simply carries clones of the pipeline and bind group. Assets are loaded and validated on the first `init` (entering fullscreen), not at app startup.

### D2. Musical time as the master clock
`beats: f64` = beat index + phase from the score's tempo segment at the audible position. Shaders never see wall time. Pause freezes; seek jumps. Beatless regions: `beats` advances at a nominal 120 BPM equivalent but `beat/bar/phrase` phases are marked unreliable (routes can gate on `beat_confidence`). Before the score covers the playhead: live-only mode (energy and live onsets) with a crossfade into score mode when coverage arrives.

### D3. Signal kinds
| Kind | Signals |
|---|---|
| Phase 0→1 | `beat`, `bar`, `phrase` (8 bars), `section_progress` |
| Trigger | `kick`, `snare`, `hat`, `downbeat`, `drop`, `section_change` (fired from score timestamps as the clock crosses them — zero detection latency) |
| Continuous | `energy`, `bass`, `mid`, `treble`, `brightness`, `tension`, `drop_in` (beats), live FFT bands (`m.bands`, the 19 spectrum bars packed in the uniform, read with `band(m, i)`) |
| Discrete | `section_kind`, `section_label`, `beat_confidence` |
Triggers crossed between frames are all delivered (with sub-frame offset), so fast hats aren't lost at 60 fps. Shaders see each trigger as *beats since it last fired* (`m.kick`, …), shaped with `pulse(m.kick, rate)`; musical time scaled by the speed macro is `m.motion`.

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
The engine generates the bindings (`struct Music` uniform, `prev_frame` texture + sampler, `prev(uv)`, `band(m, i)`, `pulse(...)`) and `struct Params` from the manifest (f32, vec2, vec3, color, int), then the helpers in `prelude/*.wgsl` (complex math, hashes/PCG/noise/fbm, palettes, rotations, SDFs, camera/fog), the author's code, and the entry points. Params are one `vec4` per parameter in a uniform array, unpacked by a generated `load_params()` (uniform-layout safe by construction). `uv` is centered with y in −1..1 and x scaled by the aspect ratio. Compute scenes write `fn simulate(i, m, p)` (one call per invocation) and splat into a half-resolution fixed-point accumulation buffer with `splat(uv, color)`; `fn scene` reads it back with `accum_at(uv)`. Raymarch loops stay in the author's code (WGSL has no function pointers for a generic marcher).

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
`visuals/director.ron`: ordered rules `{ on: event, do: [actions] }`, reloadable at runtime.
- **Events** (from the `SongScore` as the audible position crosses a section boundary, plus time):
  - `rise(min_db)` / `fall(min_db)`: the new section is louder / quieter than the last by at least `min_db` of energy.
  - `change`: any other section boundary.
  - `kind(drop|build|breakdown|…)`: optional, for music that does follow that structure (off in the default show).
  - `repeat`: the new section's label was seen before in this track.
  - `track_change`, `idle(bars)`.
- **Actions:** `cut(pick)`, `crossfade(pick, bars)`, `morph(pick, bars)`, `set_macro(name, value|signal, glide)`, `flash(beats)`.
- **Picks:** `tag(...)`, `same_as_label`, `highest_rated_unused`, `random_weighted`, `sibling` (another variant of the current scene).
- **Scheduling:** transitions start and land on downbeats. A boundary is pre-scheduled from the score (the next section start is known ahead of time), so a cut is presented in the frame where the section's first beat is audible. As built: boundary transitions start in the frame where the section's first beat becomes audible and last whole bars; idle morphs fire only on a downbeat trigger. A morph to a look of another scene becomes a crossfade.
- **Track identity vs seed:** the director resets its per-track state when the audible track changes; when the seed changes within a track (analysis arrives and replaces the title hash with the content hash) it only reseeds its random picks.
- **Safety:** a boundary is acted on only if it is final, or has stayed at the same beat for at least 8 beats of analysis.

Default rules (first match wins; `repeat` comes first so a recurring label returns to its look even when it is also louder, as the structural-memory requirement asks):
```
repeat              → cut(same_as_label)
rise(4 dB)          → cut(highest_rated_unused) + flash(1 beat)
fall(4 dB)          → crossfade(tag("calm"), 1 bar)
change              → morph(sibling, 2 bars)
idle(16 bars)       → morph(sibling, 4 bars)
track_change        → crossfade(random_weighted, 2 bars)
always              → set_macro(stretch, tension)          (continuous, not an event)
```
Show state (which variant per label, rng) is per track and deterministic from the seed (`SongScore.content_hash`, or a hash of the title before analysis).

### D9. Compositor
```
[scene A] ─┐
           ├─ transition mix ─▶ [feedback warp (prev frame, decay, warp fn)] ─▶ [post: bloom,
[scene B] ─┘  (only during xfade)                                                chroma kick,
                                                                                 vignette, grain]
            render targets at scale s ∈ [0.5, 1.0]  ──upscale──▶ surface  ─▶ egui overlay/deck (full res)
```
- Offscreen targets in Rgba16Float; ping-pong for feedback.
- Dynamic resolution: governor on frame time. The host presents with vsync, so wall frame time sits at the refresh interval whenever rendering keeps up; the governor learns that interval (the shortest recent frame) and counts *missed* frames (> 1.4× the interval). 10 misses in a row step s down by 0.1; 600 clean frames (~10 s) step it back up. Crossfades temporarily double scene cost, so the governor pre-lowers s at transition start. A scene can cap s with `max_scale`.
- Feedback: the composite pass mixes each layer's scene color (by its alpha) over the warped previous frame × decay; decay is scaled for frame time and by the feedback macro. Scenes may also sample `prev(uv)` themselves (liquid_feedback does its own flow warp). Post FX (bloom from a quarter-res bright pass + separable blur, chromatic kick, vignette, grain, soft-knee tone map, the hue macro as a global hue rotation) run in the final pass and are not fed back.
- Device changes: eframe owns the wgpu device. If the host calls `init` again with a new render state, all engine resources are re-created from the loaded manifests and the show resumes from musical time (nothing is stored on the GPU that cannot be rebuilt).

### D10. Overlay
Bottom-left: artist (small caps), title (large), progress line with section ticks (kinds colored, provisional ticks faint), `m:ss / m:ss`. Readable on any background via soft dark gradient + text shadow. Visible 5 s on fullscreen entry and on track change, then fades out over 1 s; reappears on mouse move or any key. Font: egui's bundled proportional font (no extra font file), with the skin's LCD green as accent option. When the host's analysis strip is visible, the overlay sits above it.

### D11. Fader deck
Toggle with `D` (fullscreen only; the host keeps `T` and `A`). Two groups: MACROS (6) and SCENE PARAMS (up to 12 faders: one per component of the current scene's mutable non-color params; colors change through mutation and the hue macro). Double-click re-arms a fader to AUTO. Each fader shows the base (thin line) and the live post-modulation value (knob). Modes: **AUTO** (follows director/automation) or **MANUAL** (user value wins). On release, a MANUAL fader glides back to AUTO after its RETURN time; RETURN ∈ {1 beat, 1 bar, 4 bars, phrase, ∞}; a global RETURN selector plus per-fader override (right-click cycles). Return glides land on a bar line. `speed` snaps to ¼, ½, 1, 2, 4×. Deck also shows current scene/variant name, rating, and M/K/undo buttons. `K` captures current fader positions into the new variant.

### D12. Starter scenes
1. `julia_tunnel` (fragment): Julia set with orbit-trap coloring; `c` sample-held per bar, kick → zoom punch.
2. `liquid_feedback` (fragment, feedback-heavy): simple emitter shapes + strong warp field; the MilkDrop feel.
3. `kifs_cathedral` (fragment, raymarched): kaleidoscopic IFS; folds on beat, camera flight speed on tempo; tagged `3d`.
4. `flame` (compute): fractal flame chaos-game accumulation into a storage buffer, log-density tone map; transform weights from bands; tagged `calm`/`silky`.

### D13. Benchmark
`cargo run -p visuals --example visual_bench --release` renders each bundled scene offscreen at the Mac's native fullscreen resolution (3024×1964 on the M2 MacBook here) through the whole chain except the final upscale, waiting for the GPU each frame. After tuning (julia iterations 64, liquid fbm 3 octaves, KIFS 56 steps / 7 folds / `max_scale` 0.6): flame ≈ 5 ms, julia ≈ 7 ms, KIFS ≈ 8 ms (p95 18 ms), liquid ≈ 10 ms mean per frame. In the app, `WINAMP_VISUAL_BENCH=1` first runs 15 s with no visuals (the host's own frame rate), then each scene for 15 s, and writes frame rate, p95 frame time, CPU time in the engine (paint, GPU submit, egui layer), GPU time and render scale to `<config>/winamp_rust/visuals/bench.txt`. The first in-app run showed every scene at ~33 fps with the scale pinned at 0.5, while the same scenes take 5–10 ms offscreen: the bottleneck is not scene cost, and lowering the scale did not help. The governor therefore now undoes a series of steps that reached the floor without reducing missed frames, and waits 5 minutes before trying again. A second run with the host-only phase showed 42–46 fps with *no* visuals and < 1 ms of engine CPU per frame; a host-side frame profile (`WINAMP_FRAME_STATS=1`) put all the time in eframe's paint+present, and a bare eframe probe (`cargo run -p ui --example fullscreen_probe --release`) that only draws black showed the same 30–60 fps swing, with or without Low Power Mode. With vsync and eframe's default of one frame in flight, frames on this M2 regularly miss their vsync slot; with up to three queued (`desired_maximum_frame_latency: Some(3)`) the probe and the player hold 60. The app sets this at startup (`ui::app::surface_config`); switching at runtime is not possible because eframe 0.36's `Frame::set_wgpu_surface_config` changes a clone of the render state. The windowed player draws only on input or at 30 fps, so its queue stays short. Because frames now reach the screen about 2–3 refreshes after they are drawn, the engine reads the audio clock 2.5 refresh intervals ahead while playing. The governor also ignores the second after a scene switch (a one-off pipeline warm-up hitch). Final in-app run: every scene at a steady 60 fps (p95 17.1 ms), flame and julia at full resolution, KIFS at its 0.6 cap.

## Risks / Trade-offs

- [Raymarched KIFS too heavy at 4K] → Dynamic resolution + iteration caps as mutable=false params; scene-level `max_scale`.
- [Compute flame on WebGPU limits (atomics on storage buffers)] → Use u32 atomic add accumulation (supported in WebGPU); fixed-point density.
- [Provisional sections cause wrong cuts] → Director acts on `drop` only when final or when `drop_in` has been stable for ≥ 8 beats.
- [Too many routes make scenes unreadable] → Manifest supports route groups with names; deck shows route depths.
- [egui overlay cost] → Minimal widgets; overlay hidden (no layout) when faded out.

## Open Questions

- Exact visual style of overlay typography (tune after first prototype).
- Whether the director should also react to energy within long `groove` sections beyond idle morphs.
