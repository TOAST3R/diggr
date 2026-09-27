## Context

Target music is electronica: steady tempo on a 4/4 grid, changes on 8/16/32-bar phrases, and a recognizable build → drop → breakdown vocabulary. The top product constraint is zero playback latency, so analysis must be fully decoupled and never delay audio. Some inputs are hour-long DJ mixes with tempo drift and track changes.

What exists (merged):
- `audio::decode::TrackDecoder::{open(files, track), decode_next(&mut Vec<f32>) → source-rate interleaved stereo, seek(secs), src_rate(), info()}`.
- `audio::decode::StereoResampler` (rubato FFT, exact output length).
- Engine events: `TrackLoaded { id: TrackId, index, track: TrackRef }` for every playback instance, and `PreWarm { index, track }` about 30 s before a track ends. `Position { track: TrackId, frame, sample_rate, state }` from the clock.
- `platform::Spawner` with `Priority::Low`, and `platform::FileSource`.
- `ui::fullscreen::{VisualScene, SceneFrame}`: the host routes transport keys itself and all other keys to the scene.
- `ui::settings::Store` (config dir, RON, atomic writes).

## Goals / Non-Goals

**Goals:**
- Score covers the first 32 bars within ~1 s of play start with a cold cache; instant on cached tracks.
- Beats: F-measure ≥ 0.9 at ±70 ms on the user's annotated electronica set.
- Section boundaries within ±1 bar for ≥ 70% of annotated boundaries.
- Bounded memory regardless of file length; deterministic results.
- Zero effect on playback: no start-latency regression, zero underruns with analysis running.

