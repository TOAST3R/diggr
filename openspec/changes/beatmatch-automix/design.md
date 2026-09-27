## Context

- **Engine today:** one decode worker produces one continuous stream (decoder → fixed-ratio FFT resampler → ring). It can splice at an exact output frame (scheduled jumps and loops from `waveform-navigation`). The audio callback applies EQ, tap and volume. The clock switches track id at a gapless boundary, and a splice number marks jumps without a flush.
- **Analysis:** it produces `SongScore`s (beats, downbeats, phrase origin, sections with per-beat energy in dB, tempo) ahead of the playhead, for the current track and a pre-warmed next one. `analysis` depends on `audio`, so the engine can never see a score.
- **Player:** it already drives the analysis playhead and pre-warm, and has a waveform section with a zoomed row and a track overview.

## Goals / Non-Goals

**Goals:**
- A playlist plays like a DJ set: beat-matched, phrase-aligned, EQ'd, no bass clash, no loudness jump, and no long intros or outros on their own.
- One held set tempo, vinyl-style.
- Zero risk to real-time safety: mixing happens in the decode worker, and the callback gains only a speed factor for the clock.
- Never worse than today: every failure falls back to a crossfade or the gapless cut.

**Non-Goals:**
- Key-lock (tempo without pitch) and harmonic (key) mixing.
- Manual DJ controls (jog, pitch fader, per-band knobs).
- Reordering the playlist by key or tempo.

## Decisions

### D1. The player plans, the engine executes
```
 analysis::mixplan::plan(score_a, score_b, set_bpm, settings) ──▶ audio::MixPlan (plain data)
                 ▲                                                        │
   ui orchestrates: SYNC on, next track pre-warmed, both scores ready ────┘──▶ engine.schedule_mix(plan)
```
`MixPlan` holds everything in frames and plain numbers:
- the incoming queue index and B's entry point (seconds of B);
- A's mix-start (seconds of A) and the mix length (A output frames);
- the handover offset and B's tempo ratio;
- B's gain (loudness match), and the envelope shape parameters.

The engine validates the plan (A's mix-start not yet sent) and executes it. It knows nothing about scores, so it is testable with hand-written plans. If the plan can no longer be met, the engine reports `MixMissed` and the player replans or falls back.

### D2. Two decks in the worker
```
 A (outgoing): decoder → resampler (A's ratio) → isolator(lo,mid,hi) × gains ─┐
                                                                             ├─ Σ → ring → callback (unchanged)
 B (incoming): pre-warmed decoder, seeked to entry → variable resampler (r) → isolator × gains ─┘
```
- **Deck B's resampler:** B uses rubato's asynchronous (sinc) resampler, whose ratio can be adjusted at runtime. It carries B's tempo ratio times the source→output rate ratio, plus drift nudges (D3). A deck at ratio 1 keeps the cheaper FFT resampler.
- **Isolator:** Linkwitz–Riley crossovers at 200 Hz and 2.5 kHz (as in the track overview), with per-band gains ramped per sample.
- **During a mix:** the worker renders both decks for each segment and sums them, and the tap carries the mixed, audible signal.
- **After the mix:** deck A is dropped and B carries on as the only deck, keeping its ratio (D3).

### D3. Held set tempo, vinyl-style
- **Set tempo:** the tempo the listener hears, i.e. the current track's analyzed BPM × its playback ratio. The first track of a session plays at ratio 1, so the set tempo starts at its own BPM.
- **Ratio for B:** `r = set_bpm / bpm_B`, allowing half and double time: take the one of {set/B, 2·set/B, set/(2·B)} closest to 1. It must lie within [0.92, 1.08].
- **Held:** B keeps `r` for its whole length, with pitch following tempo like vinyl. There is no glide back.
- **Out of range: skip.** In SYNC mode, a next track with no ratio in range is skipped. The player looks further down the queue, following the repeat and shuffle order, and mixes into the first track that can be matched. Each skip is shown briefly. Only tracks with a known tempo count as out of range: a track not analyzed yet is analyzed on pre-warm like any other.
- **Nothing matches:** if no remaining track can be matched, the next one is crossfaded at ratio 1 (D7), and the set tempo becomes its BPM.
- **User choice wins:** a track the user starts is never skipped.
- **Drift:** during the mix, the worker compares both decks' next downbeats (from the plan's grids, passed as a short list of downbeat times) and nudges B's ratio by at most ±0.1% to keep kicks within 5 ms. A grid needing more is treated as unreliable (D7).
- **Pitch:** at 124→126 BPM it shifts +0.3 semitones, which is hard to hear. At the ±8% cap it is about 1.3 semitones. Accepted (non-goal: key-lock).

