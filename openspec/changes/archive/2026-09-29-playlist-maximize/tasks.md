## 1. Scroll fix

- [x] 1.1 Playlist scroll accumulator (keeps partial steps; dropped when the pointer leaves or at an end); headless tests for the slow scroll and no-jump scenarios

## 2. Mode and geometry

- [x] 2.1 Settings: `playlist_maximized` (default false); a test that old settings load
- [x] 2.2 `layout.rs`: the maximized geometry from a window size (strip width, list width, rows with and without the band); unit tests
- [x] 2.3 Toggle action: show the playlist if hidden, focus the playlist, `Maximized(true)` / `Maximized(false)`, `last_size` reset; skip `InnerSize` while maximized; leave the mode if the viewport reports not maximized; headless tests for the flag, focus and viewport commands
- [x] 2.4 Shift+P and the ⇔ button in the playlist title bar (skin-gen `btn_max`, `btn_max_p`; regenerate the skin)

## 3. Drawing

- [x] 3.1 The strip: play-state LED, mm/ss LCD digits, prev, play or pause, next, ⇔; clicks give the player focus; headless test that next in the strip starts the next track
- [x] 3.2 The playlist from the window size while maximized (width, rows, columns); the grip does nothing
- [x] 3.3 The full-width waveform band above the playlist when the waveform is on; test that a click in it seeks
- [x] 3.4 Visuals fullscreen round trip: leaving `F` while maximized re-maximizes

## 4. Launch, docs and checks

- [x] 4.1 `apps/native`: start `with_maximized(true)` when the setting is on; `--startup-time` stays < 300 ms
- [x] 4.2 Help panel (Shift+P, ⇔) and README (maximized playlist, trackpad scrolling, test count)
- [x] 4.3 Run the app to check maximize and restore, the strip and the band; fmt, clippy, the workspace tests and the wasm check
