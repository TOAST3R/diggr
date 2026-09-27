## 1. Track overview pass

- [x] 1.1 Overview data model: base blocks (min/max L/R, RMS, low/mid/high), halved levels, level selection by pixel size
- [x] 1.2 Background pass: own decoder at native rate, crossover filters, progressive publishing, lowest priority after the analyzer's first horizon
- [x] 1.3 Versioned cache by content hash; load on replay
- [x] 1.4 Tests: peak accuracy, band dominance, progressive coverage, 2-hour mix memory ≤ 16 MB, start/analysis budgets unaffected

## 2. Engine: scheduled seeks and loops

- [x] 2.1 Worker command to seek at an exact output frame: pre-decode the target, switch at the frame with a 2 ms crossfade, clock discontinuity
- [x] 2.2 Reject too-late requests with an event; engine API `seek_at` / `set_loop`
- [x] 2.3 Gapless loop regions (wrap with the same mechanism), cleared on seek outside, stop or track change
- [x] 2.4 Tests with ManualSink: bit-exact splice at the frame, too-late rejection, loop wraps repeatedly with no gap, loop cleared on track change

## 3. Waveform section

- [x] 3.1 Section layout under the main window, `W` toggle, persisted setting, skin border framing
- [x] 3.2 Overview row: coloured waveform, section bands (faint when provisional), drop markers, playhead, click/drag seek
- [x] 3.3 Zoomed row: playhead-centered scrolling, RGB band colouring, mirrored channels, beat and downbeat ticks, wheel zoom 1–64 bars
- [x] 3.4 Idle cost: no repaint while paused; mesh reuse
- [x] 3.5 Headless egui tests: toggle, click-to-seek mapping, colour of synthetic kick versus hat blocks

## 4. Structure navigation

- [x] 4.1 Targets from SongScore: next/previous section, next rise ≥ 4 dB with fallback
- [x] 4.2 Quantization: next audible downbeat to stream frame via the clock; immediate fallback when beat confidence is low or it's too late
- [x] 4.3 Keys `[`, `]`, `Shift+]`, `L`, `Shift+L` in window and fullscreen; loop region drawn on the waveform
- [x] 4.4 Tests: target selection on synthetic scores, quantized jump lands on the downbeat, loop lengths

## 5. Docs and verification

- [x] 5.1 README: waveform and structure keys
- [x] 5.2 Manual check on real tracks: jumps on the beat, loops seamless, waveform colours readable
- [x] 5.3 Shortcuts help panel (`H` / `F1`) listing every key and mouse action; Esc closes it before leaving fullscreen
