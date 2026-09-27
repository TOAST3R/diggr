## 1. Mix planning

- [ ] 1.1 `plan(score_a, score_b, settings)`: tempo compatibility with half/double time, mix start on a phrase boundary preferring the last section, entry after leading silence, handover at midpoint, envelopes
- [ ] 1.2 Fallback selection with reasons (tempo, grid confidence, not ready, too short, repeat One)
- [ ] 1.3 Unit tests on synthetic scores: 124→126 plan, 128→174 fallback, short track, low-confidence grid, outro preference

## 2. Engine: decks and mixing

- [ ] 2.1 Second deck in the worker: decoder, variable-ratio resampler, 3-band isolator, gain envelope; summing into the ring
- [ ] 2.2 Phase-locked start via scheduled frames; drift correction ≤ 0.1%; glide back to ratio 1 over 16 bars after A is silent
- [ ] 2.3 Early pre-warm and analysis priority hint; plan (re)computation; deadline fallback
- [ ] 2.4 Clock: per-deck frame attribution with ratios; handover switch at the audible downbeat
- [ ] 2.5 Transport during a mix (pause, seek cancels, next completes handover, stop); `mix_next_now`
- [ ] 2.6 Events MixPlanned/Started/Handover/Finished/Fallback
- [ ] 2.7 ManualSink tests: downbeat alignment ≤ 5 ms, bass-swap energy, handover clock, next during mix, no underruns with two 48 kHz FLAC decks, RT-alloc test still clean

## 3. UI

- [ ] 3.1 Automix toggle and mix length in the options; persisted
- [ ] 3.2 `Shift+B` mix now; MIX indicator with countdown to handover; brief fallback message
- [ ] 3.3 Director: verify the visuals' track-change crossfade lands on the handover

## 4. Docs and verification

- [ ] 4.1 README: automix, settings, fallbacks
- [ ] 4.2 Listening test on a real electronica playlist: mixes on the beat, no bass clash, sensible mix points
