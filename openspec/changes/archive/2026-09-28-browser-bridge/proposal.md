## Why

After `discogs-digging`, a Discogs page reaches the player by copying its address and pasting it. But digging happens in the browser. A button on Discogs pages (Play in, Enqueue in, Send to crate), and a right-click on any Discogs link, turn the browser into a remote control for the player: going from a page to a crate takes one click.

## What Changes

- **A local bridge in the app:** a small HTTP server that listens only on 127.0.0.1, started once the window is up.
  - It accepts only Discogs page addresses, a mode, a crate name and the two filters, never file paths or commands.
  - It hands each request to the same send path as pasting.
  - It answers at once, while the crate fills in afterwards.
- **Pairing:** OPT ▸ Browser… shows a 6-digit code, valid for 2 minutes.
  - The extension asks for the code once, and receives a long random key that it sends with every request.
  - Web pages can't use the bridge.
  - "Forget browsers" revokes every key.
- **Chrome extension:** Manifest V3, in `extensions/chrome/`, loaded unpacked for now. Chromium browsers such as Brave, Edge and Arc load it the same way.
  - **Page button:** on Discogs release, master release, artist, label, wantlist and list pages, a button next to the page title. Its menu has Play in ‹App›, Enqueue in ‹App› and Send to crate ▸ (the app's crates, and New crate…), plus the "vinyl only" and "skip passed" switches.
  - **Links:** right-clicking any Discogs link on any site offers Play in ‹App› and Enqueue in ‹App›.
  - **Feedback:** a short confirmation on the page ("Sent to ‹App›: Label: Lowtide Tapes"). The toolbar button shows whether the app is running and paired, what is playing, and sends in progress. The extension says clearly when the app isn't running, isn't paired, or the page isn't supported.
  - **Minimal access:** discogs.com and 127.0.0.1 only. It contacts no other server, and it has no logic beyond sending the address; the app does all the parsing.
- **App name:** the app reports its name to the extension, so the labels ("Play in ‹App›") follow one constant in the app. The name is a placeholder until you choose the rebrand name.

## Capabilities

### New Capabilities
- `browser-bridge`: the loopback server, pairing and keys, what it accepts and refuses, how fast it answers, and the crates and status it exposes.
- `chrome-extension`: the page button and its menu, the link right-click, feedback and the toolbar status, pairing, minimal access, and the app name taken from the app.

### Modified Capabilities
(none: sending a page is `discogs-intake`'s behaviour, and the bridge only calls it)

## Impact

- **`crates/dig`:** a `bridge` module: the tiny_http server thread, request checks, pairing and key storage.
- **`crates/ui`:** starting the bridge after the first frame, the OPT ▸ Browser… dialog, a snapshot of crates and status for the extension, and passing sends to the send path.
- **`extensions/chrome/`:** manifest, service worker, content script, options (pairing) page, toolbar popup and icons (original art). Plain JavaScript, with no build step and no dependencies.
- **Files:** `<config>/dig/bridge.ron`, holding the port and hashes of the paired keys, readable only by the user.
- **Dependencies:** `tiny_http`, `getrandom` and `sha2`.
- **Depends on** `discogs-digging` (send path, address parsing) and `crates` (crate list). No change to `audio` or `platform`.
