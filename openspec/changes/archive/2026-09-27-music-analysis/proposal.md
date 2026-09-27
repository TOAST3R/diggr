## Why

The visualizer must lock to the rhythm and change when the music's structure changes (build, drop, breakdown). Live-only detection is always late and cannot anticipate a drop. Because we play local files, we can analyze ahead of the playhead — giving exact beat times, phrase-aligned section boundaries, and "time until next drop" — without ever delaying playback.

The player (`audio-core`, `classic-ui`) is now built, so this change targets its real interfaces: `audio::decode::TrackDecoder`, the engine's `TrackLoaded` / `PreWarm` events and per-load `TrackId`s, the `platform::Spawner` seam, and the fullscreen `VisualScene` host.

## What Changes

- New `crates/analysis`: a rolling-horizon streaming analyzer. It decodes with its own `TrackDecoder` on `Priority::Low` threads and stays ≥ 2 minutes ahead of the audible position; the same code handles 4-minute tracks and multi-hour DJ mixes.
- Rhythm: band onsets (kick / snare / hats), tempo with an electronica prior, straight-line beat-grid fitting with piecewise tempo segments, downbeats and an 8-bar phrase grid, tempo confidence, beatless fallback.
- Structure: beat-synchronous features, windowed self-similarity novelty, phrase-snapped boundaries, online A/B/A' labels, section kinds (intro, build, drop, breakdown, groove, outro), `tension` and `drop_in` countdown.
- `SongScore`: versioned, immutable snapshots published lock-free; cached on disk by content hash, so a second play is instant.
- `AnalysisService`: the app-facing API. The player tells it which track is audible and where (per frame), which track is pre-warmed (from `EngineEvent::PreWarm`), and asks for the score of the audible track (mapped from `TrackId` via `EngineEvent::TrackLoaded`).
- Fullscreen host extension: `SceneFrame` also carries the audible track's `SongScore`, so the visual engine can read beats, sections and `drop_in`.
- Debug and tuning tools: a timeline strip drawn by the fullscreen host (`T`), an annotation mode (`A`) to mark beats and boundaries on your own tracks, and an `analysis-eval` command that scores the analyzer against those annotations.

## Capabilities

### New Capabilities
- `streaming-analyzer`: service API, rolling horizon, low-priority scheduling, seek handling, pre-warm, cache, coverage, lock-free snapshots.
- `rhythm-analysis`: onsets, tempo, beat grid, downbeats, phrase grid, confidence, beatless fallback.
- `structure-analysis`: boundaries, labels, section kinds, tension, drop countdown.
- `analysis-debug`: timeline strip, annotation capture, evaluation command.

### Modified Capabilities
- `fullscreen-mode`: the host passes the audible track's score to the scene, and keeps `T` / `A` (plus annotation keys while annotating) for itself instead of routing them to the scene.

## Impact

- New crate `crates/analysis` (depends on `audio` for `TrackDecoder`, `platform` seams, realfft, rubato, serde/postcard, xxhash-rust); new binary `analysis-eval`.
- `crates/ui`: owns an `AnalysisService`; feeds it engine events and the clock position; extends `SceneFrame`; draws the timeline strip and handles annotation keys in fullscreen.
- New on-disk cache: `~/Library/Caches/winamp_rust/` (platform cache dir; `WINAMP_CACHE_DIR` overrides) holding scores and annotations. The web target later swaps this for IndexedDB.
- No change to the playback path: analysis never shares threads, decoders or locks with it.
