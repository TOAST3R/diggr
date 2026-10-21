## Why

Early users don't find play/pause: the keys are Winamp's bottom row (Z X C V B), and Space, the key every other player uses, opens and closes records instead. Nor does the player show at a glance whether it is playing, paused or stopped: the play, pause and stop buttons look the same in every state.

## What Changes

- **Space plays and pauses** anywhere in the player window: from stopped or paused it plays, while playing it pauses. **Shift+Space stops.**
- **Opening and closing records moves off Space**: with the playlist focused in a grouped crate, → opens the record under the cursor and ← closes it (the tree-view convention). **BREAKING** for users used to Space there.
- **Z X C V B stay** as aliases; the help panel lists Space and Shift+Space first and the classic keys as "also".
- **The transport shows its state**: the play button is drawn lit while playing, pause while paused, stop while stopped, as shuffle, repeat and SYNC already are, in the main window and in the maximized playlist's mini player.
- **The mini player gets the full transport and SYNC**: separate play and pause buttons instead of one toggle, and the SYNC button beside ⇔. Turning SYNC on there puts the turntable in the band, between the mini player and the waveform.
- **SYNC says what it does**: turning it on or off tells the user every track in the list now plays at the target BPM (or its own tempo again).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities
- `player-window`: Keyboard shortcuts gain Space and Shift+Space, lose Space for records; Transport and sliders lights the button of the current play state.
- `record-view`: Opening a record by keyboard moves from Space to → / ← with the playlist focused (arrows no longer seek there).
- `playlist`: Maximized playlist: the mini player has play, pause and SYNC.
- `tempo-preview`: SYNC control: the turntable's place while maximized, and the message on turning it on or off.
- `classic-skin`: Button states adds lit sprites for play, pause and stop.

## Impact

- **`crates/ui`:** `handle_keys` / `transport_key` / `window_key` in `app.rs`, the transport loop's sprite choice, `help.rs`.
- **Skin:** `skin-gen` draws `play_on`, `pause_on`, `stop_on`; `assets/skin/default/` is regenerated (its test checks it).
- **README:** the shortcuts table and the test count.
