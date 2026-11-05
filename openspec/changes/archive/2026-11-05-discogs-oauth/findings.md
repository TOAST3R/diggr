# Findings

## Task 1.3: the OAuth flow by hand (2026-11-04)

With `curl` on macOS, the consumer key and secret read from the Keychain, User-Agent
`Diggr/0.1.5 +https://diggr.es`.

1. **Request token**: `GET https://api.discogs.com/oauth/request_token`, PLAINTEXT signature
   `‹consumer secret›&`, `oauth_callback=https://diggr.es/discogs/callback?port=47811`
   (percent-encoded). Answer: `oauth_token` (40 letters), `oauth_token_secret`,
   `oauth_callback_confirmed=true`.
2. **Authorize**: `https://www.discogs.com/oauth/authorize?oauth_token=…`, then Authorize. The
   browser landed on
   `https://diggr.es/discogs/callback?port=47811&oauth_token=…&oauth_verifier=…`:
   the per-request callback is honoured, its own query (`port`) is kept, and Discogs appends
   `oauth_token` and `oauth_verifier` after it. The verifier is 10 letters.
3. **Access token**: `POST https://api.discogs.com/oauth/access_token`, PLAINTEXT signature
   `‹consumer secret›&‹request token secret›`, with `oauth_verifier`. Answer: `oauth_token` and
   `oauth_token_secret`.
4. **Signed request**: `GET https://api.discogs.com/oauth/identity`, PLAINTEXT signature
   `‹consumer secret›&‹access token secret›`. `HTTP/2 200`, the username, and
   `"consumer_name":"Diggr"`. Rate limit headers: `x-discogs-ratelimit: 60`, the same as with a
   personal token.

Answers to the design's open questions:
- PLAINTEXT over HTTPS: **accepted**. No HMAC-SHA1, no crypto dependency.
- Per-request `oauth_callback` with `?port=`: **kept**. The app can pass its bridge port.
- OAuth rate limit: **60 a minute**.
- What Discogs sends on Deny: **not checked**, by choice. The design works either way: the
  callback page shows "You didn't allow Diggr" if it gets `denied`, and the app's wait ends
  with Cancel or the 5 minutes. Forwarding a Deny to the app stays an unconfirmed follow-up.

## Task 1.1: the API Terms of Use (2026-11-04)

Read from https://support.discogs.com/hc/en-us/articles/360009334593-API-Terms-of-Use
("Last Updated: May 27th, 2025"; the help center blocks scripted fetches, the text came from
its public Zendesk API, `/api/v2/help_center/en-us/articles/360009334593.json`).

For this change:
- **Consumer secret in a desktop app**: no clause requires the consumer key or secret to be kept
  confidential; "USER ACCOUNT, PASSWORD, AND SECURITY" only asks to comply with Discogs'
  security requirements. Building them into the release binary (design decision 6) is not
  against the terms.
- **One key for every user**: allowed. Forbidden is "creating additional API keys to overcome
  these limits", so the app must ship one key, never several to spread the rate limit.
- **The name**: "Diggr" doesn't use the Discogs trademark. The terms point to Discogs'
  "Application Name and Description Policy", not read here.

Outside this change (gaps in Diggr today, for their own change):
- The notice "This application uses Discogs' API but is not affiliated with, sponsored or
  endorsed by Discogs. 'Discogs' is a trademark of Zink Media, LLC." is required on the app and
  every public-facing use (terms or usage docs are enough). Not in the app, the README or
  diggr.es.
- "Data provided by Discogs", linked to the discogs.com page with that data (no `nofollow`), is
  required directly next to any Discogs data. Not in the app.
- Content may not be displayed more than 6 hours older than discogs.com, nor cached longer than
  needed. `crates/dig/src/discogs/cache.rs` keeps listing pages and marketplace numbers 24 h
  (`FRESH_SECS`) and release data indefinitely.
- Restricted Data (user data: collection, wantlist; marketplace data) may not be used
  commercially, and charging for an app that integrates the API needs written permission.

## Tasks 3.3 and 6.4: a real connection (2026-11-05)

The maintainer reported that a real connection works end to end: the diggr.es callback page
is live, and Connect with Discogs, then Authorize, connects the account.
