## Context

- **Engine:** `VisualEngine` implements the host's `VisualScene`. Each frame it reads the playback clock (`Position`), the live 19 spectrum bars and the `SongScore`, runs the signal bus, director and modulation, and renders through `gpu::Gpu` (offscreen targets + a final pass into egui's render pass).
- **Wall clock:** it uses wall time for `dt`, overlay fade and toasts. The governor adapts the render scale, and the clock is read 2.5 refreshes ahead to cover the display delay.
- **Headless GPU:** already works (tests and `visual_bench` render every scene offscreen).
- **Seed:** the show is seeded by the track's content hash.

## Goals / Non-Goals

**Goals:**
- Frame-exact audio/visual sync in the file: frame n shows the musical state at exactly n/fps.
- The same show as live (hands-off, cached analysis).
- Faster than real time at 1080p60 on an M2.
- A simple CLI, plus a background job from the player.

**Non-Goals:**
- A timeline editor, keyframes, or recording live fader moves (a possible later "record performance" mode).
- Other containers/codecs beyond H.264/AAC MP4 (ffmpeg arguments could be exposed later).
- The web target.

## Decisions

### D1. Split the engine's frame step from the host
The engine gets a pure step, `VisualEngine::step(&FrameInput) -> FrameDraw`:
- `FrameInput` holds position, bars, score, title, `dt`, `now` and the output size;
- `step` runs signals → director → modulation and packs uniforms;
- `FrameDraw` is then rendered by `Gpu::render` + `final_to(view)`.

The live `VisualScene` implementation becomes a thin adapter that supplies wall time, the governor scale and the display lead. Offline supplies:
- `dt = 1/fps`, `now = n/fps`;
- a fixed scale of 1.0;
- lead = 0;
- no toasts or watcher.

This keeps one code path, so live and offline can't drift apart.

### D2. Timeline and inputs
- **Frame times:** frame n is rendered at t = from + n/fps.
- **Position:** built from t (`frame = t × sample_rate`, playing).
- **Analysis:** the track is fully analyzed first (cached score if present; otherwise the analyzer runs to completion, which is ~50× realtime).
- **Bars:** the live 19-band analyzer runs offline over the decoded audio, fed chunk by chunk up to each frame time, with the same smoothing as live (it uses the same `dt`).
- **Determinism:** no randomness depends on wall time; the director and modulation RNGs are seeded from the content hash and musical time.

### D3. Readback and encoding
- **Final pass:** into an Rgba8 texture at the output size, copied to one of 3 mapped staging buffers (a ring), so GPU work, readback and piping overlap.
- **Encoder:** frames are streamed as raw RGBA to `ffmpeg -f rawvideo -pix_fmt rgba -s WxH -r fps -i - -ss from -to to -i <track>`, encoded with `-c:v h264_videotoolbox -b:v 20M` (or `libx264 -crf 18` when VideoToolbox is missing), `-pix_fmt yuv420p -c:a aac -b:a 320k -shortest`.
- **Audio:** comes from the original file, so it's bit-for-bit the source before AAC encoding.
- **Missing ffmpeg:** checked before rendering: "ffmpeg not found — install it with `brew install ffmpeg`".
- **Bandwidth:** 1080p RGBA is 8.3 MB per frame, ~500 MB/s at 60 fps through a pipe, well within an M2's capacity. VideoToolbox encodes 1080p60 several times faster than real time.

**As built:** the final pass renders into an Rgba8 texture, and a compute pass converts it on the GPU to planar YUV 4:2:0 (BT.709, limited range). Only 1.5 bytes per pixel then leave the GPU, and ffmpeg receives `-pix_fmt yuv420p` tagged BT.709, so it only has to encode.
- **Why:** with RGBA frames, 8.3 MB per 1080p frame went through the pipe, and ffmpeg's colour conversion cost about as much CPU as the rest. The first full render ran at only 1.0× real time (129 s of CPU for 170 s of video); with YUV it used 41 s.
- **Constraint:** the conversion packs samples into 32-bit words, so the frame width must be divisible by 8 and the height even (1920×1080, 1280×720, 3840×2160 and 1080×1920 all qualify).
- **Readback:** buffer-to-buffer copies into a ring of 3 mapped buffers.
- **Output file:** ffmpeg writes to a hidden `.NAME.part` file next to the output, which is renamed on success and deleted on failure, cancel or Ctrl-C.

### D4. Overlay card
With `--overlay`, the artist/title/progress overlay is drawn for the first 5 s (and faded like live) using egui rendered offscreen with `egui_wgpu::Renderer` into the same frame before readback. Same font and layout as live.

### D5. Pinned look
`--look scene/variant` disables the director's transitions (the macros still follow `Always` rules, and modulation still runs), which is useful for a single-look video.

### D6. From the player
**As built:** `ui` cannot depend on `visuals` (it is the other way round), so `ui::render_job` defines two small traits, `ShowRenderer` and `RenderJob` (status, cancel, pause). `visuals::render::BackgroundRenderer` implements them, and the app passes one in `AppContext`.
- The job runs on a low-priority thread with its own headless GPU device.
- The player's status line reads the job's status every 250 ms and pauses the job while fullscreen is active.
- The playlist entry's context menu shows "Render show…" or, while a render runs, "Cancel show render".
- **Render show… dialog:** the menu opens a dialog with every command-line option. It offers size presets or a custom size, the frame rate, the whole track or a From/To range, the title card, and automatic or pinned look. The looks listed come from `ShowRenderer::looks`, read from the visuals folder.
  - The dialog builds the same `RenderRequest` the CLI does and shares its checks (`ui::render_job::size_ok`, `ui::format::parse_clock`).
  - Size, frame rate and card are remembered in the settings.
- **Opening the menu:** egui's context menu opens only on the secondary button. The playlist also opens it on Control-click when Control isn't the command key (macOS), and that click no longer selects the entry.

A playlist entry's context menu gets "Render show…", which opens a save dialog, then runs the render on a background thread with low priority. A progress line appears in the main window ("Rendering show 42% · 1:10 left"), a notification comes on completion, and it can be cancelled. Playback is unaffected (render and playback share the GPU; the render yields between frames).

### D7. Speed (measured)
All measurements are on the M2 MacBook here, with the 2:50 generated house track at 1080p60.
- **Rendering alone** (frames discarded; `cargo run -p visuals --example render_speed --release`): 166 fps, 2.8× real time.
- **Full render to MP4 with the overlay:** 1.2× real time, but measured on battery with Low Power Mode on. In that mode even ffmpeg's hardware encoder ran at only 0.13× real time on a synthetic source. The "faster than real time" target should be re-checked on AC power with Low Power Mode off.
- **Playback during a render** (1280×720): 0 underruns (`crates/visuals/tests/render_playback.rs`).

## Risks / Trade-offs

- [Live show differs from the render when the live show saw provisional sections] → The spec promises equality only for a replay with a cached analysis. The first live play of an unanalyzed track can legitimately differ.
- [GPU contention with fullscreen visuals during a background render] → The render pauses while fullscreen visuals are active.
- [Large frames through a pipe on slower machines] → The encoder's back-pressure throttles the renderer naturally; progress shows the real speed.
- [ffmpeg licensing and presence] → It is an external tool the user installs; nothing is bundled.

## Open Questions

- Should a "record performance" mode later capture live fader moves and keys into a timeline to re-render?
- Should vertical 1080×1920 presets be offered for phones?
