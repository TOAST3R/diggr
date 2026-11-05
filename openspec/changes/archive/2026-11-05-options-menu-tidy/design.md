# Design

## Context

- The Options menu is built in one place, `DiggrApp::options_items`. Two places open it: the
  mini player's right-click (`options_menu`) and the playlist footer's gear (`pl_opts`). Today it
  holds the size toggle (`Action::ToggleScale`), Spectrogram (`Action::ToggleSpectrogram`), and
  Discogs… / Browser… / Set up Diggr… when there is digging.
- The size is `settings.scale` (1 or 2). Changing it calls `mark_settings`, which saves it. The
  band, the playlist, the EQ window and the spectrogram all read it every frame, so changing it
  takes effect on the next frame.
- The keys are gathered once per frame in the key loop (`app.rs` around line 1393). Cmd-modified
  keys skip `transport_key` and reach `window_key`.
- egui's `Options::zoom_with_keyboard` is on by default, and Diggr never turns it off. Today
  Cmd+= / Cmd+− / Cmd+0 change egui's `zoom_factor` in fractional steps, which blurs the
  nearest-neighbour skin and fights `settings.scale`.

## Goals / Non-Goals

**Goals:**
- Only the keys change the size, and only to 1× or 2×.
- The Options menu holds only digging and setup items.

**Non-Goals:**
- Sizes other than 1× and 2× (3×, fractional zoom).
- Choosing the size from the display on first launch, or a size button in the band.
- Renaming the Options menu, or changing where it opens from.

## Decisions

**1. Set the size, don't toggle it.** Cmd+= / Cmd++ set 2×, and Cmd+− / Cmd+0 set 1×. A new
`Action::SetScale(u8)` replaces `Action::ToggleScale`, and it only saves when the value changes.
A toggle on Cmd+= would shrink an app that is already at 2×, the opposite of what the key says.
With "set", pressing it again does nothing.

**2. Match both `Key::Equals` and `Key::Plus`, by logical key.** On US layouts "=" is unshifted
and "+" is Shift+=. On Spanish and other European layouts "+" has its own key and "=" is
Shift+0. Because both keys mean 2×, Cmd+= and Cmd++ work on every layout. Cmd+− is
`Key::Minus`, and Cmd+0 is `Key::Num0`. `mods.command` already means Cmd on macOS and Ctrl
elsewhere. Physical keys, as the brackets use, would be wrong here: users press what the key
says.

**3. Turn off egui's keyboard zoom.** Set `zoom_with_keyboard = false` on the context in
`DiggrApp::build`, so egui never changes `zoom_factor` and only our handler reads these keys.
Leaving it on would apply both zooms at once.

**4. Handle the keys in `window_key`,** next to W and S, so they work whichever of the player
and the playlist has focus. Like every app key, they are off while a text field (crate search)
has the keyboard.

**5. An empty menu doesn't open.** Without digging (the web build, or a native build without
`dig`), the menu would have no items. Then the mini player's right-click opens nothing and has
no "Right-click for options" tooltip, and the gear's slot stays empty (its place in the footer
layout is kept, so nothing shifts). Native builds always have digging, so on the desktop this
changes nothing.

## Risks / Trade-offs

- [The size is harder to find without the menu item] → The shortcuts help (H) lists the keys,
  and they are the keys every app uses for zoom.
- [Someone used egui's zoom on purpose for a size between 1× and 2×] → Unlikely: it blurred the
  skin, and the setting was never saved. The CHANGELOG entry gives the new keys.
- [Tests clicked "Spectrogram (S)" and "Double size (2×)" in the menu] → Rewrite them so the
  menu shows only the digging items, and add key tests for the size.

## Migration Plan

No settings format change: `settings.scale` keeps its meaning and its default. A rollback is a
plain revert.
