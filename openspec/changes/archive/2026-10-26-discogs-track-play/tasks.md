## 1. Tracklist markup

- [x] 1.1 On a live release page (5077187, plus one with headings and one with sub-tracks), find a selector for the tracklist rows and each row's position cell that uses no hashed class names; note it in design.md's Open Questions and close them

## 2. Bridge

- [x] 2.1 `SendBody.position`: optional, 1–8 characters, no control characters, accepted only with mode Play on `PageKind::Release` (422 otherwise); `BridgeCommand::Send` carries it; tests in `crates/dig/tests/bridge.rs` (accepted on a release, refused on a label, with Enqueue, too long)
- [x] 2.2 `Playing` gains `release: Option<u64>` and `position: String`, serialised in `/v1/status`; test that they are answered
- [x] 2.3 `POST /v1/control` with `{"action":"stop"}` → `202` and `BridgeCommand::Stop`; unknown action or field → 422; unpaired → 401; tests, including the 100 ms answer

## 3. App

- [x] 3.1 `play_when_ready` targets either the first new entry (today) or `(release, position)`; a track target plays the crate's matching entry, new or already there (positions trimmed, case ignored), and nothing when the send ends without one; unit tests for added, already held and missing side
- [x] 3.2 `dig_bridge_snapshot` fills `Playing.release` and `position` from the playing entry's origin (none and empty when it has none)
- [x] 3.3 `BridgeCommand::Stop` does what the Stop button does (`engine.stop()`); test that the status then names nothing playing

## 4. Extension

- [x] 4.1 `background.js`: `send` passes `position`; new `control` message → `POST /v1/control`
- [x] 4.2 `content.js`: on release pages only, a ▶ before each row with a position (selector from 1.1), in the button's shadow-DOM style; click sends Play with the position and shows ⧗; ■ sends stop; feedback on not running / not paired as the page button's
- [x] 4.3 `content.js`: poll `status` every 1 s only while a row is ⧗ or ■; ■ on the row whose position matches the status's release and position, ⧗ back to ▶ after 30 s; no request while idle
- [x] 4.4 Run the manual checklist: play a track of a new release, of a release already in the crate, stop from the page, stop and next in the app, a track with no preview, a heading row, app not running

## 5. Docs and checks

- [x] 5.1 README (extension section, `/v1/control` and the `position` field, manual checklist, test count) and CHANGELOG `## Unreleased` (changelog skill); bump the extension's version in `manifest.json`
- [x] 5.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check
