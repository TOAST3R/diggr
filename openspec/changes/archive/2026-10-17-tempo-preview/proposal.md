## Why

A DJ judges a record at the tempo it will be played at. A 122 BPM groove feels different pushed to 126, and a digger auditioning a chart for a 126 BPM set wants to hear every track there, the way it'll sit in the mix. Today every preview plays at its own tempo, and the engine can't change speed at all.

## What Changes

- **SYNC:** a SYNC control in the main window, off by default.
  - **Turning it on:** it takes the playing track's BPM as the target. The target is changed by dragging or scrolling its LCD value, or with Shift+↑ / Shift+↓ (±1 BPM).
  - **While on:** every track plays at `target / its BPM`, vinyl-style (pitch moves with speed), within a range of ±8% (switchable to ±16%).
  - **Out of range:** a track whose tempo can't reach the target, or that has no BPM yet, plays at its own tempo, marked "(own tempo)".
- **Instant and click-free:** a tempo change is heard within 50 ms, like a seek, with no click and no underrun. The next track starts at its own fitted speed, gaplessly.
- **Everything stays in the track's time:** the waveform, structure jumps, loops, the BPM shown, analysis and the spectrogram all keep working in the track's own time. Only what you hear is faster or slower, and the main window shows the playing speed ("+2.4%").
- **The engine foundation for `beatmatch-automix`:** a variable-ratio resampler in the decode worker and a clock that reports source time under speed changes. Automix will build on exactly this.

## Capabilities

### New Capabilities
- `tempo-preview`: the SYNC control, the target BPM, per-track speed and its range, what is shown, and how it persists.

### Modified Capabilities
- `audio-playback`: a new Varispeed requirement for the engine (speed changes heard within 50 ms, gapless across tracks, real-time safe).
- `playback-clock`: a new requirement that the clock reports source position and source-time rates under varispeed.

## Impact

- **`crates/audio`:**
  - `ChannelResampler` gains rubato's `Async` (sinc) for speeds other than 1, keeping the FFT path at speed 1 (and for analysis);
  - `Engine::set_speed(index, speed)`: per-queue-entry speeds;
  - the speed change is delivered by a soft flush of the ring: the old audio plays until the new is queued, then a 2 ms crossfade (design D2);
  - the clock stamps source frames, and the speed travels with them;
  - `--bench` gains a varispeed run.
- **`crates/ui`:** the SYNC control in the skin's main window, the target in settings, the per-track ratio from each entry's BPM, keys, the "+2.4%" display and "(own tempo)".
- **Skin:** a SYNC button and the LCD readouts' layout, so `skin-gen` and its generated assets change.
- **README:** SYNC, its keys, the ±8/16% range, test areas and count.
- **Performance:** sinc resampling costs more CPU than FFT. Launch < 300 ms, start < 30 ms, seek < 50 ms and 0 underruns must hold (the design measures it).
