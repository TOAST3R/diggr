## 1. Spectral analysis

- [ ] 1.1 Spectral overview in `OverviewBuilder`: 4096-point Hann FFT every 1024 frames on mid and side, 256 log rows (interpolated below one bin), power-mean columns that merge in pairs at 4096, u8 dB; shared via `Arc` in snapshots; `CACHE_VERSION` → 2; 2-hour mix still ≤ 16 MB
- [ ] 1.2 Cutoff data in the same pass: loud-frame long-term linear spectrum (2049 bins) and per-frame cutoff histogram, saved with the overview
- [ ] 1.3 `TrackInfo.lossless` from the symphonia codec type (FLAC, PCM, ALAC)
- [ ] 1.4 Cutoff detector: 90th-percentile cutoff, lossless + < 19.5 kHz + > 40 dB drop within 500 Hz, bitrate hint table
- [ ] 1.5 Detail worker: own decoder, seek before range and trim, FFT size = smallest power of two ≥ 4× column hop (512–8192), Hann, progressive strips, small result cache
- [ ] 1.6 Tests: tone lands on its row, bounded size for 3-minute vs 2-hour (and unknown-length) input, 2-hour total ≤ 16 MB, detail resolves events 10 ms apart at 2 s/1000 columns, brick-wall 16 kHz FLAC flagged, gradual roll-off not flagged, AAC not flagged

## 2. Window and rendering

- [ ] 2.1 Deferred egui viewport with `Arc<Mutex>` shared state; `S` toggle; **Spectrogram (S)** in the playlist OPT menu; persisted size and mode; `S` in the help panel
- [ ] 2.2 wgpu paint callback: u8 tiles as textures, colour-map shader with dB range uniform
- [ ] 2.3 Track mode with playhead; detail mode switching when zoomed beyond overview resolution
- [ ] 2.4 Live mode: fan out tap chunks from the app's tap loop → 4096 FFT, ring texture column updates aligned to the audible clock
- [ ] 2.5 Repaint policy: live playing, detail arriving, or input only; paused while fullscreen

## 3. Interaction and readouts

- [ ] 3.1 Axes (time, log frequency), cursor readout with note names, channel selector (mid/side in track mode; mid/L/R/side in detail and live), dB range control
- [ ] 3.2 Click to seek (same path as the waveform, clears loops outside the range), scroll/Shift+scroll zoom, drag to pan
- [ ] 3.3 Quality verdict line with cutoff and bitrate hint
- [ ] 3.4 Headless egui tests: open/close with S, readout mapping (440 Hz → A4), click-to-seek mapping, help panel lists S

## 4. Docs and verification

- [ ] 4.1 README: spectrogram window, modes, quality check, `S` key, cache rebuilt once after upgrade; update the test count
- [ ] 4.2 Manual check with a real FLAC and a transcoded one; live mode sync by eye
