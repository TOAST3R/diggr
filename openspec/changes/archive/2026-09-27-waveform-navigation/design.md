## Context

The analyzer (`crates/analysis`) decodes each track ahead of the playhead to mono at 22,050 Hz, runs an STFT (1024/256), and produces a `SongScore` with beats, downbeats, sections (labels, energy, provisional or final) and tension. Its frames are dropped after use, and 22 kHz mono is too coarse for a detailed stereo waveform. The engine (`crates/audio`) plays one continuous stream from a decode worker. Seeks happen immediately (generation-tagged flush), with no notion of acting on a particular frame. The classic UI stacks skinned sections (main, EQ, playlist) at a scale factor.

## Goals / Non-Goals

**Goals:**
- A waveform that reads like DJ software: colour shows frequency, and the beat grid and sections are visible.
- Structure jumps and loops that keep the groove: they land on downbeats, sample-accurately, with no gap or click.
- Zero cost to playback start and to the playhead analysis. The waveform fills in as the overview pass progresses.
- It works for 2-hour mixes (bounded memory).

**Non-Goals:**
- Editing, cue-point storage, hot cues (maybe later).
- Key-lock or time-stretching.
- A waveform in fullscreen (the analysis strip already covers it).

## Decisions

### D1. A separate overview pass at native rate
A new pass with its own `TrackDecoder`, not an extension of the analyzer. The analyzer is 22 kHz mono by design (speed), and the waveform needs stereo peaks and a real high band.
- Decoding is ~100× realtime, so a 5-min track takes about 3 s at low priority.
- It runs after the analyzer's first horizon is ready (the playhead analysis keeps priority), then continues independently.
- Alternative considered: widening the analyzer to full rate. Rejected, because it would slow the latency-critical first 32 bars.

### D2. The waveform pyramid
- **Level 0:** a block of 256 frames holds `min_l, max_l, min_r, max_r` (i8, scaled to the peak), `rms` (u8) and `low, mid, high` energy (u8, via a Linkwitz–Riley crossover at 200 Hz / 2.5 kHz). That's 8 bytes per block.
- **Higher levels:** each level is 4× coarser than the previous; min of mins, max of maxes, and mean energies. (Halving would add a full copy of level 0 and push a 2-hour mix past 16 MB; with 4× the total is about 1.33× level 0, and drawing merges up to 4 blocks per pixel.)
- **Band values:** band energies are stored in dB (−60 to 0 dBFS mapped to 0–255), so quiet highs still have resolution for colouring; RMS is linear.
- **Size:** a 5-min track at 44.1 kHz has about 52 k blocks, so around 550 KB for all levels. A 2-hour mix is about 13 MB.
- **Cache:** stored next to scores (`overview-<hash>.bin`, versioned) and written when complete.
- **Drawing:** pick the level whose block ≥ 1 pixel.

### D3. Colour
Each column's RGB comes from its normalized (low, mid, high) energy, with red for bass, green for mids and blue for highs, as rekordbox does. Height is the peak, and a brighter core shows RMS. Colours come from the skin's palette where possible so it sits well with the classic look.

### D4. The waveform section
The section is drawn under the main window, at the main window's width and `58 px × scale` tall. `W` toggles it and `settings.show_waveform` persists the state.
- **Top row (14 px):** the whole track, with section bands (the analysis strip's colours by kind, faint when provisional), drop markers and the playhead. Click or drag seeks.
- **Bottom row (44 px):** a zoomed view centered on the playhead, with 8 bars visible by default. The scroll wheel zooms from 1 bar to 64 bars. Beat ticks are shown and downbeats are taller. The playhead is fixed at the center and the waveform scrolls.
- It is rendered as an egui mesh: at most one quad per pixel column, about 1,000 quads at 2× scale, which is trivial. When playback is paused it doesn't repaint (the idle-cost requirement).

### D5. Quantized, sample-accurate jumps
The engine gains `seek_at(track, at_secs, target_secs)`:
- The player sends the target and the next 4 downbeats of the *audible* position (from `SongScore.downbeats`).
- The decode worker runs about half a second ahead of what is audible (the ring), and it can only splice audio it hasn't sent yet. So it picks the first candidate at or beyond what it has sent. At 128 BPM that's the next bar, or the one after when the next is less than ~0.5 s away, which is how DJ software quantizes too.
- The player shows the pending jump on the waveform. If every candidate is already sent, the engine reports `JumpMissed` and the player jumps immediately.

The splice itself:
- The worker keeps decoding until it has 2 ms of audio past the splice frame.
- It truncates the unsent output there, seeks the decoder to the target, and restarts the stream (new resampler) at that output frame.
- It crossfades 2 ms (equal power) from the replaced audio into the new audio.

**Clock:** ring segments carry a *splice number*, and the renderer folds it into the clock epoch. The clock therefore flags a discontinuity at the splice with the same track id, and nothing queued is flushed.
- When the splice starts inside a callback buffer, the snapshot carries a separate epoch for positions past the in-buffer boundary. That way a jump *backwards* (loop wrap, previous section) isn't clamped by the clock's monotonic guard.
- A new track id was considered instead, but the visuals would take it for a track change and crossfade the show.

### D6. Loops
`set_loop(track, Some((a_secs, b_secs)))` makes the worker wrap from B to A with the same splice as D5, re-armed after every wrap. A loop whose end has already been sent is rejected (`LoopRejected`). `L` toggles a loop over the current section, clamped to 32 bars. `Shift+L` cycles 4, 8 and 16 bars starting from the current downbeat. The loop region is drawn on the waveform. Seeking outside the loop or changing tracks clears it.

### D7. Section and drop targets
- `[` / `]`: the start of the previous or next section. Within the first bar of a section, `[` goes to the previous one.
- `Shift+]`: the next boundary whose energy rises by at least 4 dB (the director's rise rule), falling back to the next section. Section kinds are unreliable on real music, so "drop" means "energy rise".
- Provisional boundaries are allowed but drawn faint, since the user can see what they are jumping to.
- **Keys:** `[` and `]` are matched by physical key position (right of P) as well as by character, since on many non-US layouts the characters need Alt combinations.

### D8. Overview requests and repaint
- **When the overview is requested:** the player asks for the audible track's overview once its first analysis results exist, and asks for the next track's on its pre-warm event. It is built at low priority and published every 250 ms. Six tracks are kept in memory, and the rest come from the disk cache.
- **Repaint:** while playing with the waveform visible, the window repaints at the display rate (16 ms), so the zoomed row scrolls smoothly. When paused it doesn't repaint.
- **Default:** the section is shown by default and remembered.

## Risks / Trade-offs

- [The overview pass competes for CPU with the analyzer on long mixes] → Lowest priority, it yields between chunks, and it starts only after the analyzer's first horizon.
- [A scheduled seek near a track boundary or during pre-warm] → The worker serializes events. A scheduled seek beyond the current track's end is rejected and the UI jumps immediately instead.
- [Beat grids can be wrong on beatless intros] → Quantize only when `beat_confidence` is high at the target, otherwise jump immediately.
- [The skin has no waveform art] → The section is drawn procedurally, framed with the skin's existing border sprites.

## Open Questions

- Should the zoomed row show both channels (mirrored L up / R down) or mid only? Default: mirrored.
- Should hot cues follow later as their own change?
