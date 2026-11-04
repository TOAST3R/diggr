## 1. Before code (maintainer, and reading Discogs' documentation)

- [ ] 1.1 Read Discogs' current OAuth documentation and API terms: the three endpoints, whether PLAINTEXT is accepted over HTTPS, whether a `http://127.0.0.1:‹port›/…` callback is accepted, the rate limit for OAuth requests, and what the terms say about a desktop app's consumer secret; record in `findings.md` in this change, and revise the design if any answer differs
- [ ] 1.2 (maintainer) Register "Diggr" as an application in discogs.com ▸ Settings ▸ Developers; note the consumer key and secret for the app (never in `findings.md`)
- [ ] 1.3 A manual check with `curl`: request token, authorize in the browser, access token, then `/oauth/identity` signed with PLAINTEXT; record what worked

## 2. Transport

- [ ] 2.1 `Request.auth: Option<Auth>` (`Token` or `OAuth { token, secret }`); `headers()` builds `Discogs token=…` or the OAuth header (PLAINTEXT, nonce from `getrandom`, timestamp, percent-encoding); every caller and `Client` updated. Unit tests: both headers, encoding of awkward characters, a fresh nonce each time, no credential in `Debug` output
- [ ] 2.2 The OAuth exchange (`request_token` with the callback, `access_token` with the verifier) through the transport seam, so tests use a fake Discogs. Tests for success, refusal and a malformed answer

## 3. The callback

- [ ] 3.1 A one-time listener on 127.0.0.1 (free port) that accepts exactly one `/discogs` request whose `oauth_token` matches, answers it with a short page, answers anything else 404, and stops after the answer, Cancel or 5 minutes. Tests with real local requests (wrong path, wrong token, the right one, the timeout made short)

## 4. Storage and UI

- [ ] 4.1 `dig/oauth` stored like `dig/token` (atomic, 0600, fail soft); saving one removes the other; Disconnect removes both. Tests with `TestDir`
- [ ] 4.2 Options ▸ Discogs…: Connect with Discogs first, "Use a personal token instead" below; waiting state with Cancel; the errors from the spec; the Connect to Discogs dialog and the setup panel's Discogs step (`welcome-setup`) use the same flow. Headless tests with the fake Discogs

## 5. Docs and checks

- [ ] 5.1 README: connecting with Discogs, the personal token as the alternative, where the credentials are kept; the test count
- [ ] 5.2 CHANGELOG `## Unreleased` with the `changelog` skill
- [ ] 5.3 The four checks from AGENTS.md and `cargo xwin check`
- [ ] 5.4 A real connection on the Mac and on Windows (the listener raises no firewall prompt); record in `findings.md`
