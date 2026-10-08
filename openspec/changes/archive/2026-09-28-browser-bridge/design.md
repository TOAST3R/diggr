## Context

- **Send path:** `discogs-digging` gives the app one entry point for sending a page: an address, a mode (Play, Enqueue, Crate), an optional crate and two filters. Paste already uses it, and address parsing lives in `dig::discogs`.
- **Crates:** `crates` gives the list of crates and the shown and playing ones.
- **Workers:** dig workers are threads that talk to the UI over channels with a wake callback. The app has no network listener today; this is the first.
- **Launch budget:** interactive in under 300 ms. Nothing new may run before the first frame.

## Goals / Non-Goals

**Goals:**
- One click from a Discogs page or link to a crate, with an answer in under 100 ms.
- A bridge that only this computer's paired browsers can use, and only to send Discogs references.
- An extension simple enough to review in one sitting: no build step, no dependencies, no logic beyond sending the address.

**Non-Goals:**
- Starting the app from the browser when it's closed (a later option, with native messaging).
- Firefox and Safari, and publishing on the Chrome Web Store (it needs the rebrand name).
- Search, seller and marketplace pages (they'd need reading the release list off the page).
- Controlling playback from the browser.

## Decisions

### D1. Protocol
All requests go to `http://127.0.0.1:<port>/v1/…` with JSON bodies. The key travels in `X-Bridge-Key`.

| Request | Key | Answer |
|---|---|---|
| `GET /v1/hello` | no | `{"app": "<name>", "api": 1}` |
| `POST /v1/pair` `{"code": "123456"}` | no | `200 {"key": "<64 hex>"}`, `401` wrong code, `410` no valid code, `429` locked |
| `GET /v1/crates` | yes | `{"crates": [{"name": "…", "shown": true, "playing": false}]}` |
| `GET /v1/status` | yes | `{"app": "…", "playing": {"artist", "title", "crate"} \| null, "sends": [{"page", "done", "total"}]}` |
| `POST /v1/send` `{"url", "mode": "play"\|"enqueue"\|"crate", "crate"?, "vinyl_only", "skip_passed"}` | yes | `202 {"page": "Label: Lowtide Tapes", "crate": "…"}`, or `422 {"error": "unsupported"}` |

- **Errors:** `401` (no key or a wrong key), `403` (web origin, wrong host, preflight), `404` (unknown route), `413` (body over 16 KB), `422` (invalid body or unsupported address).
- **Parsing:** bodies are parsed with `#[serde(deny_unknown_fields)]`.

### D2. Server and threads
- **Server:** `tiny_http` on a `Spawner` thread at `Priority::Low`, bound to `127.0.0.1:<port>`. It is started by the UI after the first frame. If binding fails, the error is shown in OPT ▸ Browser… and the app runs without the bridge.
- **Validation:** each request is checked on the bridge thread, including the Discogs address, through `dig::discogs::url`. A valid send becomes a `BridgeCommand::Send`, sent to the UI over a channel with a wake. The answer (`202`) goes back at once, with the page's provisional name from its address and the target crate's name.
- **Snapshot:** the UI keeps an `ArcSwap<BridgeSnapshot>` up to date whenever it changes: the crates with shown and playing, what is playing, and send progress. `/v1/crates` and `/v1/status` read it without asking the UI, so the 100 ms answer never depends on the frame rate.

*Alternatives:*
- Chrome Native Messaging: rejected for now. Each browser needs a host manifest installed, a relay process sits in between, and the app would still need its own local channel. It becomes attractive only for starting the app from the browser.
- A custom URL scheme: rejected. It is one-way, gives no answer or crate list, and needs registration with the OS.
- WebSockets: not needed. The popup asks for the status when it opens.

### D3. Checks, in order
1. **Host:** must be `127.0.0.1:<port>` or `localhost:<port>`. This defeats DNS rebinding, where a web page's own host name points at 127.0.0.1.
2. **Origin:** absent, or `chrome-extension://…`. Any `http` or `https` origin is refused. `OPTIONS` is always refused, and no `Access-Control-Allow-*` header is ever sent. The extension doesn't need them, because its site access to 127.0.0.1 exempts it from cross-origin rules.
3. **Route and method**, then the size (`Content-Length` at most 16 KB).
4. **Key**, except for `hello` and `pair`.
5. **JSON shape**, then the Discogs address and the crate name (1 to 40 characters).

### D4. Pairing and keys
- **Code:** 6 random digits from `getrandom`. It expires after 2 minutes or on first use, and a new one is shown when the dialog opens or its countdown ends. After 5 wrong codes, pairing is locked for 60 s.
- **Key:** 32 random bytes, sent as 64 hex characters.
- **Storage:** `<config>/dig/bridge.ron` holds the port and the SHA-256 of each key, written atomically and readable only by the user. Keys are compared in constant time.
- **Forget browsers** empties the list; paired extensions then get `401` and show the pairing screen.

### D5. The extension
```
extensions/chrome/
  manifest.json   MV3: permissions storage, contextMenus; action;
                  host_permissions https://www.discogs.com/*, https://discogs.com/*, http://127.0.0.1/*
  background.js   service worker: bridge calls (key header, app name from /v1/hello, cached),
                  link context menus, toolbar badge
  content.js      discogs.com: page kind from the address, button and menu, toast
  options.html/js pairing (code → key) and port
  popup.html/js   toolbar popup: running, paired, now playing, sends in progress
  icons/          original art
```
- **Page detection:** a coarse check of the address (the kinds from `discogs-intake`, with an optional language prefix). The app does the real parsing and refuses anything else with `422`. Discogs changes pages without reloading, so the script watches address changes and updates the button.
- **Button:** placed after the page's main `h1`, or floating in the bottom-right corner when there is none. It uses the skin's colours: dark panel, green LCD text. The switches are kept in `chrome.storage.local`, on by default. The crate list is fetched when the menu opens.
- **Links:** context-menu items for links matching the supported address patterns (`targetUrlPatterns`). Feedback there is the toolbar badge (✓ or !, for 3 s), since the extension has no access to other sites' pages.
- **Name:** labels use the app's name from `/v1/hello`, cached in storage, with the placeholder until the app has answered once.

### D6. The app's name
`dig::APP_NAME` is one constant, Diggr. `/v1/hello` returns it. The manifest's `name` is Diggr too. Publishing under another product's trademarked name would not pass Chrome Web Store review.

### D7. Tests
- **`crates/dig/tests/bridge.rs`:** runs the server on an ephemeral port, with `ureq` as the client, a fake UI side (the channel) and a slow fake Discogs transport. It covers:
  - `hello`, pairing, expiry, lockout and single use;
  - keys, and a key revoked by Forget browsers;
  - Host and Origin refusals, `OPTIONS`, and the absence of any `Access-Control-*` header;
  - the size limit and unknown fields;
  - addresses that aren't Discogs pages;
  - the snapshot answers;
  - a `202` in under 100 ms while Discogs is slow.

  Loopback in tests stays within "no network and no hardware".
- **The extension** has no build step and almost no logic, so it is checked by hand with the checklist in the README: load unpacked, pair, each page kind, a forum link, the app closed, a revoked key, and a hostile local HTML page trying to call the bridge.

## Risks / Trade-offs

- [Discogs changes its page markup] → The only thing the extension looks for is the main heading, with a floating button as the fallback. Everything else comes from the address.
- [Something else uses the port] → The port can be changed, the dialog says what happened, and the app works without the bridge.
- [Malware on the computer can call the bridge] → Out of scope, since it already runs as the user. Keys are stored as hashes, and the bridge only accepts Discogs references.
- [Chrome tightens local-network access] → Extension requests with site access to 127.0.0.1 are exempt today. The manual checklist re-checks this.
- [Publishing later] → It needs the rebrand name, a short privacy note (the extension sends only Discogs addresses, to the local app), and store review. Starting the app from the browser (native messaging) would be a separate change.
