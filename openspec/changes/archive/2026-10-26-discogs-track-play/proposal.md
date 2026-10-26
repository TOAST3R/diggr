## Why

On a Discogs release page the extension only offers Play in ‹App› for the whole record, which starts at its first track. A digger reading a tracklist usually wants to hear one track, the B2 everyone talks about, and today that means sending the record, switching to the app and finding the row. Pressing play on the track in the page, and stop, is the smallest remote that saves the trip.

## What Changes

- **A ▶ before each track of a Discogs release page's tracklist.** Clicking it plays that track in the app: the release is sent like Play in ‹App› (added to the end of the crate on screen) and the entry with that side plays, instead of the release's first one. When the crate already holds the release, nothing is added and that track plays at once.
- **The playing track's ▶ becomes ■**, which stops the app's playback. A clicked track shows ⧗ until it plays; if it hasn't played after 30 s (no clip found, say), it goes back to ▶. The marks follow the app: when the track ends, another plays or the app stops, the page shows it within about a second.
- **Bridge:** a Play send can name one `position` of a release page; the status names the playing entry's release and side; a new `/v1/control` request stops playback.
- Nothing else on the page changes: the page button, the owned note and the ▶ after links on other pages stay as they are. No bar, no seek, no pause, no want/pass from the page.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chrome-extension`: new requirement "Play each track of a release page" (▶ / ⧗ / ■ on the tracklist rows).
- `browser-bridge`: "Crates and status" includes the playing entry's release and side; new requirements "Play one track of a release" (a Play send with a position) and "Stop" (`/v1/control`).

## Impact

- **`crates/dig/src/bridge/mod.rs`:** `SendBody.position` (release pages with mode Play only), `BridgeCommand::Send` carries it, `Playing { release, position }`, `/v1/control` → `BridgeCommand::Stop`; tests in `crates/dig/tests/bridge.rs`.
- **`crates/ui/src/app/digging.rs`:** `play_when_ready` can target (release, position) instead of the first new entry, and plays an entry already in the crate; the snapshot fills the two new `Playing` fields; `BridgeCommand::Stop` stops the engine.
- **`extensions/chrome/`:** `content.js` (row buttons, status poll while a row is ⧗ or ■), `background.js` (`control` message, `position` on send).
- **Docs:** README (extension section, manual checklist, test count), CHANGELOG.
- No change to playback, analysis, previews or the skin.
