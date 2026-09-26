## 1. Crate and data model

- [ ] 1.1 Create `crates/analysis`; define `SongScore`, `IntervalSet` coverage, events, curves, sections, versioning
- [ ] 1.2 Snapshot publication (atomic `Arc` swap) and query API: beat phase at time, events in range, section at beat, `drop_in`, `tension`

## 2. Front end

- [ ] 2.1 Streaming decode via `FileSource` → mono → 22,050 Hz resample
- [ ] 2.2 STFT (2048/512 Hann) and per-frame features: log-mel 64, centroid, flatness, RMS, chroma
- [ ] 2.3 Band onset functions (kick/snare/hat) with adaptive thresholding → events
- [ ] 2.4 Synthetic test signals (click tracks, generated 4/4 patterns) for unit tests

## 3. Rhythm

- [ ] 3.1 Tempo estimation: autocorrelation + 125 BPM log-Gaussian prior + octave disambiguation
- [ ] 3.2 Robust linear grid fit per window; piecewise tempo segments on residual breakdown
- [ ] 3.3 Tempo confidence and beatless marking
- [ ] 3.4 Downbeat offset selection and phrase origin

## 4. Structure

- [ ] 4.1 Beat-synchronous feature aggregation
- [ ] 4.2 Sliding-window self-similarity + checkerboard novelty; peak picking; phrase snapping
- [ ] 4.3 Online labeling with prototype matching
- [ ] 4.4 Section kind rules with rolling-percentile thresholds
- [ ] 4.5 Tension curve and `drop_in`; provisional → final marking

## 5. Streaming orchestration

- [ ] 5.1 Rolling-horizon worker (low priority), coverage tracking, idle when ≥ 120 s ahead
- [ ] 5.2 Seek restart with context lead-in
- [ ] 5.3 Pre-warm hook: start next-track analysis on engine event
- [ ] 5.4 Verify: no start-latency regression and zero underruns with analysis on

## 6. Cache

- [ ] 6.1 Sampled xxh3 content hash; postcard cache in platform cache dir with algorithm version
- [ ] 6.2 Persist partial coverage; load on play before starting workers

## 7. Debug and evaluation

- [ ] 7.1 Timeline strip widget (beats, downbeats, sections, labels, tension, coverage, playhead)
- [ ] 7.2 Annotation capture keys and JSON format (clock-corrected times)
- [ ] 7.3 `analysis-eval` CLI: beat F-measure ±70 ms, boundary hit rate ±1 bar
- [ ] 7.4 Annotate ~10 of the user's tracks (house, techno, DnB, dubstep, one DJ mix) and tune to targets
