## Context

`transport_key` (`crates/ui/src/app.rs`) maps Z X C V B to previous, play, pause, stop, next. `window_key` gives Space to the grouped playlist: it opens or closes the record under the cursor, only while the playlist has focus. ← and → always seek, which `record-view` states on purpose. The transport buttons are drawn by `widgets::button(ui, &sk, name, name, name)` with one sprite each, while shuffle, repeat and SYNC already pick an `_on` / `_off` sprite from their setting.

## Goals / Non-Goals

**Goals:**
- Space is play/pause wherever the player window has the keyboard.
- The lit button always matches `position.state`.
- Nobody loses a shortcut they already use, except Space on records.

**Non-Goals:**
- Rebinding keys from settings.
- Changing the digging keys (Y, N, I) or the structure keys.

## Decisions

### D1. Space toggles play/pause, Shift+Space stops
Stopping rewinds a 30 s preview to 0, so a Space that stops would lose the place. Pause is what Spotify, Rekordbox and YouTube do. From stopped, Space plays as X does (the cursor's entry when the playlist has focus and nothing is loaded, otherwise the current one).
*Alternative:* Space = play/stop, as first asked. Rejected after discussion.

### D2. → / ← open and close records with the playlist focused
Up/Down already split by focus (volume vs. cursor), so ←/→ splitting the same way is consistent, and it is the tree-view convention (Finder, file trees). With the playlist focused in a grouped crate: → on a closed record row opens it; ← on an open record row, or on one of its track rows, closes it and puts the cursor on the record row. Elsewhere (a flat crate, or the player focused) ←/→ seek as before.
*Alternatives:* Enter on a record row (taken: it plays the record); O; Shift+←/→ (hard to find). This reverses `record-view`'s "← and → SHALL keep seeking" in the grouped playlist; Tab back to the player seeks.

### D3. Lit sprites, generated
`skin-gen` draws `play_on`, `pause_on`, `stop_on` beside the existing ones (same frame, glyph in the lit colour shuffle uses). The transport loop picks `<name>_on` for the button matching `position.state` and keeps the pressed sprite on press. No hand-edited skin files.

### D4. Classic keys stay, quietly
Z X C V B keep working. The help panel's Playback group lists "Space: play / pause" and "Shift+Space: stop" first, then one line "Z X C V B: previous, play, pause, stop, next (classic)".

## Risks / Trade-offs

- [Users used to Space on records] → the help panel shows → / ←, and the record row's ⏵ tooltip names them.
- [Space reaching a focused egui button and clicking it] → `handle_keys` already returns early when egui wants the keyboard; the key handler consumes Space before widgets see it, as Tab is today.
- [No seek by arrows in a grouped playlist with focus] → Tab to the player, or click the seek bar.
