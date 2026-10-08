## Context

What `waveform-navigation` actually built, and what this change builds on:

- **Overview pass** (`analysis::overview`): `OverviewService::request` spawns a low-priority `build` that decodes with its own `TrackDecoder` at the native rate and feeds interleaved stereo into `OverviewBuilder::push`. It publishes a snapshot every 250 ms through an `ArcSwapOption`, keeps 6 tracks in memory, and caches the finished overview as `overview-<hash>.bin` (`CACHE_VERSION` 1). A cache hit returns early without decoding. The player requests the audible track once the analyzer has its first results, and the next track on pre-warm. The whole overview of a 2-hour mix must stay within 16 MB (it is about 13 MB today).
- **Track length can be unknown up front:** `TrackInfo::duration_secs` is an `Option`, because some files have no frame count.
- **Tap:** an SPSC `TapReader` with one consumer. `DiggrApp` pops the 256-frame, position-stamped stereo chunks in its update loop and feeds the 19-bar spectrum analyzer. A second consumer must be fed from that same loop.
- **Codec:** `TrackInfo` has no codec field, so a FLAC/WAV/ALAC file can only be told apart from AAC in an `.m4a` by asking symphonia.
- **Seeking:** the waveform's overview row seeks immediately with `Engine::seek`, and seeking outside an active loop clears it. The help panel (`H`/`F1`, `crates/ui/src/help.rs`) must list every shortcut. The playlist's **OPT** menu is the player's options menu.
- **Windows:** egui 0.36 supports extra native windows (viewports) on desktop, sharing the same wgpu device.
- **Web:** `web-target` is not implemented and is blocked on a seam redesign, so no web code is part of this change.

## Goals / Non-Goals

**Goals:**
- An instant whole-track spectrogram, with sharp detail at any zoom, and a live waterfall.
- Trustworthy forensics: an accurate frequency axis at the native rate, and a cutoff verdict that doesn't cry wolf.
- Bounded memory and CPU for any track length; the player's latency is untouched.

**Non-Goals:**
- Spectral editing, export of images or data (maybe later).
- Detecting every kind of transcode (only the high-frequency cutoff is checked).
- A spectrogram inside fullscreen visuals.
- The web version of the window (it belongs to `web-target`).

## Decisions

### D1. Three data sources for one view
```
 track mode   ◀── spectral overview: ≤ 4096 columns × 256 log rows × {mid, side}, u8 dB (≤ 2 MB), cached with the overview
 detail mode  ◀── on demand: seek + decode + FFT only the visible range, at column resolution
 live mode    ◀── tap chunks (fanned out in the app's tap loop) → 4096-point FFT, one column per ~10 ms, a ring texture
```

**Spectral overview, built in the existing overview pass:**
- **FFT frames:** `OverviewBuilder::push` also runs a 4096-point Hann FFT on mid ((L+R)/2) and side ((L−R)/2) every 1024 frames, or every eighth of a column once columns are wider. That caps the work at about 8 FFTs per column, so a 2-hour mix costs about as much as a 3-minute track.
- **Columns:** a column starts at 1024 frames and holds the mean power of its FFT frames. Whenever the column count reaches 4096, adjacent pairs are merged (power mean) and the column width doubles. This handles unknown track lengths and progressive publishing, and the result always has 2048–4096 columns. A 3-minute track ends with about 3,900 columns and a 2-hour mix with about 2,000. The UI maps columns to time with the stored column width.
- **Rows:** 256 log-spaced rows from 20 Hz to Nyquist. A row narrower than an FFT bin (below about 500 Hz at 4096 points) takes the value interpolated between the neighbouring bins, so no row is empty.
- **Storage:** u8 dB (−120 to 0 dBFS). Merging converts to power, averages and converts back.
- **Why bounded, not full resolution:** full-resolution columns grow with length (a 2-hour mix at 512 rows is about 160 MB). Screen resolution is covered, and zooming further switches to detail mode.
- **Memory:** mid and side at 4096 × 256 are 2 MB, so a 2-hour mix stays at about 15 MB, within the 16 MB bound. `snapshot()` currently clones the whole builder every 250 ms, so the spectral data is shared through an `Arc` rather than cloned.
- **Cache:** the spectral overview is part of `Overview`, and `CACHE_VERSION` goes to 2, so old cache files are rebuilt once.

**Channels:** track mode offers mid and side. Detail and live modes also offer L and R, because they decode or tap the full stereo signal. Storing L and R in the overview as well would double its size for little benefit.

**Detail:** the zoomed range is decoded from a separate decoder, in a background worker (`Priority::Low`), and delivered progressively in column strips. Decoding is about 100× realtime, so a 10-second window at 2,000 columns takes a few milliseconds of FFT. The last few views are cached.

