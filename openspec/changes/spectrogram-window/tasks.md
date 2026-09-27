## 1. Spectral analysis

- [ ] 1.1 Spectral overview in the overview pass: 4096 columns × 256 log rows, u8 dB, mid channel plus side, cached with the overview
- [ ] 1.2 Detail worker: own decoder, seek before range and trim, FFT size selection up to 8192, Hann, progressive strips, small result cache
- [ ] 1.3 Cutoff detector with steep-edge test and bitrate hint table
- [ ] 1.4 Tests: tone lands on its row, fixed size across lengths, detail time resolution, brick-wall 16 kHz flagged, gradual roll-off not flagged

## 2. Window and rendering

- [ ] 2.1 Deferred egui viewport, `S` toggle, persisted size and mode; web fallback panel
- [ ] 2.2 wgpu paint callback: u8 tiles as textures, colour-map shader with dB range uniform
- [ ] 2.3 Track mode with playhead; detail mode switching when zoomed beyond overview resolution
- [ ] 2.4 Live mode: tap → 4096 FFT, ring texture column updates aligned to the audible clock
- [ ] 2.5 Repaint policy: live playing, detail arriving, or input only; paused while fullscreen

## 3. Interaction and readouts

- [ ] 3.1 Axes (time, log frequency), cursor readout with note names, channel selector, dB range control
- [ ] 3.2 Click to seek, scroll/Shift+scroll zoom, drag to pan
- [ ] 3.3 Quality verdict line with cutoff and bitrate hint
- [ ] 3.4 Headless egui tests: open/close with S, readout mapping (440 Hz → A4), click-to-seek mapping

## 4. Docs and verification

- [ ] 4.1 README: spectrogram window, modes, quality check
- [ ] 4.2 Manual check with a real FLAC and a transcoded one; live mode sync by eye
