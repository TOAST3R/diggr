## Why

The visualizer must lock to the rhythm and change when the music's structure changes (build, drop, breakdown). Live-only detection is always late and cannot anticipate a drop. Because we play local files, we can analyze ahead of the playhead — giving exact beat times, phrase-aligned section boundaries, and "time until next drop" — without ever delaying playback.

## What Changes

- New `crates/analysis`: a rolling-horizon streaming analyzer that decodes independently of playback on a low-priority thread and stays ≥ 2 minutes ahead of the playhead. Same code path for 4-minute tracks and multi-hour DJ mixes.
- Rhythm: band onset detection (kick / snare / hats), tempo estimation with an electronica prior, straight-line beat grid fitting with piecewise tempo segments, downbeat and phrase-grid detection, tempo confidence.
- Structure: beat-synchronous features, windowed self-similarity novelty, phrase-snapped boundaries, online A/B/A' labeling, section kinds (intro, build, drop, breakdown, groove, outro), `tension` curve and `drop_in` countdown.
- `SongScore`: versioned, queryable result with events, curves, and sections; cached by file hash (second play is instant).
- Seek-aware: analysis restarts at the seek position; a "coverage" API tells consumers where the score is valid.
- Next-track pre-warm integration: the next track's score starts before it plays.
- Debug timeline strip and an annotation/evaluation harness to tune rules on the user's own tracks.

## Capabilities

### New Capabilities
- `streaming-analyzer`: rolling horizon, scheduling/priority, seek handling, pre-warm, caching, coverage.
- `rhythm-analysis`: onsets, tempo, beat grid, downbeats, phrase grid, confidence, beatless fallback.
- `structure-analysis`: boundaries, labels, section kinds, tension, drop countdown.
- `analysis-debug`: timeline strip, annotation capture, evaluation metrics.

### Modified Capabilities
<!-- none -->

## Impact

- New crate `analysis` (depends on `platform` seams, symphonia, realfft, rubato).
- Consumes `audio-core` pre-warm events and TrackRef; produces `SongScore` consumed by `visual-engine` (signal bus, director) and `classic-ui`/overlay (section ticks on progress bar).
- Adds an on-disk cache directory (native) / IndexedDB store (web, via `web-target`).
