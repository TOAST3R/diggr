## Why

The player already knows every track's beat grid, phrases, sections and energy ahead of time: exactly what a DJ uses to mix. A SYNC mode can turn a playlist into a continuous set, played the way a DJ would:
- the next track comes in on the beat, at the same tempo, under an EQ'd crossfade;
- long intros and outros are skipped;
- each transition happens at the best moment rather than when a track runs out.

## What Changes

- **SYNC button:** a new SYNC button in the main window's button row, right of shuffle and repeat, persisted and off by default, switches SYNC mode on and off. SYNC has one mode, vinyl: pitch follows tempo.
- **Icon shuffle and repeat:** to make room, the SHUFFLE and REP text buttons become icon buttons (crossed arrows; a loop, with a "1" for repeat one), lit green when on.
- **Continuous set:** in SYNC mode, consecutive tracks are mixed, not joined by gapless cuts. `Shift+B` mixes into the next track at the next phrase whether or not SYNC is on. `B` still cuts straight to the next track.
- **Held set tempo, vinyl-style:** the set keeps one tempo. The incoming track is sped up or slowed down to the set tempo, and its pitch follows, like vinyl. It stays at that tempo for the whole track. Tempos within ±8% are matched, with half/double time accepted (e.g. 87↔174). In SYNC mode, a next track whose tempo can't be matched is skipped, and the set continues with the next one that can. If none of the rest can be matched, the next track is crossfaded at its own tempo, and the set continues at that tempo.
- **Mixes of about 30 s:** the length is rounded to whole 8-bar phrases at the set tempo.
- **The best moment, not the end:**
  - The mix starts at the outgoing track's outro, after its last big section.
  - The incoming track enters so that its intro plays under that outro and its first big section (the "drop") lands as the mix ends.
  - Intros and outros are not played on their own.
- **A proper EQ mix:**
  - The incoming track's bass and kick are cut (low band below 200 Hz) at first.
  - At the handover downbeat, halfway through the mix, the lows swap within one beat: the incoming track's kick and bass come in, and the outgoing track's are muted.
  - `E` swaps the lows by hand on the next beat, earlier or back again. Once the user has swapped, the automatic swap doesn't happen.
- **Volume:**
  - The outgoing track starts louder, and the balance moves little by little, an equal-power crossfade, until the incoming track is at full volume and the outgoing one is silent.
  - The two tracks' levels are matched by their analyzed loudness.
- **Two waveforms while mixing:** during a mix, the waveform's zoomed row shows both tracks, the outgoing one over the incoming one. Both are drawn at the set tempo, so their beat grids line up, with the handover and the EQ state visible.
- **Behind the scenes:** the player plans each mix from both tracks' analysis and hands the plan to the engine, which mixes two decks in the decode thread. The clock reports the main track at the right speed, so visuals, overlay and analysis stay in sync.

## Capabilities

### New Capabilities
- `automix`: SYNC mode covering tempo, mix length, mix points, EQ, volume, fallbacks, mix-now and transport during a mix.

### Modified Capabilities
- `audio-playback`: in SYNC mode, tracks overlap in planned mixes and the next track is pre-warmed earlier. The engine executes mix plans (two decks, a tempo ratio, band gains).
- `playback-clock`: during a mix, the clock reports one track at a time, switches at the handover, and follows the incoming track's speed.
- `player-window`: icon shuffle and repeat buttons, a SYNC button to their right, `Shift+B` (mix now) and `E` (swap the kick and bass during a mix).
- `waveform-view`: two stacked waveforms during a mix.

## Impact

- **`crates/analysis`:** a mix planner, a pure function of two `SongScore`s and the set tempo returning an `audio::MixPlan`. The engine cannot see scores, since `analysis` depends on `audio`.
- **`crates/audio`:**
  - `MixPlan` data, `Engine::schedule_mix` / `cancel_mix`, and mix events;
  - a second deck in the worker, with a variable-ratio resampler (rubato's asynchronous resampler) and a 3-band isolator with gains;
  - a speed factor per ring segment for the clock (one float, safe in the audio callback).
- **`crates/ui`:**
  - orchestration: early pre-warm, planning, rescheduling and fallbacks;
  - icon sprites for shuffle and repeat, the SYNC button (skin generator plus layout), `Shift+B`, `E`, a MIX indicator;
  - the two-deck waveform.
- **`crates/visuals`:** nothing new. The director's track-change rule fires at the handover because the clock switches there.
- **Builds on** frame-exact splicing (`waveform-navigation`) and the track overview (for the incoming track's waveform).
