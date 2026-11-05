## 1. Before code (maintainer, and reading Discogs' documentation)

- [x] 1.1 Read Discogs' current OAuth documentation and API terms: the three endpoints, whether PLAINTEXT is accepted over HTTPS, whether a `http://127.0.0.1:‹port›/…` callback is accepted, the rate limit for OAuth requests, and what the terms say about a desktop app's consumer secret; record in `findings.md` in this change, and revise the design if any answer differs
- [x] 1.2 (maintainer) Register "Diggr" as an application in discogs.com ▸ Settings ▸ Developers, with the callback `https://diggr.es/discogs/callback`; the consumer key and secret are in the Keychain as `diggr-discogs-key` and `diggr-discogs-secret` (never in `findings.md`)
- [x] 1.3 A manual check with `curl`: request token (with `oauth_callback=https://diggr.es/discogs/callback?port=47800`), authorize in the browser, access token, then `/oauth/identity` signed with PLAINTEXT; record whether the per-request callback and its `?port=` came back, and what a Deny adds to the callback

## 2. Transport

- [x] 2.1 `Request.auth: Option<Auth>` (`Token` or `OAuth { token, secret }`); `headers()` builds `Discogs token=…` or the OAuth header (PLAINTEXT, nonce from `getrandom`, timestamp, percent-encoding); every caller and `Client` updated. Unit tests: both headers, encoding of awkward characters, a fresh nonce each time, no credential in `Debug` output
- [x] 2.2 The OAuth exchange (`request_token` with the callback, `access_token` with the verifier) through the transport seam, so tests use a fake Discogs. Tests for success, refusal and a malformed answer

## 3. The callback

- [x] 3.1 The bridge route `/discogs`, without a pairing key: it answers only while a connection waits, only for the matching `oauth_token`, once, and behind the host check; a `denied` answer ends the wait; anything else gets a 404. Tests with real local requests (no connection waiting, wrong token, the right one twice, a Deny, a foreign Host header)
- [x] 3.2 When the bridge isn't running, Connect with Discogs starts a one-time listener on `127.0.0.1:0` serving only `/discogs` with the rules of 3.1, passes its port in the callback, and stops after the answer, Cancel or 5 minutes; the waiting state shows "If your browser says it can't reach 127.0.0.1, make sure Diggr is open and click Connect with Discogs again". Tests with real local requests and the fake Discogs
- [x] 3.3 The callback page on diggr.es is live (its change in the diggr.es repository); a real Authorize reaches the bridge through it

## 4. Storage and UI

- [x] 4.1 `dig/oauth` stored like `dig/token` (atomic, 0600, fail soft); saving one removes the other; Disconnect removes both. Tests with `TestDir`
- [x] 4.2 Options ▸ Discogs…: Connect with Discogs first, "Use a personal token instead" below; waiting state with Cancel; the errors from the spec; the Connect to Discogs dialog and the setup panel's Discogs step (`welcome-setup`) use the same flow. Headless tests with the fake Discogs

## 5. Release builds

- [x] 5.1 The consumer key and secret through `option_env!`; without them Connect with Discogs is hidden. One helper reads them from the Keychain for `make-dmg.sh` and `make-windows.sh`, `make-linux.sh` passes them with `docker run -e`, and `scripts/release.sh` stops if either is missing. Check: `strings` finds the key in a release binary and not in a plain `cargo build`

## 6. Docs and checks

- [x] 6.1 README: connecting with Discogs, the personal token as the alternative, where the credentials are kept; the test count
- [x] 6.2 CHANGELOG `## Unreleased` with the `changelog` skill
- [ ] 6.3 The four checks from AGENTS.md and `cargo xwin check`
- [x] 6.4 A real connection on the Mac and on Windows (the bridge route and the one-time listener raise no firewall prompt); record in `findings.md`
