## Context

Target music is electronica: steady tempo on a 4/4 grid, changes on 8/16/32-bar phrases, and a recognizable build → drop → breakdown vocabulary. The top product constraint is zero playback latency, so analysis must be fully decoupled and never delay audio. Some inputs are hour-long DJ mixes with tempo drift and track changes.

## Goals / Non-Goals

**Goals:**
- Score coverage of the first 32 bars within ~1 s of play start (cold cache); instant on cached tracks.
- Beat accuracy: F-measure ≥ 0.9 at ±70 ms on the user's annotated electronica set; beat times typically within a few ms thanks to grid fitting.
- Section boundaries within ±1 bar for ≥ 70% of annotated boundaries.
- Bounded memory regardless of file length.
- Deterministic results (same file → same score).

**Non-Goals:**
- ML models / pretrained networks (rules are tunable and explainable; may revisit).
- Genre-general accuracy (jazz, classical rubato).
- Key detection, lyrics, stem separation.
- Real-time detection for live input (the visual engine's live FFT covers texture only).

## Decisions

### D1. Rolling horizon instead of whole-file analysis
```
 file:  |=========================================================|
             ▲ playhead
             │◀─ covered ─▶│◀── worker decoding at 50-200× ──▶
                            target: covered_until ≥ playhead + 120 s, then idle
```
One worker per active track (plus one for the pre-warmed next track) on a `Priority::Low` thread. It decodes via its own `FileSource` handle (never shares the playback decoder). Coverage is an interval set; seeking outside it restarts the worker at the seek point (aligned back ~8 s for context). Alternative (analyze whole file first) rejected: breaks mixes and wastes work on skipped tracks.

### D2. Front end
Mono downmix → resample to 22,050 Hz → STFT (Hann, 2048 window, 512 hop ≈ 23.2 ms). Per frame: 64-band log-mel, spectral centroid (brightness), flatness (noise/risers), RMS loudness, 12-bin chroma. Band onset functions via half-wave-rectified spectral flux with adaptive median threshold:
- kick 40–120 Hz, snare 150–400 Hz + 2–5 kHz noise, hats > 6 kHz.
PCM is discarded after feature extraction.

### D3. Tempo and beat grid
- Tempo: autocorrelation of the combined onset envelope over 60–200 BPM, weighted by a log-Gaussian prior centered at 125 BPM. Octave disambiguation with hat density (16ths at the half-tempo would be implausibly dense → choose double) and kick periodicity.
- Grid: fit `beat_n = t0 + n·period` by robust least squares (Huber) to kick onsets within a window (~32 bars). Electronica is quantized, so a line is far more accurate than frame-by-frame tracking.
- Piecewise tempo: slide the window; when residuals exceed a threshold for ≥ 4 bars, close the tempo segment and fit a new one (handles DJ-mix transitions and drift).
- Confidence: from residual RMS and onset regularity. Below threshold → beatless mode (no grid; energy-only signals).

### D4. Downbeats and phrase grid
Test the 4 bar offsets; score each by kick-pattern repetition across bars, bass/chroma change at bar starts, and novelty peaks at bar starts. The phrase origin (mod 8 bars) is chosen to maximize alignment of detected boundaries.

### D5. Structure
- Beat-synchronous feature vectors (mean of frames per beat).
- Self-similarity over a sliding window of 128 bars; Foote checkerboard kernel (size 8–16 bars) gives a novelty curve; peaks → candidate boundaries; snap to the nearest phrase boundary (8 bars) within ±2 bars, else nearest bar.
- Online labeling: each finalized segment's mean vector is compared to label prototypes (cosine); ≥ threshold → reuse label (A, A'), else new label. Keeps constant memory for mixes.
- Kinds by rules on per-segment stats (thresholds relative to rolling percentiles):

| Kind | Rule |
|---|---|
| drop | kick present ≥ 80% beats ∧ energy ≥ p75 ∧ preceded by build/breakdown |
| build | brightness slope > 0 ∧ onset-density slope > 0 (snare roll) and/or flatness rising |
| breakdown | kick present ≤ 20% ∧ energy < median |
| intro/outro | first/last segment of a track (or of a tempo segment in mixes) with sparse mid energy |
| groove | fallback |

### D6. Tension and drop countdown
`drop_in(beat)` = beats until the next `drop` start (∞ if none known within the horizon). `tension` = ease-in(progress through a build that precedes a drop) × normalized riser strength (brightness + flatness + onset-density rise), 0 elsewhere, dropping to 0 at the drop.

### D7. SongScore and finalization
The score is append-only in time: regions near the horizon edge are provisional; once the horizon has moved 32 bars past a region, its sections finalize. Consumers read through a snapshot API (`Arc` swap) — no locks on the render path.
```
SongScore { version, track_hash, coverage: IntervalSet,
  tempo_segments: [{start_s, t0, period, confidence}], downbeat_offset, phrase_origin,
  events: {kick, snare, hat, downbeat}: sorted Vec<f64 seconds>,
  curves (per beat): energy, bass, mid, treble, brightness, flatness, tension,
  sections: [{start_beat, end_beat, label, kind, final}] }
```

### D8. Cache
Key = fast content hash: file size + xxh3 of first/middle/last 256 KB (full blake3 rejected: too slow for multi-GB mixes before playback-adjacent work). Stored as postcard in the platform cache dir, versioned with an analysis algorithm version (bump → invalidate). Partial scores are cached too (coverage preserved).

### D9. Budgets (4-min track, M-series Mac)
Decode ~0.5–1 s total, STFT/features ~50–100 ms, grid/SSM/rules ~20 ms — all streamed, first 32 bars ready in < 1 s.

### D10. Debug & evaluation
- Timeline strip (toggle key in fullscreen): beats, downbeats, section spans colored by kind, labels, tension curve, playhead.
- Annotation capture: while listening, keys mark "beat tap" and "boundary here + kind"; saved as JSON next to the cache.
- `analysis-eval` CLI: runs the analyzer on annotated tracks and prints beat F-measure and boundary hit rate.

## Risks / Trade-offs

- [Half/double tempo errors on DnB/dubstep] → Prior + hat density; annotations will reveal failures; allow per-track manual tempo halve/double override.
- [Rule thresholds overfit to a few tracks] → Percentile-relative thresholds; evaluate across the annotated set, not single tracks.
- [Provisional sections change after being displayed] → Director only acts on sections marked final or on those with high novelty confidence; overlay ticks fade in on finalization.
- [Sampled hash collisions] → Include file size and three regions; acceptable for a local cache.
- [Mix transitions mislabeled as drops] → Tempo segment boundary resets kind context (treated as new "track" for intro/outro rules).

## Open Questions

- Exact tension shaping curve — tune by eye with the visual engine.
- Whether to expose per-track manual overrides (tempo ×2/÷2, phrase shift) in the UI for MVP.