### D4. Where to mix: the best moment, not the end
Section energy is the mean of `curves.energy` (dB) over the section. A "rise" is a boundary at least 4 dB louder than the previous section.
- **Mix length L:** the multiple of 8 bars at the set tempo closest to the setting (30 s by default), at least 8 and at most 32 bars. At 124–128 BPM that is 16 bars; at 174 BPM it is 16 bars (22 s), since 32 bars would be 44 s.
- **B's main start `M_B`:** the first rise in B (its first drop). If there is none, the start of B's second section. If B has no sections yet, its first downbeat + 16 bars.
- **B's entry `E_B = M_B − L`,** clamped to B's first downbeat after leading silence. B's intro before `E_B` is skipped. The mix ends as B's main part starts, and the handover (lows swap) falls at `E_B + L/2`.
- **A's mix start `O_A`:**
  - It is the start of A's outro: the first boundary after A's last rise where energy falls by at least 4 dB, rounded to A's phrase grid (32 beats from `phrase_origin`).
  - It needs L bars of A left, and must be at least 16 bars after A's last rise started, so the peak is heard.
  - With no clear outro, `O_A` is the latest phrase boundary that leaves L bars.
  - A's outro after `O_A + L` is never played.
- **Who starts playing where:** only tracks that arrive through a mix skip their intro. A track the user starts (play, double-click, `B`) plays from its start.

### D5. Volume and EQ
With `t` running 0 → 1 over the mix:
- **Volume:** an equal-power crossfade, `g_A = cos(πt/2)` and `g_B = sin(πt/2)`. A is louder in the first half (≥ 6 dB above B at t = 0.25), they are equal at the middle, B is louder after, and A is silent at the end.
- **Loudness match:** B's gain is multiplied by the difference in the tracks' analyzed loudness (mean energy of A's section before `O_A` versus B's main section), capped at ±6 dB. At its end the mix ramps that correction out over 4 bars, so B plays at its own level after the mix. The first track's level is the reference; no normalization happens outside mixes.
- **Bass and kick (low band < 200 Hz):**
  - B's lows are cut to −26 dB from its entry until the handover.
  - At the handover downbeat, the lows swap within one beat: B's lows come up to 0 dB and A's go down to −26 dB. A's kick and bass are muted from then on.
- **Highs:** A's high band (> 2.5 kHz) follows `g_A` one bar ahead, so its hats thin out slightly before its body.
- **Manual swap (`E`):** during a mix, `E` swaps the low bands on the next beat of the outgoing track, using the same one-beat ramp. Pressing it again swaps them back. Once the user has swapped, the automatic swap at the handover is cancelled.
  - The engine gets `swap_lows()`; the worker picks the next beat from the plan's beat list that hasn't been sent to the device yet.
  - The clock's handover (main track switch) stays at the midpoint either way.

### D6. Pre-warm and planning timing
- **Pre-warm:** as soon as A's analysis covers A's end, the player computes a provisional `O_A`. The engine pre-warms the next track (new `Engine::prewarm_next`) at least 60 s before `O_A` or 30 s before A's end, whichever is earlier, and the analysis service pre-warms it too.
- **Planning:** the plan is computed when B's analysis covers `M_B` and A's covers `O_A + L`. It is rescheduled if either score changes the points before `O_A` has been sent to the device.
- **Too late:** if no plan is ready 2 s before `O_A`, the player tries the next phrase boundary. If none fits, the transition becomes today's gapless cut at A's end (D7).

