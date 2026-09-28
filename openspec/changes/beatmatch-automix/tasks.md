## 1. Mix planning (analysis)

- [ ] 1.1 `audio::MixPlan` data type (frames, ratio, entry, handover, gains, downbeat lists for drift)
- [ ] 1.2 Set tempo and ratio selection: half/double time, ±8% cap; out of range → skip to the next matchable track in play order (never a user-started one), or crossfade and a new set tempo when none is left
- [ ] 1.3 Mix length: 8-bar multiple closest to the setting at the set tempo (8–32 bars)
- [ ] 1.4 Mix points: A's outro (first fall after the last rise, ≥ 16 bars after it, phrase-aligned) with the latest-boundary fallback; B's main start (first rise) and entry `M_B − L`; loudness match (±6 dB)
- [ ] 1.5 Fallback selection with reasons (tempo, grid, not ready, too short, repeat One, last track)
- [ ] 1.6 Unit tests on synthetic scores: 124→126, 172→87 half time, 128→150 skipped for a 126, nothing matchable → crossfade, outro/intro skipping (64-bar outro, drop at bar 64), 30 s at 128 and 174 BPM, fallbacks

## 2. Engine: decks, EQ and clock (audio)

- [ ] 2.1 Per-segment `speed` in the ring, renderer and clock snapshot; readers interpolate with it (RT-safe)
- [ ] 2.2 Deck B: pre-warmed decoder seeked to the entry, variable-ratio (async sinc) resampler, held ratio after the mix
- [ ] 2.3 Isolator (200 Hz / 2.5 kHz) with per-band gains on both decks; volume envelopes (equal power), loudness gain with its 4-bar return, low swap at the handover, A's highs one bar ahead
- [ ] 2.4 `schedule_mix` / `cancel_mix` / `prewarm_next`; frame-exact start; handover attribution; drift correction ≤ 0.1%; events MixScheduled/Started/Handover/Finished/Missed
- [ ] 2.5 Transport during a mix: next cuts to B, seek cancels, pause/stop both; `swap_lows()` on the next unsent beat, toggling, cancelling the automatic swap
- [ ] 2.6 ManualSink tests with click tracks: downbeats ≤ 5 ms apart, low-band energy ≤ 3 dB over A alone before the handover, swap within one beat (automatic and manual, and swap back), volume at 25/50/100%, clock switch and speed after the handover, missed plan, no underruns with two 48 kHz FLAC decks; RT-alloc test still clean

## 3. Player (ui)

- [ ] 3.1 Icon shuffle and repeat buttons (23×15 at x 164 and 188; crossed arrows, loop, loop with "1"; green when on), SYNC setting and button (47×15 at x 217, text sprites lit when on), layout in the skin generator and regenerated skin; MIX countdown indicator
- [ ] 3.2 Orchestration: provisional mix point, early pre-warm (engine and analysis), planning when both scores cover the points, rescheduling, 2 s deadline, fallbacks with a 3 s message; SYNC off cancels a pending mix
- [ ] 3.3 `Shift+B` mix now; `B` cuts straight during a mix; `E` swaps the lows; skipped-track message; help panel and README keys
- [ ] 3.4 Two-lane zoomed waveform during a mix: aligned grids at the set tempo, mix region, handover line, dimmed bass while cut, gain-following brightness; the incoming track's overview requested on pre-warm
- [ ] 3.5 Headless tests: SYNC toggle persists, the main button row's rects don't overlap and stay inside the window, countdown text, other-deck position from the plan, lane grid alignment

## 4. Docs and verification

- [ ] 4.1 README: SYNC mode, what it skips, EQ and volume behaviour, fallbacks, Shift+B
- [ ] 4.2 Listening test on a real electronica playlist: kicks together, no bass clash, sensible mix points, no loudness jumps, visuals switch at the handover