**Non-Goals:**
- ML models, genre-general accuracy, key detection, lyrics, stems.
- Live-input analysis (the visual engine's live FFT covers texture).
- The visual engine itself (signals, director): `visual-engine` consumes the score.

## Decisions

### D1. Service API owned by the app
```rust
pub struct AnalysisService { /* workers, cache, snapshots */ }
impl AnalysisService {
    pub fn new(spawner: Arc<dyn Spawner>, files: Arc<dyn FileSource>, cache: Option<ScoreCache>) -> Self;
    /// Called every UI frame with the audible track and seconds (cheap: atomics + a map lookup).
    pub fn playhead(&self, track: &TrackRef, secs: f64);
    /// Start analyzing a track that is about to play (EngineEvent::PreWarm).
    pub fn prewarm(&self, track: &TrackRef);
    /// Latest immutable snapshot for a track, if any analysis exists.
    pub fn score(&self, track: &TrackRef) -> Option<Arc<SongScore>>;
}
```
The UI maps the clock's `Position.track` (a `TrackId`) to a `TrackRef` using `TrackLoaded` events, which it already receives. It calls `playhead` each frame and `prewarm` on `PreWarm` events. Scores are keyed by `TrackRef` in memory and by content hash on disk. Alternative (analysis subscribing to the engine directly) rejected: the engine would depend on analysis, and the app already owns the event stream.

### D2. Rolling horizon
```
 file:  |=========================================================|
             ▲ playhead
             │◀─ covered ─▶│◀── worker decoding at 50-200× ──▶
                            target: covered_until ≥ playhead + 120 s, then idle
```
- At most two workers: the audible track and the pre-warmed one. Each is a `Priority::Low` thread from the `Spawner` with its own `TrackDecoder`.
- Coverage is an interval set. A playhead outside coverage restarts the worker there, with an 8 s lead-in for context.
- Workers drop (and cache what they have) when their track is neither audible nor pre-warmed for 30 s.
- Alternative (analyze whole file first) rejected: breaks mixes and wastes work on skipped tracks.

### D3. Front end
`TrackDecoder` (source-rate stereo) → mono downmix → resample to 22,050 Hz (`audio::decode::ChannelResampler`, the playback resampler generalized to N channels) → STFT (Hann, **1024 window, 256 hop ≈ 11.6 ms**; the planned 2048/512 gave 23 ms frames, too coarse for the ±20 ms kick requirement). Per frame: 32-band log-mel, spectral centroid (brightness), flatness, RMS, 12-bin chroma, bass/mid/treble energy. At the start of a file half a window of silence is prepended so a kick at t = 0 can be located. Band onsets by half-wave-rectified spectral flux with an adaptive median threshold: kick 40–120 Hz, snare 150–400 Hz + 2–5 kHz noise, hats > 6 kHz. PCM is discarded after feature extraction.

### D4. Tempo and beat grid
- Tempo: autocorrelation of the onset envelope over 60–200 BPM × a log-Gaussian prior centered at 125 BPM. Octave disambiguation by hat density and kick periodicity (DnB at 174 must not read as 87).
- Grid: robust (Huber) least-squares fit of `beat_n = t0 + n·period` to kick onsets over ~32-bar windows. Electronica is quantized, so a line beats frame-by-frame tracking.
- Piecewise tempo: when residuals exceed a threshold for ≥ 4 bars, close the segment and fit a new one (DJ-mix transitions, drift).
- Confidence from residual RMS and onset regularity; below threshold → beatless (no grid; energy only).

### D5. Downbeats and phrase grid
Score each of the 4 bar offsets by kick-pattern repetition, bass/chroma change at bar starts, and novelty peaks at bar starts. The phrase origin (mod 8 bars) maximizes alignment of detected boundaries.

### D6. Structure
- Beat-synchronous feature vectors; self-similarity over a sliding 128-bar window; Foote checkerboard novelty (8–16 bars); peaks snapped to the nearest phrase boundary within ±2 bars, else the nearest bar.
- Online labels: each finalized segment's mean vector is matched (cosine) against label prototypes; reuse a label above a threshold, else create one. Constant memory for mixes.
- Kinds by rules on per-segment stats, thresholds relative to rolling percentiles:

| Kind | Rule |
|---|---|
| drop | kick present ≥ 80% of beats ∧ energy ≥ p75 ∧ preceded by build/breakdown |
| build | brightness slope > 0 ∧ onset-density slope > 0 (snare roll) and/or flatness rising |
| breakdown | kick present ≤ 20% ∧ energy < median |
| intro/outro | first/last segment of a track (or of a tempo segment in mixes) with sparse mid energy |
| groove | fallback |

### D7. Tension and drop countdown
`drop_in(beat)` = beats until the next `drop` start (`None` if none known within the horizon). `tension` = ease-out(progress through a build that precedes a drop), i.e. `1 − (1 − p)²`, 0 elsewhere, falling to 0 at the drop. (Ease-out rather than the planned ease-in, so it is near its peak for the last bars, as the spec's "8 beats before the drop → near maximum" requires; riser strength is not multiplied in yet.)

### D8. SongScore, snapshots and finalization
```rust
pub struct SongScore {
    pub version: u32, pub content_hash: u64, pub coverage: IntervalSet /* seconds */,
    pub tempo_segments: Vec<TempoSegment /* start_s, t0, period, confidence */>,
    pub downbeat_offset: u8, pub phrase_origin: u32,
    pub events: Events /* kick, snare, hat, downbeat: sorted Vec<f64> seconds */,
    pub beats: BeatCurves /* per beat: energy, bass, mid, treble, brightness, flatness, tension */,
    pub sections: Vec<Section /* start_beat, end_beat, label, kind, final */>,
}
impl SongScore { fn beat_at(&self, secs) -> Option<f64>; fn events_in(&self, kind, a, b); fn section_at(&self, beat); fn drop_in(&self, beat); fn tension(&self, beat); }
```
Workers publish a new `Arc<SongScore>` roughly every few seconds of analyzed audio through an atomic swap (`arc-swap`), so readers on the UI/render thread never block. Regions within 32 bars of the coverage edge are provisional; sections finalize once the horizon moves past them.

### D9. Cache
- Key: file size + xxh3 of the first, middle and last 256 KB (full hashing is too slow for multi-GB mixes).
- Location: `dirs::cache_dir()/winamp_rust/scores/<hash>.postcard`; `WINAMP_CACHE_DIR` overrides.
- Stamped with an algorithm version (bump → ignored and regenerated). Partial coverage is cached too, and written when a worker stops or every 30 s.

### D10. Fullscreen integration (modifies `fullscreen-mode`)
- `SceneFrame` gains `score: Option<&SongScore>`. The placeholder `BeatFlash` switches from its fixed 120 BPM grid to real beats when a score is present: a quick check that the grid is right.
- The host keeps `T` (timeline strip) and `A` (annotation mode) for itself; while annotating it also keeps `Space` (beat tap) and `1`–`6` (boundary of kind intro/build/drop/breakdown/groove/outro). Everything else still goes to the scene.

### D11. Debug and evaluation
- **Timeline strip** (`T`), drawn by the host along the bottom of fullscreen over any scene: beats, downbeats, section spans colored by kind with labels, the tension curve, coverage, and the playhead.
- **Annotations** (`A`): taps and boundaries are stamped with the clock's audible position (so output latency is already compensated) and saved to `<cache>/annotations/<hash>.json` with the track path.
- **`analysis-eval <dir>`** (`cargo run -p analysis --bin analysis-eval`): analyzes every annotated track, prints per-track and overall beat F-measure (±70 ms) and boundary hit rate (±1 bar).

### D12. Budgets (4-min track, M-series Mac)
Decode ~0.5–1 s, STFT/features ~50–100 ms, grid/SSM/rules ~20 ms, all streamed: first 32 bars in < 1 s.

## Implementation notes

What was built (crate `analysis`), and what changed from the plan above.

- **Modules:**
  - the pipeline: `frontend` (STFT features) → `onsets` → `rhythm` (tempo, grid, segments, downbeats) → `structure` (novelty, boundaries, kinds, labels, tension);
  - streaming: `region` (incremental analysis of one contiguous stretch), `assemble` (joins regions and a cached prefix into one `SongScore`), `source` (decoding), `service` (`AnalysisService` and workers);
  - storage and tuning: `cache`, `eval` (annotations and metrics), `synth` (ground-truth test music);
  - binary: `analysis-eval`.
- **Onsets:**
  - A peak must exceed a local median and a share of the band's **±30 s level**: 45% for kicks, 25% for snares and hats. A ±2 s level let weak bumps in kick-less builds and breakdowns through, and a lower kick ratio let basslines (same band) read as kicks.
  - A measured 5 ms detection bias is compensated.
  - On the synthetic test track: 224/224 kicks, worst error 7 ms.
- **Tempo segments:**
  - 16 s windows every 4 s, each fitted with a robust straight-line grid; windows that agree merge.
  - Windows whose timing error is above 2.5× the median (minimum 3 ms) are dropped. That is how a tempo change is found: straddling windows fit at 4–8 ms, steady ones at ~1 ms.
  - The coarse autocorrelation tempo only seeds the fit (within 1%); the fitted grid is what the score reports.
- **Grid through breakdowns:** electronica keeps tempo when the kick stops. An open region carries its last grid forward to its analyzed end (provisional), and a region that starts mid-file (a seek) carries its first confident grid back to its start. Without this, a seek into a breakdown left ~20 s without beats. A beatless intro at the start of a file still gets no grid.
- **Boundaries:** novelty peaks are chosen by **prominence** (height above the lowest novelty within ±4 bars), not by a global mean + ½ std threshold. The global threshold hid real but weaker boundaries (breakdown → build, drop → outro) behind very strong drop boundaries.
- **Streaming without copying:** a region keeps one growing result and each update truncates only its provisional tail. The first version cloned all final data every 8 s, which made memory grow with length (122 MB after an hour); now it is flat (~29 MB).
- **Cache ordering:** a complete score is written to disk *before* it is published, so anyone who sees `complete` (e.g. the next app start) finds it cached. The reverse order was a race that made a test hang about 1 in 8 runs.
- **Player integration (crates/ui):**
  - `WinampApp` owns the service, maps `TrackId` → `TrackRef` from `TrackLoaded`, calls `playhead` every frame, and calls `prewarm` on `PreWarm`.
  - `SceneFrame.score` carries the score, and `BeatFlash` flashes on analyzed beats.
  - Host keys are a pure function (`host_action`).
  - The strip is `ui::timeline`.
  - Annotations use the clock's audible position and are saved on every mark.
- **Where things are stored:** scores in `~/Library/Caches/winamp_rust/scores/`, annotations in `…/annotations/` (`WINAMP_CACHE_DIR` overrides).

## Verification

- **Automated:** `cargo test -p analysis`: 36 tests (plus 2 long ones, see below), all synthetic with exact ground truth; 150 across the workspace. The analysis crate is optimized even in dev builds (workspace `Cargo.toml`). They cover:
  - kicks within 20 ms;
  - tempo at 124/128/140/174 BPM;
  - beats within 10 ms;
  - the 122→128 split;
  - a beatless intro;
  - downbeats;
  - boundaries on the exact bars;
  - all section kinds at 124/128/140;
  - repeated drops sharing a label;
  - tension;
  - streaming = one-shot;
  - provisional/final;
  - seek joins;
  - the cache round trip;
  - pre-warm;
  - the horizon;
  - evaluation.
- **Speed targets** (`-- --ignored timing`, run alone, release): first 32 bars in 525–607 ms, seek coverage in 86–144 ms (targets < 1 s). They are separate because analysis runs at minimum OS priority by design, and under a full parallel test run it is (correctly) starved.
- **2-hour mix** (`-- --ignored two_hour`): 43 tracks, 120 min, frames held ≤ 7,918 (limit 9,296), 15,136/15,136 beats, 43 tempo segments, peak memory 90 MB for the whole test process including rendering.
- **Per-frame UI calls:** p99 < 1 ms.
- **UI:** `crates/ui/tests/analysis_playback.rs`: real-time playback while the current and the next track are analyzed, 0 underruns. Also headless tests of the strip, host keys and flash-on-beats.
- **Real Mac** (`--bench --analysis`, muted, 5 × 60 s files):
  - with analysis: worst start 20.9 ms, worst seek 21.1 ms, 0 underruns over 300 s, 300 s analyzed;
  - without analysis: 21.9 ms, 21.6 ms, 0 underruns;
  - no regression.
- **`analysis-eval`** on generated annotated tracks (house 124, DnB 174): 100% beat F, 100% boundaries.
- **First real track** ("SUBLIMINAL CHAOS", your own production, 138 BPM; 29 beat taps, 1 drop mark):
  - The beat grid is right: 138.0 BPM, beat F 89.7%. The misses are the first taps of each tapped stretch (+88 ms, +129 ms human warm-up).
  - This exposed an evaluation bug, now fixed: beats in a *pause* between tapped stretches were counted as wrong (it had reported 55.9%).
  - **Section kinds are not reliable yet on real music.** No drop was detected anywhere, the marked drop at 77.4 s was found as a boundary (77.35 s) but classified as "build", and almost every section got label A. The rules were tuned on synthetic music. **Decision (task 8.3 closed):** the user's tracks do not follow the intro/build/drop/breakdown/groove/outro structure, so rule tuning on section annotations was skipped. The user judged the section *changes* visually correct in fullscreen, and boundary positions are reliable (the marked change was found within 0.1 s). Section *kinds* stay heuristic; the visual engine should key on boundaries, energy and tension rather than trust `drop`/`build` labels.
- **Not yet verified:**
  - **accuracy on real music**, which is task 8.3 and needs annotations of your own tracks;
  - the strip and annotation keys in the live fullscreen window (the logic and drawing are tested headless, but they haven't been looked at on screen).

## Risks / Trade-offs

- [Half/double tempo on DnB/dubstep] → prior + hat density; annotations reveal failures; per-track manual ×2/÷2 override later.
- [Thresholds overfit to a few tracks] → percentile-relative thresholds; evaluate across the whole annotated set.
- [Provisional sections change after display] → the director only acts on final sections or a stable `drop_in`; the strip shows provisional sections faintly.
- [Sampled hash collisions] → size + three regions; acceptable for a local cache.
- [Mix transitions read as drops] → a tempo-segment boundary resets kind context.
- [Synthetic tests pass but real music fails] → synthetic signals (generated 4/4 patterns with builds and drops) cover the mechanics; accuracy targets are only claimed after the user annotates real tracks (task 7.4).

## Open Questions

- Exact tension shaping curve: tune by eye with the visual engine.
- Per-track manual overrides (tempo ×2/÷2, phrase shift) in the UI for MVP?
