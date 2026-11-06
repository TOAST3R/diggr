# Design

## Context

The audio path is `decode → varispeed resampler → PCM ring → renderer → device`
(`crates/audio`). Varispeed lives in the decode worker and is limited to 0.84–1.16; a speed
change is a soft flush (a new generation crossfaded in 2 ms). The renderer
(`renderer.rs::render`) reads **runs** from the ring into `scratch` per 1024-frame block, each run
stamped with its track, start frame and speed, then applies EQ, low-pass, writes the tap per
run, and the device. The clock snapshot carries the buffer's starting `(track, frame)` and a
`speed` (track frames per buffer frame); readers extrapolate `frame + offset × speed`, so a
speed of 0 freezes the position. State crosses threads only through atomics (`Control`), rings
and the seqlock clock, and `tests/rt_alloc.rs` counts allocations in the callback.

## Goals / Non-Goals

**Goals:**
- A momentary vinyl stop and spin-up, heard within 20 ms of the press, without clicks.
- Playback without a hold exactly as today (bit for bit), and every latency target kept.
- A stage that scratching can extend later (a history buffer and negative rates).

**Non-Goals:**
- Scratching, reverse playback, latching, keyboard holds, user-set brake times.
- Holding during a gapless boundary in a smart way: a boundary during a hold just plays on at
  the current rate.

## Decisions

**A rate stage inside the renderer, after the ring read.** The renderer gains a `Platter`:
`rate: f32`, `phase: f32` (the interpolation point between two input frames), the two frames it
interpolates, and a fixed input buffer (`BLOCK_FRAMES + 2` frames) allocated in `Renderer::new`.
When it is **at rest** (no hold, rate exactly 1, phase 0) the existing run loop runs untouched:
that is the bit-exact bypass. Otherwise the block is produced by the stage: for each output frame
the rate slews toward its target (0 while held and confirmed, else 1) by
`1 / (BRAKE_SECS · sample_rate)` or `1 / (SPINUP_SECS · sample_rate)` per frame; the phase
advances by the rate; each whole step consumes one input frame (read from the ring through the
same run reader, with its stamps); the output is the linear interpolation of the two frames at the
phase. *Alternative:* using the decode worker's varispeed: it can't reach 0, and every change is a
soft flush through the ring, far too slow for a hand.

**Returning to rest.** When a release has brought the rate to 1, the phase is generally not 0, and
staying interpolated at rate 1 would low-pass the audio slightly. The stage then runs at
`1 + (1 − phase) / 64` until the phase wraps (a 0.1% pitch blip for under 2 ms), sets the phase to 0
and hands back to the bypass. *Alternative:* staying in the stage forever after a hold: it would
break "bit-exact at rest" for the rest of the track.

**Control.** `Control` gains `hold: AtomicBool` and `hold_seen_ns: AtomicU64`. `Engine::hold(true)`
sets both (with the clock's time base, `now_ns`); `Engine::hold_keepalive()` refreshes the time;
`Engine::hold(false)` clears the flag. The callback treats the hold as off when
`info.host_ns − hold_seen_ns > 500 ms` (the watchdog). `Engine::seek`, `stop`, `pause`,
`play_index`, `next`, `previous` and `set_speed` clear it before bumping the generation; and the
renderer resets the stage to rest whenever it starts a new generation, so audio after a flush
always begins at its own speed.

**The clock.** The stage keeps the track frame of its interpolation point. The snapshot's `frame`
is that point at the buffer's start and its `speed` the run's speed × the block's mean rate (0 when
held). Within a 5–10 ms buffer the mean rate is close enough to the 1 ms bound; the next buffer's
snapshot starts from the true point, so there is no drift. `starved` stays false while held (the
output is meant to be silent), and holding doesn't count as an underrun.

**The tap.** While the rate is 0 nothing is written to the tap, so the spectrogram and analysis
freeze rather than see silence stamped at a speed of 0; otherwise each block's audio is written
with the run's speed × the block's mean rate, never 0.

**The UI.** In `deck_section`, a press on the platter's circle (its rect, `deck_platter`, with a
radius test) while a track plays calls `engine.hold(true)`; every frame while the button is
down it calls `hold_keepalive()` and asks for a repaint within 100 ms; release, the pointer leaving
the window, or the window losing focus call `hold(false)`. With SYNC off, the deck's dimming skips
the platter's rect. The platter's mark already turns with `position.speed`, so it slows and stops
with the record for free.

**Times.** `BRAKE_SECS` = 0.4 and `SPINUP_SECS` = 0.3, constants in `audio`.

## Risks / Trade-offs

- [The renderer is the most delicate code] → The stage is a separate path entered only when not at
  rest; the bypass is the old code. Engine tests use `ManualSink` and fake time; `rt_alloc` covers a
  full hold cycle.
- [Linear interpolation at low rates] → At rates ≤ 1 it only upsamples, so images stay low; it is the
  sound of a slowing record anyway.
- [A soft flush (SYNC changing the speed) during a hold ends it] → Documented and rare; the record
  plays on at the new speed.
- [Two decks later (`beatmatch-automix`)] → The stage is per stream, so it moves with the deck.

## Migration Plan

No data changes. The engine API only adds methods.

## Open Questions

- (Settled) The brake time stays at 0.4 s and the spin-up at 0.3 s.
