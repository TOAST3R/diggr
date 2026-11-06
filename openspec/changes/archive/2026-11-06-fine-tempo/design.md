# Design

## Context

`SyncSettings` (`crates/ui/src/sync.rs`) holds `target: u16` (whole BPM, 40–250) and `wide`;
`speed(bpm)` is `target / bpm` (or its double or half). The deck (`deck_section`) maps the
pitch fader's pointer to a whole target (`Action::SyncTarget(u16)`), scrolls by
`Action::SyncNudge(i32)` (1 BPM per 4 px), and Shift+↑ / Shift+↓ call `nudge_sync(±1)` with the
player focused (`app.rs` key handling, after the `egui_wants_keyboard_input` guard). The big
digits are `digit_*` sprites at `deck_target` (3 × 10 px); "BPM" is painted in `deck_bg` at
x 120. Records' BPM is `Entry::bpm: Option<u16>`, rounded by `format::dj_bpm` in
`set_track_bpm` from the analysis's `f64`.

## Goals / Non-Goals

**Goals:**
- A target with 0.01 BPM resolution, reachable by dragging (Shift), scrolling, keys and typing.
- Old settings load unchanged; new settings stay readable by an older build.
- Nothing on the playback path changes: the engine already takes any `f64` speed.

**Non-Goals:**
- Decimals for records' own BPM (the spike decides; a later change would add a precise `tempo`
  beside `Entry::bpm`).
- Changing the coarse controls.

## Decisions

**Hundredths beside the legacy field.** `SyncSettings` gains `target_cents: Option<u32>`
(BPM × 100, `#[serde(default)]`). The effective target is
`target_cents.unwrap_or(target as u32 * 100)`; every write sets both (`target` to the rounded
whole BPM), so an older build reading the file keeps working. Accessors `target_bpm() -> f64`
and `set_target_bpm(f64)` (clamped to 40–250, rounded to 0.01) replace direct use of `target`;
`nudge(f64)` steps in BPM. `speed` and `fit` use `target_bpm()`. *Alternative:* turning `target`
into an `f32`: RON doesn't read `124` as a float everywhere, and older builds would fail to load.

**Actions.** `SyncTarget(u32)` carries hundredths; `SyncNudge(i32)` carries hundredths too, so
1 BPM is `100`, the fine step `1`. The range clamp for a record of `b` BPM becomes
`[ceil(b·(1−r)·100), floor(b·(1+r)·100)]`.

**Fader modes.** A plain drag is absolute as today, rounded to 0.01. Shift+drag is relative: on
press (or when Shift goes down mid-drag) the app stores `fader_anchor = (pointer_y, target)`, and
the target becomes `anchor_target + (anchor_y − y) × 0.01` per skin pixel. Scrolling keeps its
4 px accumulator; with Shift held, each step is 1 hundredth instead of 100.

**Keys.** Alt+↑ / Alt+↓ (player focused) call `nudge_sync(±1)` in hundredths; Shift+↑ / ↓ stay
±100. While SYNC is on, plain ↑ / ↓ call `nudge_sync(±10)` instead of changing the volume: three
steps (0.01, 0.1, 1 BPM) for the deck, and the volume keeps its slider. With SYNC off the plain
arrows are the volume as before. Every keyboard and scroll step is kept within the playing
record's range (a tempo already outside it, set before the record, may only move back toward
it). *Alternative:* plain arrows at 0.01 with Alt for the volume: rejected as too slow a default
step.

**Typing.** A click on `deck_target` with SYNC on and a known BPM sets
`deck_typing = Some(String)`. While it is `Some`, the keyboard handler routes `Event::Text`
(digits, `.` and `,`), Backspace, Enter and Esc to the editor and returns before any shortcut
(the same place `egui_wants_keyboard_input` returns), so Space and the letters do nothing. The
editor keeps at most three digits before the point and two after. Enter parses, clamps to the
record's range, sends `SyncTarget`, and posts a message when it clamped; Esc, a click outside
the digits, or SYNC turning off clears it. The digits draw the typed text with the skin font in
the LCD colour and a caret blinking at 2 Hz (a repaint request only while typing). The digits
are bitmap sprites, not an egui `TextEdit`, so this small editor is cheaper than overlaying one.

**Display.** The big digits stay `{whole:>3}`; the hundredths are drawn with the skin font at a
new layout entry `deck_fine` (x 115, y 11, 3 characters, baseline-aligned with the digits),
for example `.35`. The "BPM" label in `deck_bg` moves from x 120 to x 136. The counting digits
interpolate in hundredths. Messages format the target with `format::bpm_2` (up to two
decimals, trailing zeros and point dropped).

**The spike.** `analysis::synth` renders tracks at known tempos. A test-only harness (not
shipped) runs the analyzer on 8 of them (for example 98.5, 122.37, 124, 128.25, 140, 150.8, 172.5,
174) and records the error of the score's tempo, in `design.md` of this change. The decision on
decimals for records is left to a later change.

## Risks / Trade-offs

- [Older builds round the target on their next save] → They write `target` only, and this build
  then reads `target_cents` as absent and uses the whole BPM: the decimals are lost, nothing
  breaks.
- [The typing editor is custom] → It is small and tested; it only accepts the characters a tempo
  needs.
- [Shift is also the playlist's selection modifier] → The fader and its scroll are on the
  turntable, so Shift there never reaches the playlist.

## Migration Plan

No migration step: `target_cents` is optional and written alongside `target`.

## Spike results

`rhythm::tests::tempo_accuracy_on_fractional_tempos` (ignored; run with
`cargo test -p analysis tempo_accuracy -- --ignored --nocapture`) analyses the standard
synthesized track at eight tempos:

| True BPM | Segment BPM | Beat-line BPM |
|---|---|---|
| 98.50 | 98.500 | 98.500 |
| 122.37 | 122.370 | 122.370 |
| 124.00 | 124.000 | 124.000 |
| 128.25 | 128.250 | 128.250 |
| 140.00 | 140.000 | 140.000 |
| 150.80 | 150.800 | 150.800 |
| 172.50 | 172.500 | 172.500 |
| 174.00 | 174.000 | 174.000 |

Both the tempo segment and a straight line through the detected beats hit every tempo to three
decimals, so the estimator has no rounding bias: today's whole-BPM records throw away precision
the analysis has. These tracks have a perfectly steady grid, though, so this says nothing yet about
real records (live drummers, drifting edits, swing).

**Recommendation:** a later change can store the analysed tempo with two decimals beside
`Entry::bpm` and fit SYNC against it, but should first check it on the `eval` tools' real files,
and show decimals for a record only when its tempo segment is a single steady one.

## Open Questions

- How accurate the analysed tempo is on real records (see the spike's recommendation).
