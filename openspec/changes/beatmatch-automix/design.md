## Context

- **Engine today:** one decode worker produces one continuous stream (decoder → resampler → ring). The RT callback applies EQ, tap and volume. Gapless transitions switch decoders in the worker, and the clock switches track id at the exact audible boundary. The next track is pre-warmed 30 s before the end.
- **Analysis:** it produces `SongScore`s (beats, downbeats, phrase origin, sections with energy, tempo segments) ahead of the playhead, and pre-warmed tracks get their first 32 bars within about half a second.
- **Frame-exact scheduling:** `waveform-navigation` adds scheduled seeks in the worker, which this change builds on.

## Goals / Non-Goals

**Goals:**
- Mixes a DJ would accept: phrase-aligned, on the beat, no bass clash, no audible tempo jumps.
- Zero risk to real-time safety: all mixing happens in the decode worker, and the RT callback is unchanged.
- Graceful fallbacks: never a worse transition than today's gapless cut.

**Non-Goals:**
- Key-lock (time-stretch without pitch change) and harmonic (key-aware) mixing, maybe later.
- Manual DJ controls (jog, pitch fader, cue): this is automatic.
- Choosing track order by key or BPM: the playlist order is respected.

## Decisions

### D1. Mix in the worker with two decks
```
 deck A (outgoing): decoder → resampler(ratio_a) → isolator(low,mid,high) → gain_a ┐
                                                                                   ├─ Σ → ring → callback
 deck B (incoming): decoder → resampler(ratio_b) → isolator(low,mid,high) → gain_b ┘
```
- Both decks are ordinary gapless-capable streams. The worker renders both for each segment and sums them.
- The ring, callback, EQ, tap and volume are unchanged. The tap therefore carries the mixed signal, which is what is audible.
- Alternative considered: mixing in the callback. Rejected, because two decoders' output would have to be buffered separately, which means more RT complexity.

### D2. Tempo matching
- **Ratio:** deck B plays at `r = bpm_A / bpm_B`, allowing half/double time (`r` is taken from {bpm_A/bpm_B, 2·…, ½·…}, whichever is closest to 1). It must be within [0.92, 1.08], otherwise the mix is not beat-matched (D5).
- **How:** a resampler with a variable ratio (rubato's relative ratio adjustment). The pitch shifts with tempo, like vinyl.
- **Phase lock:** deck B's downbeat grid is aligned to deck A's at the mix start. During the mix, the worker corrects drift against both grids with tiny ratio nudges (≤ 0.1%) so the kicks stay together even if a tempo segment drifts.
- **Glide back:** after the handover and once A is silent, B's ratio glides back to 1 over 16 bars, smoothly (a half-cosine).

### D3. Mix plan
The plan is a pure function `plan(score_a, score_b, settings) -> Plan`, unit-testable.
- **Length L:** 8, 16 or 32 bars (a setting).
- **Mix start on A:** the latest phrase boundary (32 beats from `phrase_origin`) such that the mix ends by A's last downbeat. Prefer the start of A's last section if that leaves at least L bars; otherwise use the latest boundary that fits.
- **Entry on B:** the first downbeat after leading silence (RMS above −40 dBFS), rounded to a phrase if B's intro allows.
- **Handover:** the downbeat at the mix midpoint (L/2 bars in).
- **Envelopes:**
  - equal-power crossfade over the whole mix;
  - bass swap: B's low band is cut (−26 dB) until the handover, then swapped with A's over one beat;
  - A's high band fades out slightly earlier to reduce hat clutter.

### D4. Pre-warm timing
With automix on, the next track is pre-warmed at `max(30 s, L bars at A's tempo + 20 s)` before A's end, and the analysis service is asked to prioritize it. The plan is (re)computed when B's first 32 bars and A's final sections are known. If B isn't ready 2 s before the planned start, the plan falls back (D5).

### D5. Fallbacks
Fallbacks are chosen in order, and each is reported as `MixFallback { reason }` and shown briefly in the UI:
1. Tempos are incompatible, or a grid is unreliable (low beat confidence at the points): an equal-power crossfade of L bars on A's phrase boundary without tempo change.
2. There is no analysis in time, a track is shorter than the mix, repeat is One, or it's the last track with repeat Off: the plain gapless cut (today's behaviour).

### D6. Clock during overlap
- The clock reports deck A's track until the handover downbeat is audible, then deck B's track at its exact position (with a discontinuity-free track switch, like a gapless boundary).
- Tempo ratios are applied to the clock's frame→seconds mapping per deck, so `Position.seconds()` stays in each track's own time.
- Consumers (overlay, analysis playhead, director) then follow "the main track".
- Alternative considered: exposing both decks in `Position`. Deferred, because no consumer needs it yet.

### D7. Transport during a mix
- **Pause/resume:** pauses both decks.
- **Seek:** cancels the mix and seeks within the main track.
- **Next:** completes the handover instantly (cut to B at its current position).
- **Stop:** stops both.
- `Shift+B` plans a mix starting at the next phrase boundary of the current track (at least 1 bar ahead), even if automix is off.

## Risks / Trade-offs

- [The worker has twice the decode and resample load during mixes] → Decoding is ~100× realtime. The ring target is unchanged; measure underruns in the engine test harness with two 48 kHz FLAC decks.
- [Beat grids wrong in places] → The confidence check at the mix points, and drift correction capped at 0.1% (a grid that needs more is treated as unreliable → fallback 1).
- [Pitch shift audible on large ratios] → The ±8% cap; most electronica within a genre is within 124–130 BPM.
- [Clock mapping with variable ratios] → The worker attributes output frames to input frames per deck exactly, extending the existing gapless attribution; covered by ManualSink tests.

## Open Questions

- Should the incoming track instead keep its own tempo and the outgoing one adapt ("meet halfway")? Default: incoming adapts.
- Should there be a per-playlist-entry "don't mix into this track" flag?
