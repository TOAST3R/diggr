## Why

A spectrogram shows what the 19-bar analyzer can't: the whole frequency picture over time. It's beautiful to watch, and it's the standard way to check a file's real quality. An upsampled 128 kbps MP3 sold as FLAC shows a hard wall at 16 kHz. The user wants to open one from the player.

## What Changes

- **Spectrogram window:** a separate, resizable window (`S`, or **Spectrogram** in the playlist's OPT menu) showing the current track's spectrogram on a log-frequency axis with a perceptual colour map. It has three modes:
  - **Track:** the whole track, coloured by level, with the playhead. Click to seek; drag or scroll to zoom into any range.
  - **Detail:** zoomed ranges computed on demand at the native rate, with an FFT size that follows the zoom (512 to 8192 points), so fine structure is sharp at any zoom.
  - **Live:** a scrolling waterfall of the audible sound from the audio tap, in sync with what you hear.
- **Readouts:** the cursor shows time, frequency (Hz and note name) and level (dB). There are axis labels, a channel selector (mid and side for the whole track; also left and right when zoomed or live) and a dB range control.
- **Quality check:** automatic detection of the spectrum's high-frequency cutoff. The window shows a verdict such as "Content ends at 16.0 kHz: likely from a lossy source (≈128 kbps MP3)" for files decoded with a lossless codec (FLAC, WAV, ALAC) that show a lossy-style cutoff.
- **Spectral overview:** the track overview pass (from `waveform-navigation`) also produces a bounded spectral overview (at most 4096 columns, mid and side), so the track mode opens instantly and costs the same for a 3-minute track as for a 2-hour mix. The overview cache format changes, so cached overviews are rebuilt once.

## Capabilities

### New Capabilities
- `spectral-analysis`: the spectral overview in the overview pass, on-demand detail computation, and cutoff detection.
- `spectrogram-window`: the window, its modes, axes, readouts, controls and seeking.

### Modified Capabilities
- `player-window`: keyboard shortcuts gain `S` (spectrogram window).

## Impact

- Builds on `waveform-navigation` (done and archived): the `track-overview` pass, its cache and the waveform's seek path.
- `crates/analysis`: spectral overview and cutoff data in `overview.rs` (cache version 2), detail worker, cutoff detector.
- `crates/audio`: `TrackInfo` gains a `lossless` flag from the codec type.
- `crates/ui`: a second native window via egui viewports, GPU texture uploads for spectrogram tiles, live FFT fed from the app's existing tap loop, an OPT menu entry and the help panel. The web version is left to `web-target`.
- New dependency: none expected (FFT via the existing `realfft`/`rustfft` used by the analyzer).