### D7. Fallbacks
Each fallback is reported as `MixFallback { reason }` and shown for 3 s in the main window:
1. **A grid unreliable at the mix points, or no track in the rest of the queue can be matched:** an equal-power crossfade of L bars at A's `O_A`, ratio 1, with the same EQ swap. Out-of-range tempos are skipped first (D3).
2. **No analysis in time, a track shorter than L, repeat One, or the last track with repeat Off:** the plain gapless cut.

### D8. Clock during and after a mix
- **Main track:** the clock reports A until the handover downbeat is audible, then B in B's own time. The worker attributes output frames per deck, as for a gapless boundary.
- **Speed:** each ring segment gets a `speed` (the deck's tempo ratio, 1.0 normally). The renderer copies it into the clock snapshot, and readers interpolate `frame + offset × speed`. `Position.seconds()` therefore advances at B's own musical time, so beats, triggers and analysis stay locked.
- **Continuity:** the track switch is a boundary, not a discontinuity, so the visuals see a track change at the handover and their crossfade lands there.
- **The other deck:** the player computes the other deck's position from the plan (`b = E_B + (a − O_A) · r`, and the reverse after the handover), for the waveform.

### D9. Transport during a mix
- `B`: cuts straight to the incoming track at its current position; A stops.
- `E`: swaps the low bands by hand (D5).
- `Shift+B`: plans a mix starting at the next phrase boundary of the current track at least one bar ahead, with B's entry per D4, even when SYNC is off.
- **Seek:** cancels the mix and seeks within the main track.
- **Pause / stop:** they apply to both decks, which pause together because there is one stream.
- **Turning SYNC off:** before a mix starts, this cancels it. During a mix, the mix completes.

### D10. SYNC button and indicator
- **Button:** a SYNC toggle in the main window, in the skin's style (new sprites from the skin generator). It lights when on, and its state persists in the settings. Its position is decided while implementing: the main window's button row is nearly full, so the layout must be checked (next to REP, the left end of that row, or near the EQ/PL toggles).
- **Indicator:** while a mix is planned or running, the time display area shows `MIX 0:12`, the time until the mix starts or until the handover.

### D11. Two waveforms while mixing
- **Two lanes:** from the moment B is pre-warmed for a planned mix until the mix ends, the waveform's zoomed row splits into two lanes: A (outgoing) on top, B (incoming) below.
- **Aligned grids:** each lane is centered on its own audible position and drawn at the set tempo (B's time axis scaled by `r`), so both beat grids line up vertically.
- **Mix state:** the mix region is shaded, the handover is a vertical line, and a lane whose low band is cut shows its red (bass) columns dimmed. Each lane's brightness follows its gain.
- **Overview row:** the overview row keeps showing the main track, with the mix region marked.

## Risks / Trade-offs

- [Two decks double the decode and resample load, and the sinc resampler is costlier] → Decoding runs ~100× real time; measure underruns with two 48 kHz FLAC decks in the engine test harness.
- [Wrong grids or sections in places] → Confidence checks at the mix points, drift capped at 0.1%, and fallbacks. The planner only moves points onto whole phrases.
- [Skipping intros and outros removes something the listener wanted] → It happens only in SYNC mode, only for tracks arriving through a mix, and `B` / double-click still play a track from the start.
- [Held tempo drifts the set away from some tracks] → The ±8% cap. Beyond it, a crossfade resets the set tempo to the new track.
- [Clock speed factor in the callback] → One `f32` per segment, copied into the snapshot; no allocation, no locks. Covered by the real-time allocation test.

## Open Questions

- The SYNC button's exact position in the main window (D10).
