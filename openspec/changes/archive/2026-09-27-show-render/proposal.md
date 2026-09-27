## Why

The visual show is deterministic: the same track, the same looks and the same director rules always give the same show, and the renderer already runs headless on the GPU. That makes it possible to render a track's show to a video file, faster than real time, to share a set's visuals, make a music video, or project them without running the player. Few music players can do this.

## What Changes

- **Render command:** `winamp-native --render-show <track> -o show.mp4` renders the hands-off show for a track to an H.264/AAC MP4. Options:
  - `--size WxH` (default 1920×1080) and `--fps` (default 60);
  - `--from`/`--to` for a time range;
  - `--overlay` for the artist/title card at the start;
  - `--look <scene>/<variant>` to pin one look with the director off.
- **From the player:** "Render show…" on a playlist entry runs the same render in the background with progress, and notifies when the file is ready.
- **Offline visual engine:** the engine steps on a supplied timeline (frame n at n/fps) instead of the wall clock, at a fixed render scale, with no display-delay lead. It reads the audio and spectrum bars decoded offline and the track's complete analysis.
- **Same show:** a render matches the live show of a replay with a cached analysis and no user input (same seed, same looks, same director decisions, same frames at the same musical times).

## Capabilities

### New Capabilities
- `show-render`: offline rendering of a track's visual show to video: timeline, determinism, encoding, options, progress, and the in-player entry point.

### Modified Capabilities
<!-- none: the live visual requirements are unchanged; the engine gains an offline mode behind the same behaviour -->

## Impact

- `crates/visuals`: separate the frame step from egui and wall time (an injected time source and frame input); an offscreen final pass into a readable texture; headless egui rendering for the overlay card.
- `crates/ui`: the offline spectrum analyzer (feeds bars at exact frame times); a playlist context entry and background job.
- `apps/native`: `--render-show` command.
- External tool: `ffmpeg` on the PATH (checked up front, with install instructions); VideoToolbox H.264 when available.
- Not on the web target (no process to pipe to).
