## Context

- **The decode path:** decoder → `ChannelResampler` (rubato `Fft`, `FixedSync::Input`, a fixed ratio of device rate over file rate) → a lock-free ring of about 0.5 s → the audio callback (EQ, tap, volume). A ratio change at the resampler would be heard only after the ring drains (about 0.5 s).
- **Seeks and loops:** they already use a frame-scheduled splice: the worker re-decodes from a source position and the engine switches at an exact output frame, without a flush click (`audio-playback`, Frame-scheduled seek).
- **The clock:** it publishes which source frame is at the speaker. Analysis, the waveform and structure navigation all work in source seconds.
- **rubato 5:** it provides `Async` (sinc interpolation, `Adjustable`, `set_resample_ratio_relative`) for ratios that change while running.
- **Entries:** each has a BPM once analysed, or from its store (`beatport-traxsource`).

## Goals / Non-Goals

**Goals:**
- Hear any track at a chosen BPM, switching instantly and cleanly, with every existing feature unaware of the speed.
- One engine mechanism that `beatmatch-automix` reuses unchanged.

**Non-Goals:**
- Key lock (time-stretching without pitch change): a separate, much heavier DSP step.
- Pitch bend or nudging, and syncing two decks: that's automix.
- Speeds beyond ±16%.

## Decisions

### D1. A sinc resampler for speeds other than 1
- **The change:** `ChannelResampler` keeps its contract ("`n` input frames become exactly `round(n · ratio)` output frames", startup delay trimmed, tail flushed) over either rubato `Fft` or `Async` with sinc interpolation (256 taps, Blackman-Harris², 128× oversampling). `with_speed` uses `Fft` at speed 1 and sinc otherwise, with the ratio `device / file / speed`.
- **Why not sinc for every track (as first planned):** analysis also resamples through `ChannelResampler` (mono, to its own rate), so swapping it would move every analysis result and its eval goldens; and at speed 1 the FFT path stays bit-identical to before, with no CPU cost added to the common case. The 90 dB comparison is moot: speed 1 is the old path.
- **The edge cases of two paths:** a track at a different speed starts a new stream, as a new source rate already did. To keep that join from ringing (each filter would otherwise see silence past its edge), the old resampler's tail is filled with the next track's first 4096 frames (`finish_with`) and the new one is primed with the old track's last 4096 (`prime`). This also smooths the existing join between source rates.

### D2. Speed changes flush the ring softly
- **Why not the frame-scheduled splice:** it splices audio not yet sent to the ring, about 0.5 s ahead of the speaker, so a change would be heard only after that.
- **How:** `Engine::set_speed(index, s)` restarts the audible track (`Command::Respeed`, which names it) at the source position that will be audible when the new audio takes over (now + a buffer + the output latency + 5 ms, at the old speed; a little early is inaudible), in a new generation marked soft (`Control::mark_soft`). The callback keeps playing the old generation until audio of the new one is queued, then takes up to 2 ms of the old (96 frames) and crossfades it into the new. Heard within a buffer or two, with no gap and no click.
- **Edge cases:** an active loop stays (a position past its end wraps into it). Past the track's end, nothing restarts: it finishes at its old speed and the next start has the new one. If the decoder is already in the next track (the last 0.5 s), the audible track is restarted by its queue index, as a device reload does.
- **Rapid changes:** while dragging the target, the UI applies a change to the playing track at most every 30 ms. Other entries' speeds are just stored, and used at their next start.

### D3. The clock stays in source time
- **What the clock publishes:** each output block carries the source frame it starts at and the speed it plays at. Readers interpolate source position as `start + elapsed_output · speed · file/device`.
- **What doesn't change:** `Position::seconds()` is still source seconds, so the waveform, `[`/`]` (on source downbeats), loops, the analysis horizon and the spectrogram need no change.
- **What's new:** `Position::speed` lets the UI show "+2.4%".
- **A boundary within one block:** when a speed change or track change falls inside a block, the block is split at the boundary. This already happens for track changes.

