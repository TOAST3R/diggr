## Why

For electronica, the best transition between tracks is a DJ mix: the next track comes in on the beat, at the same tempo, over a phrase, and the bass lines don't clash. The player already knows every track's beat grid, phrases and sections ahead of time, which is exactly what a beat-matched automatic mix needs. Few players can do this, and it makes a playlist play like a set.

## What Changes

- **Automix mode** (off by default, persisted): consecutive tracks overlap in a beat-matched, phrase-aligned mix instead of a gapless cut.
  - The incoming track starts on a downbeat, and its tempo is matched to the outgoing track (vinyl-style: pitch follows tempo, within ±8%, with half/double time accepted).
  - It is crossfaded over 8, 16 or 32 bars (default 16), with a bass swap so two kick drums and bass lines never play at full level together.
  - After the outgoing track has faded, the incoming one glides back to its own tempo over 16 bars.
- **Mix planning:** the mix start is chosen on a phrase boundary of the outgoing track, preferring its last section (the outro), and the incoming track enters at its first downbeat after any leading silence.
  - When tempos are incompatible or a beat grid is unreliable, it falls back to an equal-power crossfade on a phrase boundary without tempo change.
  - When analysis isn't ready in time, it falls back to a gapless cut.
- **Mix now:** `Shift+B` mixes into the next track starting at the next phrase boundary, whether or not automix is on.
- **Earlier pre-warm:** in automix, the next track is pre-warmed (opened, decoded and analyzed) early enough for its beat grid to be ready before the mix begins.
- **Clock and visuals:** during a mix, the clock reports the outgoing track until the handover downbeat at the mix midpoint, then the incoming track. So analysis, overlay and director follow the music, and the visuals crossfade on the handover.

## Capabilities

### New Capabilities
- `automix`: mix planning (points, compatibility, fallbacks), deck mixing (tempo ratio, crossfade, bass swap, tempo glide), settings and the mix-now key.

### Modified Capabilities
- `audio-playback`: gapless transition and pre-warm timing now depend on automix; with automix on, tracks overlap and pre-warm starts earlier.
- `playback-clock`: during an overlap, the clock reports one track at a time and switches at the handover downbeat.

## Impact

- `crates/audio`: the decode worker gains a second "deck" (decoder + variable-ratio resampler + 3-band isolator + gain envelope) and mixes decks into the ring. The RT callback is unchanged. New engine API: `set_automix(settings)`, `mix_next_now()`, and events `MixPlanned`, `MixStarted`, `MixHandover`, `MixFinished`, `MixFallback`.
- `crates/analysis`: a mix planner (pure function of two `SongScore`s) and a prewarm-priority hint.
- `crates/ui`: automix toggle and mix length in the options, `Shift+B`, and a "MIX" indicator with a countdown in the main window.
- `crates/visuals`: the director's track-change rule fires at the handover (no code change needed if the clock switches there).
- Depends on the scheduled-seek work from `waveform-navigation` (frame-exact scheduling in the worker).
