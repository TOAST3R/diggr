## 1. Bridge server (`dig::bridge`)

- [x] 1.1 Add `tiny_http`, `getrandom` and `sha2`, and run the server on a `Spawner` thread bound to 127.0.0.1 (configurable port, "port taken" reported) with the routes from design D1 and strict JSON bodies; verify with an integration test on an ephemeral port
- [x] 1.2 Implement the checks from design D3 (Host, Origin, `OPTIONS` refused, no cross-origin headers, 16 KB limit, key, Discogs addresses and crate names only); verify with one integration test per refusal
- [x] 1.3 Implement pairing and keys: a single-use 6-digit code valid for 2 minutes, lockout for 1 minute after 5 wrong codes, 256-bit keys stored as SHA-256 hashes in `<config>/dig/bridge.ron` (readable only by the user), constant-time checks, and Forget browsers; verify with integration tests, and add `bridge.ron` to the README's "Where files are kept" table

## 2. App wiring (`ui`)

- [x] 2.1 Start the bridge after the first frame, forward sends to the `discogs-intake` send path, and keep the crates and status snapshot up to date; verify with a test that a send is answered within 100 ms while the fake Discogs transport is slow, and that the crate fills in afterwards
- [x] 2.2 Build OPT ▸ Browser…: pairing code with its countdown, number of paired browsers, port, Forget browsers, and bridge errors; verify with headless tests, and document pairing and what the bridge accepts in the README

## 3. Chrome extension (`extensions/chrome/`)

- [x] 3.1 Write the MV3 manifest (storage, contextMenus, action, and site access to discogs.com and 127.0.0.1 only), the service worker (bridge calls with the key, app name from `/v1/hello`) and the options page (pairing, port); verify by loading it unpacked in Chrome and pairing with the app
- [x] 3.2 Write the content script: page detection that follows in-page navigation, the button after the main heading (floating as a fallback), the menu (Play in, Enqueue in, Send to crate with the app's crates and New crate…, the two remembered switches), the confirmation toast, and messages for not running, not paired and unsupported; verify with the manual checklist on every supported page kind
- [x] 3.3 Add the link context menus for any site, the toolbar badge (✓ or ! for 3 s) and the toolbar popup (running, paired, now playing, sends in progress); verify with the manual checklist
- [x] 3.4 Write the README section: load unpacked, pair, the placeholder name, and the manual checklist; verify by following it from scratch in a fresh Chrome profile

## 4. Integration checks

- [x] 4.1 Confirm `winamp-native --startup-time` stays under 300 ms with the bridge enabled
- [x] 4.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check; confirm the README's test count matches