### D4. Per-track speed from the target
- **The rule:** with SYNC on, a track with BPM `b` plays at `s = target / b`.
- **Folding:** the ratios `target / (b · 2)` and `target / (b / 2)` are also considered (half and double time), and the one closest to 1 wins.
- **Range:** if `|s − 1|` exceeds the range (8% or 16%), the track plays at 1.0, marked "(own tempo)". A track without a BPM also plays at 1.0.
- **Pre-warm:** every UI frame sets the speeds of the playing track's neighbours in the queue (next, previous, first), and an entry started by hand gets its speed just before `play_index`, so the gapless boundary carries the next track's ratio from its first frame.
- **Changing the target:** this re-applies to the playing track at once (D2), and to the queued one at its start.

### D5. Performance guard
- **Benchmark:** `--bench` gains `--speed 1.04`, running start, seek and a speed-change sweep. Targets: start < 30 ms, seek < 50 ms, speed change heard < 50 ms, 0 underruns, and the CPU in the decode worker reported.
- **Fallback:** if the 256-tap filter misses on the reference Mac, use 128 taps.
- **Measured (M-series Mac, CoreAudio 48 kHz, 512 frames, the fixtures):** start 6.9–19.4 ms, seek 4.2–20.8 ms, speed change 7.3–21.2 ms. 0–1 underruns per run in the gapless pass over the 1–2 s fixtures, the same as the build before this change. **256 taps stay.** Decode CPU is not reported separately.

### D6. Settings and UI
- **Persistence:** `settings.sync { on, target, wide }`, remembered across launches.
- **The control:** a SYNC button right of REP opens a turntable section under the main window (`deck_bg`, 104 px, like the EQ and waveform sections). A first version put the target on the main LCD with hidden drag, scroll, double-click and right-click; users wouldn't find where to change the tempo, so everything is now visible on the turntable:
  - **Pitch fader:** a Technics-style long fader, up faster (the 1200 has + at the bottom; here the direction reads at a glance). Its centre is the playing record's own tempo, its ends ± the range, so pulling it is "push this record by x%", and the target it sets (whole BPM, clamped inside the range so the record stays locked) carries to every following record, where the thumb then shows that record's push. Scroll over it or Shift+↑/↓ for 1 BPM steps.
  - **LCD:** the target in 7-segment digits, the push ("+3.3%") and "REC 122"; an OWN TEMPO lamp; RESET (the record's own tempo) and 8%/16% buttons.
  - **Platter:** turns at 33⅓ rpm × the speed heard (a mark on the label). Its 24 strobe dots stand still while the record is fitted and drift by `own BPM / target − 1` revolutions a second otherwise, as on a 1200 whose pitch is off.
  - The sprites and layout entries are optional, so older skins still load (without the turntable). The maximized playlist's mini player has no turntable.
- **Visualizers:** tap chunks carry their speed, so the spectrum bars and the live spectrogram match heard samples to track frames under varispeed.
- **Keys:** Shift+↑ / Shift+↓ change the target by 1 BPM.
- **Display:** while a track plays sped up, the main window shows "+2.4%". The entry still shows its own BPM.

## Risks / Trade-offs

- **[Sinc resampling changes the sound at speed 1]** → A 256-tap filter is transparent in the audible band. A test compares the 1.0 path with the previous FFT output (SNR ≥ 90 dB on the fixtures).
- **[CPU on old Intel Macs]** → Measured (D5). The filter length is the knob.
- **[Rounding drift between source and output frames]** → The clock derives source position from stamped block starts, not by integrating, so it can't drift. A test plays 10 minutes at 1.06 and checks the clock against the source position within 1 ms.
- **[Gapless at differing speeds]** → The next track's resampler starts at its own ratio. The boundary test plays two tracks at 1.03 and 0.97 and checks for no gap and no click.

## Migration Plan

- **The resampler swap (D1)** lands first, with speed fixed at 1 and every existing test green.
- **Then** the speed path and the UI.
