## Context

- Today: Options ▸ Discogs… takes a personal access token, checked with `/oauth/identity`,
  stored in `<config>/dig/token` (mode 0600 on Unix), sent as `Authorization: Discogs
  token=…`. `Request.token` and `Request::headers()` (`crates/dig/src/discogs/transport.rs`)
  are the only place a credential becomes a header.
- Discogs documents OAuth 1.0a for applications: a request token
  (`https://api.discogs.com/oauth/request_token`), the user's authorization
  (`https://www.discogs.com/oauth/authorize?oauth_token=…`), and the access token
  (`https://api.discogs.com/oauth/access_token`). From memory, it accepts the PLAINTEXT
  signature over HTTPS and a callback URL given with the request token; task 1 checks both
  against Discogs' current documentation before any code.
- The bridge already listens on 127.0.0.1 with `tiny_http`, so the app knows how to receive a
  local HTTP request safely.
- `welcome-setup` adds a setup panel whose Discogs step offers Connect….

## Goals / Non-Goals

**Goals:**
- Connecting an account is: click, Authorize in the browser, done.
- Nothing about requests, rate limits or caching changes for users who keep a personal token.

**Non-Goals:**
- OAuth 2 or anything Discogs doesn't offer.
- Several accounts at once.
- Hiding the consumer secret: in an open-source desktop app it can't be secret (see Risks).

## Decisions

### 1. Credentials as one enum
`Request.token: Option<String>` becomes `Request.auth: Option<Auth>`, with
`Auth::Token(String)` and `Auth::OAuth { token, secret }` (the consumer key and secret are
constants). `headers()` builds `Discogs token=…` or the OAuth header. Every caller passes what
`Client` holds, so the change stays in the transport and the client.

### 2. PLAINTEXT signature over HTTPS
The OAuth header carries `oauth_consumer_key`, `oauth_token`, `oauth_nonce` (16 random bytes,
hex, from `getrandom`), `oauth_timestamp`, `oauth_signature_method="PLAINTEXT"`,
`oauth_version="1.0"` and `oauth_signature="‹consumer secret›&‹token secret›"`, all
percent-encoded. Every Discogs request is already HTTPS. Alternative: HMAC-SHA1, which needs
the base string of every request and a crypto dependency; only if Discogs refuses PLAINTEXT.

### 3. The callback: a one-time local listener
Connect with Discogs starts a listener on `127.0.0.1` on a free port, asks for a request token
with `oauth_callback=http://127.0.0.1:‹port›/discogs`, opens the authorization page through the
browser seam, and waits up to 5 minutes for one request to `/discogs` whose `oauth_token`
matches the request token; it answers with a short "Diggr is connected, you can close this tab"
page and stops listening. Any other request is answered 404. Cancel, a timeout or a mismatch
stops it. Alternative: the out-of-band flow (the user copies a verifier code back), as a
fallback if Discogs refuses local callbacks.

### 4. Storage
The access token and secret go to `<config>/dig/oauth` with the same rules as `dig/token`
(written atomically, mode 0600, never logged, shown only as the username). A personal token and
an OAuth connection are exclusive: saving one removes the other. Disconnect removes both.

### 5. UI
Options ▸ Discogs… leads with Connect with Discogs; the token field moves under "Use a personal
token instead". While waiting: "Waiting for Discogs in your browser…" with Cancel. Errors say
what happened (Discogs refused, the browser didn't come back in 5 minutes, Discogs can't be
reached). The Connect to Discogs dialog and the setup panel's Discogs step call the same flow.

## Risks / Trade-offs

- **The consumer secret is public** (it's in the source and the binary) → someone could make
  requests as "Diggr", but only with their own user's authorization; Discogs' terms for desktop
  apps are read in task 1, and the key can be reset from the developer settings if abused.
- **Discogs refuses local callbacks or PLAINTEXT** → the out-of-band verifier and HMAC-SHA1 are
  the fallbacks; task 1 decides before any code.
- **A firewall prompt for the listener** → it binds 127.0.0.1 only, like the bridge (which
  raises no prompt on Windows, per `windows-port`).
- **Users with a personal token** → unchanged; the token keeps working.

## Migration Plan

Existing tokens keep working. Connecting with Discogs replaces a stored token. Rollback: the
previous release reads `dig/token` only; an OAuth-only user would have to paste a token again.

## Open Questions

- Does Discogs accept PLAINTEXT and a 127.0.0.1 callback today? (Task 1.)
- Is the rate limit for OAuth the same 60 a minute as for a personal token? (Task 1.)