### D2. Frequency axis and FFT sizes
- **Axis:** log frequency from 20 Hz to Nyquist, with 256 rows (overview) or one row per pixel (detail). Rows narrower than a bin are interpolated, as in D1. Asking for "at least one bin per row" is impossible at the low end: at 8192 points a bin is 5.4 Hz, while a pixel row near 20 Hz is under 1 Hz.
- **Detail FFT size:** the smallest power of two at least 4× the column hop, clamped to 512–8192 points. Time and frequency resolution trade off: an 8192-point window lasts 186 ms and would smear the 10 ms events the detail spec asks for. At 2 s over 1,000 columns (2 ms per column) this picks 512 points (11.6 ms). A 60-second view picks 8192.
- **Window:** Hann. **Overlap:** as much as it takes to fill every pixel column.
- **Channels:** the default is mid ((L+R)/2). L, R and side ((L−R)/2) are selectable (see D1 for track mode); side shows stereo width.

### D3. Colour and range
A perceptual colour map (inferno, 256 entries) turns the u8 dB values into colours. The dB range control (default −120 to 0 dBFS) changes contrast without recomputing any spectrum.
- **As built:** the lookup happens on the CPU when a tile is uploaded as an ordinary egui texture, not in a wgpu fragment shader. A range change re-colours at most ~2 M texels (a few ms), the same code draws in the deferred viewport, in an embedded window and in headless tests, and there is no second render pipeline to maintain. A shader remains possible if re-colouring ever shows up in profiles.

### D4. Cutoff detection
**Data:** the log rows are too coarse for this (at 16 kHz a row spans about 440 Hz, as wide as the whole steep-edge window). So the overview pass also keeps, for loud FFT frames (RMS above −30 dBFS):
- a linear-frequency long-term mean power spectrum (2049 bins, about 10.8 Hz each);
- a histogram of per-frame cutoffs. A frame's cutoff is the highest bin within 30 dB of the frame's median level in the 2–8 kHz band.

Both are a few kilobytes and are stored with the overview.

**Deciding:**
- **Cutoff:** the 90th percentile of the per-frame cutoffs.
- **Verdict:** a "lossy-style cutoff" when the codec is lossless (FLAC, PCM/WAV, ALAC), the cutoff is below 19.5 kHz, and the long-term spectrum drops by more than 40 dB within 500 Hz above the cutoff.
- **Codec:** `TrackInfo` gains a `lossless: bool` taken from the symphonia codec type, so ALAC in `.m4a` is covered and AAC in `.m4a` is not flagged.
- **Wording:** always shown as "likely", with the cutoff value. The ≈ bitrate hint comes from a small table: ~16 kHz ≈ 128 kbps, ~19 kHz ≈ 192 kbps, ~20 kHz ≈ 256–320 kbps.
- **Why these thresholds:** music can legitimately lack highs, so the steep-edge test avoids false alarms from dark recordings.

### D5. Window
- **Opening:** a deferred egui viewport titled "Spectrogram — <artist> – <title>", resizable. It opens with `S` or with **Spectrogram (S)** in the playlist's OPT menu. Size and mode persist in `Settings`. The help panel lists `S`.
- **Shared state:** the deferred viewport callback must be `Send + Sync`, so the window's state (view, detail worker handle, live ring) lives behind an `Arc<Mutex<…>>`. The main update loop writes the audible position, the overview handle and the live columns into it, and the window only reads from it and returns actions (seek, mode changes).
- **Rendering:** each tile (overview, detail, live ring) is one texture whose width is frequency and height is time, drawn as a quad with transposed UVs, clipped to what it covers. Live mode updates only the new columns of its ring texture (`set_partial`).
- **Without native windows:** egui embeds a deferred viewport as an in-app window, so the same code serves as the fallback panel.
- **Idle cost:** the window repaints at the display rate while playing in live mode, every 40 ms while the detail worker is delivering strips, every 100 ms while playing in track mode (the playhead), and otherwise only on input. The player wakes it when the track, position or overview changes.

### D6. Interaction
- **Click** in track or detail mode: seek with the same `Engine::seek` path as the waveform's overview row. Seeking outside an active loop clears it, just as the waveform does.
- **Scroll:** zoom time around the cursor. **Shift+scroll:** zoom frequency.
- **Drag:** pan.
- **Readouts:** time (m:ss.mmm), frequency (Hz and nearest note, e.g. "440 Hz A4"), and level (dB) at the cursor.

## Risks / Trade-offs

- [Two windows on one wgpu device double presentation work] → The spectrogram repaints only when needed. Fullscreen visuals keep priority, and the spectrogram window stops repainting while fullscreen is active.
- [False "lossy" verdicts] → Conservative thresholds, steep-edge requirement, "likely" wording, cutoff value always shown. Tested with synthetic brickwall-filtered versus naturally dark signals.
- [Seeking inside compressed files for detail can be inexact] → Decode from a little before the range and trim by timestamps; the overview is still available as a fallback.
- [The overview pass gets slower] → At most 43 FFTs per second of audio per channel (short tracks), and far fewer for long mixes, which is small next to decoding. It stays at low priority. (Found while testing: the band filters hit denormal floats after silence or DC, making the pass ~15× slower; their outputs are now flushed to zero.)
- [Cache version bump] → Every cached overview is rebuilt once on first play after upgrading.

## Open Questions

- Should the verdict also appear as a marker in the playlist? (It could be a small follow-up.)
- Should reassigned or "sharpened" spectrograms be offered for extra detail? (Probably not needed.)
- Should detail mode draw the beat grid and section bands like the waveform does? (Cheap, since the score is already in the app.)
