## Context

- `waveform-navigation` adds a background overview pass that decodes each track at its native rate in stereo and caches the results by content hash.
- The audio tap delivers position-stamped stereo chunks of what is audible, which already feed the 19-bar spectrum and the scope.
- egui 0.36 supports extra native windows (viewports) on desktop, sharing the same wgpu device.
- Long mixes (2 h) must stay cheap, the same constraint as the analyzer's rolling horizon.

## Goals / Non-Goals

**Goals:**
- An instant whole-track spectrogram, with sharp detail at any zoom, and a live waterfall.
- Trustworthy forensics: an accurate frequency axis at the native rate, and a cutoff verdict that doesn't cry wolf.
- Bounded memory and CPU for any track length; the player's latency is untouched.

**Non-Goals:**
- Spectral editing, export of images or data (maybe later).
- Detecting every kind of transcode (only the high-frequency cutoff is checked).
- A spectrogram inside fullscreen visuals.

## Decisions

### D1. Three data sources for one view
```
 track mode   ◀── spectral overview: fixed 4096 columns × 256 log-frequency rows, u8 dB (1 MB), cached
 detail mode  ◀── on demand: seek + decode + FFT only the visible range, at column resolution
 live mode    ◀── tap chunks → 4096-point FFT, one column per ~10 ms, a ring texture
```
- **Why fixed-size overview:** storing full-resolution columns grows with length. A 2-h mix at 512 rows would be ~160 MB. 4096 columns cover any track at screen resolution, and zooming beyond that switches to detail mode.
- **Detail:** the zoomed range is decoded from a separate decoder, in a background worker, and delivered progressively in column strips. Decoding is ~100× realtime, so a 10-s window at 2,000 columns is a few milliseconds of FFT. Results are cached for the last few views.

### D2. Frequency axis and FFT sizes
- **Axis:** log frequency from 20 Hz to Nyquist, 256 rows (overview) or one row per pixel (detail).
- **Detail FFT size:** chosen so each row has at least one bin, up to 8192 at 44.1/48 kHz, with a Hann window.
- **Overlap:** time columns overlap as needed to fill the pixel columns.
- **Channels:** the default is mid ((L+R)/2); L, R and side ((L−R)/2) are selectable, and side shows stereo width.

### D3. Colour and range
A perceptual colour map (inferno-like, 256 entries) is applied in a fragment shader from u8 dB values, so the dB range control (default −120 to 0 dBFS) changes contrast without recomputing anything.

### D4. Cutoff detection
For each overview column, the detector finds the highest frequency whose level stays within 30 dB of the column's median in the 2–8 kHz band.
- **Cutoff:** the 90th percentile of those frequencies over loud columns (RMS above −30 dBFS).
- **Verdict:** a "lossy-style cutoff" when a lossless-container file (FLAC, WAV, ALAC) has a cutoff below 19.5 kHz with a steep edge (more than 40 dB drop within 500 Hz).
- **Wording:** always shown as "likely", with the cutoff value. The ≈ bitrate hint comes from a small table: ~16 kHz ≈ 128 kbps, ~19 kHz ≈ 192 kbps, ~20 kHz ≈ 256–320 kbps.
- **Why these thresholds:** music can legitimately lack highs, so the steep-edge test avoids false alarms from dark recordings.

### D5. Window
- **Opening:** a deferred egui viewport, titled "Spectrogram — <artist> – <title>" and resizable. Size and mode persist in settings.
- **Web:** it becomes an overlay panel in the page instead.
- **Rendering:** a wgpu paint callback draws the u8 tiles as textures with the colour-map shader. Live mode updates one texture column per step.
- **Idle cost:** the window repaints only while playing in live mode, or while the detail worker delivers strips.

### D6. Interaction
- Click in track or detail mode: seek.
- Scroll: zoom time around the cursor. Shift+scroll: zoom frequency.
- Drag: pan.
- Readouts: time (m:ss.mmm), frequency (Hz and nearest note, e.g. "440 Hz A4"), and level (dB) at the cursor.

## Risks / Trade-offs

- [Two windows on one wgpu device double presentation work] → The spectrogram repaints only when needed. Fullscreen visuals keep priority, and the spectrogram window stops repainting while fullscreen is active.
- [False "lossy" verdicts] → Conservative thresholds, steep-edge requirement, "likely" wording, cutoff value always shown. Tested with synthetic brickwall-filtered versus naturally dark signals.
- [Seeking inside compressed files for detail can be inexact] → Decode from a little before the range and trim by timestamps; the overview is still available as a fallback.

## Open Questions

- Should the verdict also appear as a marker in the playlist? (It could be a small follow-up.)
- Should reassigned or "sharpened" spectrograms be offered for extra detail? (Probably not needed.)
