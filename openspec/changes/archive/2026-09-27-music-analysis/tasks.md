## 1. Crate and data model

- [x] 1.1 Create `crates/analysis` (deps: `audio`, `platform`, realfft, rubato, serde, postcard, xxhash-rust, arc-swap); add to the workspace
- [x] 1.2 Define `SongScore` (coverage `IntervalSet`, tempo segments, events, per-beat curves, sections, version) and its query API: `beat_at`, `events_in`, `section_at`, `drop_in`, `tension`
- [x] 1.3 Snapshot publication via `arc-swap`; test that readers never block while a writer publishes

## 2. Front end

- [x] 2.1 Generalize `audio::decode::StereoResampler` to N channels (keep its tests green) and use it for mono 22,050 Hz
- [x] 2.2 Streaming decode with `TrackDecoder` → mono downmix → 22,050 Hz, from any start time
- [x] 2.3 STFT (1024/256 Hann at 22,050 Hz; see design D3) and per-frame features: log-mel 32, centroid, flatness, RMS, chroma, bass/mid/treble
- [x] 2.4 Band onset functions (kick/snare/hat) with adaptive thresholding → events
- [x] 2.5 Synthetic test-signal generator: 4/4 kick/snare/hat patterns at a given BPM, with builds (risers, snare rolls), drops and breakdowns, rendered to WAV

## 3. Rhythm

- [x] 3.1 Tempo: autocorrelation + 125 BPM log-Gaussian prior + octave disambiguation (tests: 124 house, 128 techno, 140 dubstep, 174 DnB)
- [x] 3.2 Robust linear grid fit per window; piecewise tempo segments on residual breakdown (test: 122 → 128 BPM mix)
- [x] 3.3 Tempo confidence and beatless marking (test: 60 s beatless intro)
- [x] 3.4 Downbeat offset and phrase origin

## 4. Structure

- [x] 4.1 Beat-synchronous feature aggregation
- [x] 4.2 Sliding-window self-similarity + checkerboard novelty; peak picking; phrase snapping
- [x] 4.3 Online labeling with prototype matching (test: repeated drop gets the same label)
- [x] 4.4 Section-kind rules with rolling-percentile thresholds (test: synthetic intro/build/drop/breakdown/drop/outro)
- [x] 4.5 Tension curve and `drop_in`; provisional → final marking

## 5. Streaming service

- [x] 5.1 `AnalysisService { playhead, prewarm, score }` with at most two `Priority::Low` workers from the `Spawner`
- [x] 5.2 Rolling horizon (≥ 120 s ahead, then idle), coverage tracking, 30 s drop of unused workers
- [x] 5.3 Seek restart with an 8 s lead-in (test: jump far ahead in a long file)
- [x] 5.4 Verify on a generated 4-min track: first 32 bars covered < 1 s; bounded memory on a generated 2-hour file

## 6. Cache

- [x] 6.1 Sampled xxh3 content hash; `ScoreCache` in `dirs::cache_dir()/winamp_rust/scores` (`WINAMP_CACHE_DIR` override), postcard, algorithm version
- [x] 6.2 Save partial coverage when a worker stops and every 30 s; load before starting a worker (test: second play is instant)

## 7. Player integration (crates/ui)

- [x] 7.1 `WinampApp` owns an `AnalysisService`; map `TrackId` → `TrackRef` from `TrackLoaded`; call `playhead` each frame and `prewarm` on `PreWarm`
- [x] 7.2 Extend `SceneFrame` with `score`; `BeatFlash` flashes on analyzed beats when a score exists
- [x] 7.3 Host keys in fullscreen: `T` timeline strip, `A` annotation mode (`Space`, `1`–`6` while annotating); not forwarded to the scene
- [x] 7.4 Timeline strip: beats, downbeats, section spans by kind with labels (provisional faint), tension, coverage, playhead, "analyzing…" state
- [x] 7.5 Verify with the engine: no start-latency regression (`--bench`) and zero underruns with analysis running on current + pre-warmed tracks

## 8. Evaluation

- [x] 8.1 Annotation capture: clock-corrected times, JSON per track in `<cache>/annotations/`
- [x] 8.2 `analysis-eval <dir>`: per-track and aggregate beat F-measure (±70 ms) and boundary hit rate (±1 bar); tests with synthetic annotated tracks
- [x] 8.3 (needs you) Annotate ~10 of your own tracks (house, techno, DnB, dubstep, one DJ mix) in annotation mode, run `analysis-eval`, and tune rules toward the targets
