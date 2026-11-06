# Design

## Context

The UI is egui/eframe, immediate mode: every frame draws everything from the app's state. Today
the only transition is the record card's slide (`app/card.rs`, `SLIDE` 0.22 s and its own
`ease`), plus one hover fade in insights. Context menus use `egui::Popup::context_menu` (about 15
call sites in `app.rs`), in-window dialogs are `egui::Window`s (`app/digging.rs`, `app.rs`), and
EQ, spectrogram and insights are separate OS windows through `show_viewport_immediate`.
`repaint_after(Activity)` (`app.rs`) decides how often the main window repaints: 16 ms while
playing, nothing when idle. The headless test `Rig` drives frames and reads the painted shapes.

## Goals / Non-Goals

**Goals:**
- One small module that every animation in the app uses, so timings and feel stay consistent.
- Opening and closing motion for menus, dialogs, panels and separate windows.
- Zero change to what an action does or when: motion is drawing only.

**Non-Goals:**
- Feedback for presses, verdicts, counters and the pitch fader (`motion-feedback`).
- Level meter and beat lamp (`level-meter`).
- Animating the OS window frame itself (opening an EQ window still appears at once; its
  content fades).
- Following the operating system's reduce-motion preference (egui doesn't expose it).

## Decisions

**One pass over egui's layers, not a change at every call site.** Every popup, context menu and
`egui::Window` draws into a layer of its own. After the frame's UI, `Motion::end_frame` takes
the area layers egui reports visible in the `Middle` and `Foreground` orders that were painted this
frame. A layer it hasn't seen before starts opening; each frame its shapes are moved and faded by its
progress (`PaintList::transform` and the colours multiplied), and its look is kept. A layer it saw last
frame and doesn't see now has closed: its kept look is painted, fading out, into a layer that no
`Area` owns, so it takes no input. Tooltips (`Order::Tooltip`) are left alone. *Alternative:* wrapping
each of the ~25 popups and windows: more code, and every new menu would have to remember it.

**Kinds by order, with a few named exceptions.** Foreground layers (menus, popups) move as `Popup`
(fade and 4 pt rise), Middle layers (windows) as `Dialog` (fade and 96%→100% about the centre).
`new_motion` in `app.rs` names the exceptions: the filter panels (`filters_panel` and each facet's
list) slide 12 pt as `Panel`; the record card is a `Dialog` and its dark layer a `Fade`; the EQ
floating over the playlist where there are no separate windows is a `Fade`.

**The module.** `crates/ui/src/motion.rs`: the durations (`FAST` 0.12, `MEDIUM` 0.18, `SLOW` 0.24 s;
closing about two thirds), `ease_out_cubic` and `ease_in_out`, a damped `Spring` with fixed sub-steps
(for `motion-feedback`), `Motion::fade` (progress by key, continuous when it turns around), and the
layer pass. Time is `ctx.input().time`, so the headless tests step it by frames.

**Shapes move, input doesn't.** egui hit-tests against where widgets were laid out, and the pass
changes only the painted shapes after layout, so an opening menu takes clicks at its final place
from its first frame (the 4 pt it is drawn off by never leaves a menu row). *Alternative:* moving the
area itself: input would follow the moving rect.

**egui's own fade is turned off.** egui fades areas in over `style.animation_time` (0.2 s); with
that and this module both, things would fade twice. `begin_frame` sets it to 0.

**Separate windows fade their content.** egui cannot set an OS window's alpha, so the EQ,
spectrogram and insights windows draw their content at `Motion::fade`'s progress over 180 ms
(`ui.multiply_opacity`); the spectrogram, a deferred viewport, is handed the opacity. Where
viewports are embedded (tests, the web) they are areas of the main window, and the layer pass moves
them instead.

**The record card.** Its record-to-record slide keeps `SLIDE` (0.22 s) and uses the module's
`ease_out_cubic`; opening and closing it is the layer pass's `Dialog`.

**Repaint.** `Activity` gains `animating`. While animating, `repaint_after` returns
`Duration::ZERO` (repaint next vsync, so 120 Hz displays get every refresh); otherwise the current
rules stand. When everything settles, `animating` is false and an idle window repaints nothing.

**Reduce motion.** `Settings::reduce_motion: bool` (`serde(default)`, so old files load as off).
When on, `fade` returns 0 or 1 at once and the layer pass does nothing. It is a checkbox at the end
of the Options menu, which exists where digging is available (see `player-window` "Options menu").

**Tests.** The headless `Rig` turns `reduce_motion` on after building the app, so the existing UI
tests keep reading final positions in the frame after a click. A `motion_tests` module turns it off and
covers each scenario of `ui-motion`, stepping `ctx` time by fixed frames.

## Risks / Trade-offs

- [The pass keeps a copy of every open popup's and window's shapes each frame, for its fade-out] →
  Shapes hold their text and meshes behind `Arc`s, so a copy is a few hundred small clones while
  something is open, and nothing when nothing is.
- [An area that is drawn some frames and not others would flicker in and out] → None of the app's
  areas do; one that did would show as a fade, not a gap.
- [Repainting at vsync while animating costs GPU time] → Only for the 80–240 ms an animation
  runs; idle stays at zero.
- [Existing tests run with motion off, so they don't exercise it] → The motion tests run the main
  flows (menus, dialogs, panels) with motion on, including clicks on the first frame.

## Migration Plan

`reduce_motion` is new with a default; nothing to migrate. Rolling back is removing the module's
call sites.
