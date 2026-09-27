## 1. Engine refactor

- [x] 1.1 `FrameInput` / `step` / `FrameDraw`: separate the frame step from egui and wall time; the live `VisualScene` becomes an adapter (wall time, governor scale, display lead)
- [x] 1.2 Fixed scale and zero lead offline; no watcher/toasts; seed and RNGs independent of wall time
- [x] 1.3 `Gpu::final_to(view)` into an Rgba8 texture; 3-buffer readback ring
- [x] 1.4 Tests: two offline runs give identical frames; live adapter output unchanged (existing engine tests pass)

## 2. Offline inputs

- [x] 2.1 Complete analysis before rendering (cache or run to completion)
- [x] 2.2 Offline spectrum bars: feed the 19-band analyzer with decoded audio up to each frame time
- [x] 2.3 Test: kick at 12.500 s produces its visual response on frame 750 at 60 fps

## 3. Encoding and CLI

- [x] 3.1 ffmpeg detection with install hint; VideoToolbox → libx264 fallback; audio muxed from the original with range
- [x] 3.2 `--render-show` with `-o`, `--size`, `--fps`, `--from`/`--to`, `--overlay`, `--look`
- [x] 3.3 Progress with ETA; Ctrl-C and errors remove the partial file
- [x] 3.4 Overlay card via offscreen egui renderer
- [x] 3.5 Measure: 5-minute track at 1080p60 on this Mac; record the speed

## 4. From the player

- [x] 4.1 Playlist context "Render show…" with save dialog; background job, progress line, cancel, completion notice
- [x] 4.2 Pause the render while fullscreen visuals are active; verify no underruns during a render

## 5. Docs

- [x] 5.1 README: rendering shows (CLI and player), ffmpeg requirement, determinism note
