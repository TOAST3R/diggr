## Why

The thin seek bar says nothing about the music, while the analyzer already knows the beat grid, the sections and where the energy rises. A DJ-style coloured waveform, together with keys that jump by musical structure, turns that knowledge into navigation: see the drop coming, jump to the next section on the beat, and loop a section without breaking the groove.

## What Changes

- **Track overview pass:** a background pass that decodes each track at its native rate in stereo and builds a waveform pyramid (min/max per channel, RMS, and low/mid/high energy per block). It is cached by content hash like scores, available progressively, and never delays playback or the playhead analysis. The spectrogram change reuses this pass.
- **Waveform section:** a new skinned section under the main window, toggled with `W`. It has two rows:
  - an overview of the whole track with section bands, drop markers and the playhead (click or drag to seek);
  - a zoomed view scrolling around the playhead, coloured by frequency (low/mid/high → RGB) with the beat grid. Downbeats are stronger and zoom follows the scroll wheel.
- **Structure navigation:** keys that work in the window and in fullscreen:
  - `[` / `]`: previous / next section;
  - `Shift+]`: next energy rise ("the drop");
  - `L`: loop the current section, or 4/8/16 bars with `Shift+L`.
  - Jumps and loops land on a downbeat, sample-accurately, so the beat never stumbles.
- **Audio engine:** seeks scheduled for an exact frame, and gapless loop regions, both prepared ahead by the decode worker.

## Capabilities

### New Capabilities
- `track-overview`: the background native-rate overview pass, its multi-resolution waveform data, and its cache.
- `waveform-view`: the waveform section, including its overview and zoomed rows, colouring, markers, seeking and zoom.
- `structure-navigation`: section and drop jumps and bar-quantized loops driven by the song score.

### Modified Capabilities
- `audio-playback`: adds frame-scheduled seeks and gapless loop regions.
- `player-window`: keyboard shortcuts gain `W`, `[`, `]`, `Shift+]`, `L`, `Shift+L`.

## Impact

- `crates/analysis`: new overview module (portable, no platform code) and cache entries.
- `crates/audio`: worker commands `SeekAt { frame, target }` and `SetLoop`, pre-decoding the target; clock discontinuity semantics at scheduled seeks.
- `crates/ui`: waveform section, skin layout (the section is drawn in the skin's style; no new skin atlas art is required), key routing, settings (`show_waveform`, zoom).
- Web target: the overview pass and navigation are portable. The waveform section is egui and works on the web too.
