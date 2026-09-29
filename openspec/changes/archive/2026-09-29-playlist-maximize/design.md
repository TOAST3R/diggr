## Context

- Today the window's size is forced by the layout: every frame the app computes `window_size(settings)` and sends `InnerSize` when it changes. The player column is fixed at the skin's main width × scale, and the playlist sits to its right (`layout.rs`).
- The window is borderless (`with_decorations(false)`).
- **Spike result (macOS, egui/eframe 0.36):** on a borderless window, `ViewportCommand::Maximized(true)` fills the screen's usable area, (0, 30) to (1440, 900) on a 1440 × 900 display (below the menu bar), and reports `maximized = true`. `Maximized(false)` restores the exact previous frame.
- Visuals fullscreen (`F`) saves the outer rect, goes `Fullscreen(true)`, and on leaving restores `InnerSize(window_size)` and the position.
- The playlist's wheel handling rounds each frame's `smooth_scroll_delta` to whole rows and drops the remainder, so trackpad deltas of a few points never scroll.

## Goals / Non-Goals

**Goals:**
- A playlist that can use the whole screen, with the player reduced to what's needed to steer playback.
- An exact round trip back to the normal layout.
- Trackpad scrolling that follows the fingers.

**Non-Goals:**
- A separate playlist window.
- Crate switching by swipe, and pinch to zoom the playlist (both deferred).
- A now-playing title in maximized mode (the playing row is highlighted).
- Volume in the strip (↑/↓ with the player focused still set it).

## Decisions

**1. Maximize through the OS, not by computing a size.**
Use `ViewportCommand::Maximized`. The OS knows the usable area (menu bar, Dock, notch, the monitor the window is on) and restores the old frame itself.

Rejected alternatives:
- macOS full screen: its own Space and an animation, and `F` already means full screen here.
- Sizing from `monitor_size`: it doesn't know the menu bar or the Dock.

**2. While maximized, the layout follows the window.**
The window-size loop doesn't send `InnerSize` while maximized, and the layout is computed from the window's inner size:
```
strip_w   = STRIP_W (27 skin px)
list area = window width / scale − strip_w, window height / scale
band      = waveform::HEIGHT when the waveform shows (0 otherwise)
rows      = (height − band − pl_top_h − pl_bottom_h) / pl_row_h, at least 4
```
These are per-frame values. The saved `playlist_width` and `playlist_rows` stay untouched, so restoring brings the user's normal size back. The resize grip does nothing while maximized.

**3. The strip.**
It's a column of existing sprites on a stretched panel background:
- the play-state LED;
- the elapsed time as two stacked pairs of LCD digits (mm, ss);
- prev, then play or pause (whichever applies), then next;
- ⇔ at the bottom.

It's 27 skin px wide, since the transport sprites are 23. A click on the strip gives the player focus, as the main window does. The strip needs no new sprites except the ⇔ button (`btn_max`, `btn_max_p`), which is also drawn in the playlist title bar, left of close.

**4. The band.**
When the waveform shows, `waveform::draw` gets a rect across the whole list area, above the playlist's title bar. It keeps the same overview row and zoomed row, seeking and zoom. No new drawing code.

**5. Entering and leaving.**
Enter:
- if the playlist is hidden, show it;
- set `playlist_maximized`;
- set focus to the playlist;
- send `Maximized(true)`.

Leave:
- clear the flag;
- send `Maximized(false)`;
- reset `last_size`, so the normal layout's size is enforced again once the OS has restored the frame.

If the OS un-maximizes the window (a title-bar double-click can't happen, since it's borderless, but a window manager might), the app leaves the mode when `viewport().maximized` turns false.

**6. Visuals fullscreen from maximized.**
Leaving visuals fullscreen normally restores `InnerSize(window_size)`. While maximized, it sends `Maximized(true)` instead of a size.

**7. Launch.**
When `playlist_maximized` is saved, `apps/native` builds the viewport `with_maximized(true)`, so the first frame is already maximized. Check that `--startup-time` stays under 300 ms.

**8. Scroll accumulator.**
Add `pl_scroll_acc: f32` in points. Each frame adds `smooth_scroll_delta.y`, moves whole rows while `|acc| ≥ row height`, and keeps the remainder. It resets when the pointer leaves the list or the list reaches an end, so leftovers don't jump later. Mouse wheels, which send bigger steps, behave as before.

## Risks / Trade-offs

- [Maximize behaves differently on Windows or Linux window managers] → it follows the viewport's reported `maximized` state. If a platform refuses, the mode still works at the current window size, since the layout follows the window.
- [The monitor changes while maximized] → the OS resizes the window, and the layout follows.
- [Losing track of what's playing without the title line] → the playing row is highlighted, `P` scrolls to it, and the band shows the track's waveform.
- [An old settings file] → `playlist_maximized` defaults to false.
