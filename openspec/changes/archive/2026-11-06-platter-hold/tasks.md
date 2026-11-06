# Tasks

## 1. Engine

- [x] 1.1 Add `hold` and `hold_seen_ns` to `Control`, and `Engine::hold(bool)` and `Engine::hold_keepalive()`; clear the hold in `seek`, `stop`, `pause`, `play_index`, `next`, `previous` and `set_speed`. Verify: engine unit tests that each of those clears it.
- [x] 1.2 Add the `Platter` stage to the renderer: rate slew to 0 and back (`BRAKE_SECS` 0.4, `SPINUP_SECS` 0.3), interpolation, consuming the ring at the rate, the return to rest, the watchdog, the reset on a new generation. Verify: `ManualSink` tests for the brake time (±10 ms), silence and no ring consumption while held, spin-up back to the speed (1 and 1.03), release before the stop, the watchdog at 0.5 s, and the output without a hold bit-identical to the old renderer.
- [x] 1.3 Publish the clock and the tap under a hold (mean rate, held position, no tap writes at 0, no underrun counted). Verify: clock tests for each scenario of "Clock under a hold"; `tests/rt_alloc.rs` with a hold and release; `diggr --bench` reports the hold's reaction and the latency targets still pass.

## 2. The turntable

- [x] 2.1 In `deck_section`, hold on press of the platter while a track plays, keep it alive while held, release on button up, pointer leaving or focus lost; leave the platter undimmed with SYNC off. Verify: headless tests for each scenario of "Holding the platter" (the engine reports the hold, the time stops, release resumes).

## 3. Docs and checks

- [x] 3.1 Update README (the Turntable section, the Tests count), the help panel and CHANGELOG. Verify: they describe holding the platter.
- [x] 3.2 Run the AGENTS.md checks (including the wasm check of `audio`) and `openspec validate platter-hold --strict`. Verify: all clean.
