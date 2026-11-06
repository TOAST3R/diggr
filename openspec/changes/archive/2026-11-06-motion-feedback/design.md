# Design

## Context

This change builds on `motion-system`'s `motion` module (transitions keyed by id, easings, a
`Spring`, the durations, `animating` in `repaint_after`, and Reduce motion). The player is drawn
from a generated skin atlas: buttons are sprites drawn through a few helpers in `app.rs`, the
turntable's digits are segment sprites (`deck_target` and friends), the pitch fader's thumb is
placed from the target in `deck_section` (`fader_anchor` while dragged), and verdicts set
`DiggrApp::flash` (the word on the title line) and change the row's look in the playlist.

## Goals / Non-Goals

**Goals:**
- Every action answers within its first frame, and the motion that follows is under 300 ms.
- Feedback lives in the shared helpers (button, row, digit, count), not at every call site.

**Non-Goals:**
- New sounds or haptics.
- The level meter and beat lamp (`level-meter`).

## Decisions

**Stateless helpers keep their state in egui's context.** The skinned widgets are free functions
that don't hold the app, so their motion lives in `ctx.data` (temp) keyed by the widget's id, and
Reduce motion reaches them through a flag `Motion::begin_frame` puts there (`motion::reduced`).
`motion::press` (instant press-in, `FAST` spring-back, through egui's own timed bool animation),
`motion::lamp` (bloom and fade-out ages), `motion::afterglow` (segments that just went out) and
`motion::rolling` (a label's numbers rolling) are all of this kind. *Alternative:* threading the
app's `Motion` into every widget call: many signatures for no gain.

**Press and bloom in the button helpers.** `widgets::pressable` (used by `widgets::button` and the
app's `sprite_button`) draws the sprite a skin pixel lower while held. `widgets::lamp` (used by
`widgets::toggle`, `icon_toggle` and SYNC) draws, as a lamp lights, an `epaint::Shadow` glow in
`widgets::GLOW` behind it that shrinks and fades over `MEDIUM`, and as it goes out its `_on` sprite
over the `_off` one, fading over `FAST`. *Alternative:* glow sprites baked into the skin: atlas
space per lamp and no fade.

**Verdicts answer in the row painter.** `DiggrApp::answer_rows` notes (entry, answer, time) where a
verdict is applied (want, pass, adding to the cart), and the playlist's row painter reads it:
want draws a band of the want colour growing across the row, the cart flashes the row's
background, pass blends the row's colour to its dimmed look and moves the row's shapes aside and
back (`PaintList::transform_range` over the shapes it drew). The verdict itself is applied at once;
the title line's verdict word slides in over `FAST`.

**Rolling labels.** `motion::rolling` compares the label with the one it last drew: if only the
numbers differ (same text around them), each rolls from its old value to the new over `MEDIUM`, in
the new value's decimals, and the label is drawn brightened by a pulse that fades. Anything else
shows at once (so switching crates doesn't "roll" one name into another). It is used for the CART
switch, the filter bar's facet buttons ("STYLES 3") and the crate title's filtered count. What the
label means, and what any action reads, is always the new value.

**The list moves by shape ranges.** `motion::ListMotion` notes where each row's shapes start as the
row loop draws them, keyed by the row's identity (entry, record, copy). After the loop: if the crate
changed, the new rows' shapes slide 12 pt from the side of travel (by the crates' order in the
sidebar) and fade in, while last frame's list shapes, kept for this, fade out the other way over
`MEDIUM`; if the list changed under an unchanged scroll, rows whose place moved get a critically
damped spring from their old place. Scrolling never animates. Rows are hit-tested where they are
laid out.

**The fader.** The thumb's drawn place is a `Spring` on its fraction of the travel: set to the
target while dragged (or with Reduce motion), chasing it otherwise with ω = 35, ζ = 0.7 (about 5%
overshoot, settled in under 0.3 s). The tempo, the speed and the label change at once.

**Digit glow and afterglow.** `widgets::lcd_digit` draws the skin's seven-segment sprite, then a
faint fill of the LCD colour one skin pixel around each lit segment, and the segments that just went
out filled at a fading alpha for `FAST`. The segment layout comes from the skin generator
(`SEGMENTS`, `DIGIT_MASKS`), so the skin itself is unchanged.

## Risks / Trade-offs

- [Glow and blur cost GPU fill on every frame they are drawn] → Only while a bloom or fade runs, or
  for lit lamps (a few small rects); measure with `--startup-time` and a frame-time check while
  playing.
- [Row transitions for thousands of rows] → Transitions exist only for rows that changed in the
  last 300 ms; the map is pruned when they settle.
- [The playing record's row is redrawn by playback anyway] → Fine; the transition is time-based.
- [Paused turntable must ask for no repaint] → Springs and fades settle and drop out of
  `animating`; the paused test in `tempo-preview` still passes.

## Migration Plan

None: drawing only. Reduce motion turns it all off.
