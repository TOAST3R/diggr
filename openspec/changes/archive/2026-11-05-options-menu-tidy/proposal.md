# Proposal

## Why

The Options menu mixes view toggles with account setup. **Spectrogram (S)** repeats the mini
player's spectrum button and the S key. **Double size / Classic size** is the only way to change
the size, and nobody looks for a zoom under a gear. Every desktop app has its zoom on
Cmd+= / Cmd+− / Cmd+0. Those keys already do something in Diggr: egui's built-in keyboard zoom,
left on, scales the whole UI by fractional steps and blurs the pixel skin.

## What Changes

- Remove **Spectrogram (S)** from the Options menu. The mini player's spectrum button and S
  still open and close the spectrogram window.
- Remove **Double size (2×) / Classic size (1×)** from the Options menu.
- New keys: Cmd+= (or Cmd++) switches to double size (2×), Cmd+− to classic size (1×), and
  Cmd+0 back to classic size. On Linux and Windows they use Ctrl. The size is still saved in
  `settings.scale`, and a key that asks for the current size does nothing.
- Turn off egui's own keyboard zoom, so the skin is only ever drawn at whole-number scales.
- The Options menu (right-click on the mini player, or the footer's gear) keeps Discogs…,
  Browser… and Set up Diggr….
- The shortcuts help (H) lists the size keys. README and CHANGELOG follow.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `player-window`: "Options menu" loses Double size and Spectrogram. "Keyboard shortcuts" gains
  Cmd+= / Cmd+− / Cmd+0 for the size.
- `spectrogram-window`: "Open the spectrogram" no longer lists the Options menu as a way to
  open it.

## Impact

- `crates/ui/src/app.rs`: `options_items` (drop two items), `window_key` or the key loop (size
  keys), egui options at startup (`zoom_with_keyboard = false`), and the tests that open the
  spectrogram or change the size through the menu (around `app.rs:10860`).
- `crates/ui/src/help.rs`: the "Mini player: right-click" row and a new size row in the window
  keys.
- `README.md` (the Options description, Spectrogram section, keys) and `CHANGELOG.md`.
- No settings format change, no skin change, and nothing on the playback path.
