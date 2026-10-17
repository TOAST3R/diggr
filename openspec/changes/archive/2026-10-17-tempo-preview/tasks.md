## 1. The resampler swap (D1)

- [x] 1.1 `ChannelResampler::with_speed`: `Async` (sinc) for speeds other than 1, `Fft` kept at speed 1 (and for analysis), with its frame-count contract, delay trim and tail flush; every existing audio test passes (the 90 dB comparison is moot: speed 1 is the old path, see D1)
- [x] 1.2 Measure with `--bench`: start, seek and underruns on the reference Mac; 256 taps kept and recorded in D5 (decode CPU not reported separately)

## 2. Speed in the engine (D2–D3)

- [x] 2.1 `Engine::set_speed` and per-queue-entry speed; the speed change delivered by a soft flush with a 2 ms crossfade (D2; coalesced to one per 30 ms in the UI); verified with `ManualSink` tests: heard within 50 ms, no click, no underrun
- [x] 2.2 Clock stamping source frames and speed per block; `Position::speed`; verified: the stamp is exact 10 minutes in at 1.06 (worker unit test), the engine's clock runs 1.06 track frames per frame, and stays monotonic across a change to 0.96
- [x] 2.3 Gapless at differing speeds (1.03 → 0.97) and the real-time allocation guard (`rt_alloc.rs`) at speed ≠ 1
- [x] 2.4 `--bench --speed S` with a speed-change sweep and the README table

## 3. SYNC in the player (D4, D6)

- [x] 3.1 Settings `sync { on, target, wide }`; the per-track speed with half/double folding and the range rule; the next track's speed set before it starts; verify with unit tests (fitted, half time, out of range, no BPM)
- [x] 3.2 Skin: SYNC button and LCD value via `skin-gen` (generated assets and their test); drag, scroll, double-click and Shift+↑/↓; "+2.4%" display and "(own tempo)" rows; help entries; verify with headless tests
- [x] 3.3 Section jumps, loops, waveform and analysis unaffected at speed 1.04 (they all work in track seconds); verified on the engine with `ManualSink`: a scheduled jump and a loop land in track time at 1.04

## 5. The turntable (D6)

- [x] 5.1 Skin: `deck_bg` (platter, tonearm, LCD, fader groove), pitch thumb, OWN TEMPO lamp, 8%/16% and RESET buttons via `skin-gen`; the old LCD readouts and their hidden gestures removed
- [x] 5.2 The turntable section under the main window while SYNC is on: fader (centre = the record's own tempo, clamped in range), scroll, RESET, range, ×, the turning platter and strobe; layout height; help and README; verified with headless and layout tests

## 4. Docs and checks

- [x] 4.1 README: SYNC, keys, range, "(own tempo)", the bench option, test areas and count
- [x] 4.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check`, the wasm check (`audio` must still build for wasm32), `--startup-time` and `--bench --speed 1.04`; listen to a chart with SYNC on at 126 (all green but the load-sensitive `searching_a_big_crate_is_quick`; `--startup-time` 196–484 ms, noisy on this Mac; listened and confirmed working)
