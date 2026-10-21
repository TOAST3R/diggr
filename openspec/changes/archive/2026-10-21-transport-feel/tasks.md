## 1. Keys

- [x] 1.1 Space toggles play/pause and Shift+Space stops in `transport_key`, consumed before egui widgets; unit tests through the headless rig (playing → paused → playing; stopped → playing; Shift+Space → stopped)
- [x] 1.2 Remove Space from `window_key`; → opens and ← closes the record under the cursor with the playlist focused in a grouped crate (cursor to the record row on close); ←/→ still seek with the player focused or a flat crate; rig tests for each
- [x] 1.3 `help.rs`: Space and Shift+Space first in Playback, Z X C V B on one "classic" line, → / ← for records, Space gone from the player-window group (the ⏵ mark is painted and has no tooltip, so the help carries it)

## 2. Lit transport

- [x] 2.1 `skin-gen` draws `play_on`, `pause_on`, `stop_on`; regenerate `assets/skin/default/`; the skin test passes
- [x] 2.2 The transport loop draws `<name>_on` for the button matching `position.state`; the mini player draws the same five buttons (play and pause apart), lit the same way (no rig test: the rig cannot read drawn sprites, and the mapping is one match)

- [x] 2.3 Mini player: SYNC button left of ⇔, volume moved right to fit; rig test clicks pause, play, SYNC and next
- [x] 2.4 Maximized: the turntable sits right of the mini player while SYNC is on, the band grows to its height (`layout::band_h`), rows follow; SYNC on/off says what it does; layout and rig tests

## 3. Docs and checks

- [x] 3.1 README shortcuts table and test count
- [x] 3.2 `cargo test --workspace`, clippy `-D warnings`, `fmt --check`, wasm check of `audio` and `platform`
