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
- "Diggr" is registered with Discogs (task 1.2) with the callback URL
  `https://diggr.es/discogs/callback`. The consumer key and secret are in the maintainer's macOS
  Keychain as `diggr-discogs-key` and `diggr-discogs-secret`.
- The landing site diggr.es is hosted on Vercel, in its own repository.

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

### 3. The callback: diggr.es forwards to the bridge
The callback registered with Discogs is `https://diggr.es/discogs/callback`, a static page on
the landing site (a change in the diggr.es repository). It holds no secret and calls no
function: the verifier is useless without the request-token secret, which only the app has.

```
 Discogs ──redirect──▶ https://diggr.es/discogs/callback?oauth_token=…&oauth_verifier=…[&port=…]
                              │ static page: forwards the query string by navigation
                              ▼
                       http://127.0.0.1:‹port›/discogs?oauth_token=…&oauth_verifier=…
                              │ the bridge's route
                              ▼
                       Diggr exchanges the verifier; the tab says "Diggr is connected"
```

- **The app** asks for the request token with
  `oauth_callback=https://diggr.es/discogs/callback?port=‹bridge port›`. If Discogs ignores a
  per-request callback and uses the registered one (task 1.3), the page uses the default port
  47800.
- **The page** forwards with a top-level navigation (`location.replace`), never with `fetch`:
  browsers ask before a website contacts the local network ("local network access"), but not
  for a navigation. It accepts only `oauth_token`, `oauth_verifier` and `port` (digits, 1024 to
  65535), and forwards only to `127.0.0.1`. If the query carries `denied`, it says the user
  didn't allow Diggr and doesn't forward; the app's wait ends with Cancel or the 5 minutes
  (forwarding the Deny is a follow-up once task 1.3 shows what Discogs sends).
- **A listener is guaranteed before Discogs opens.** The app passes the port it actually
  listens on. If the bridge isn't running (off, or its port taken), Connect with Discogs starts
  a one-time listener on a free port (`127.0.0.1:0`) serving only `/discogs`, with the same
  rules, and passes that port; it stops after the answer, Cancel or the 5 minutes. So the
  forward can only fail if Diggr is closed or crashes during Authorize, and then nothing could
  finish the exchange: the request-token secret was in memory.
- **No code to paste.** An earlier version had the page show the verifier on Back, to paste in
  the app. Chrome and Edge leave a page that navigates before any user interaction out of the
  Back history (`diggr.es` change `discogs-callback`, task 4.3), so Back never reaches it there;
  and with the guarantee above, the code would only exist when it can't be used. The waiting
  state says instead: "If your browser says it can't reach 127.0.0.1, make sure Diggr is open
  and click Connect with Discogs again."
- The page's details are in the diggr.es repository, change `discogs-callback`.
- **The bridge route** `/discogs` takes no pairing key. It answers only while a connection is
  waiting and only when `oauth_token` matches the request token, once. Anything else gets a 404.
  The bridge's host check (against DNS rebinding) still applies. The answer is a short page
  saying Diggr is connected and the tab can be closed.
- Cancel, the 5 minutes passing, or a Deny ends the wait.

Alternatives: a callback straight to `http://127.0.0.1:‹port›/discogs` (simpler, but Discogs'
form asks for one registered URL, and the landing page can explain a Deny); the one-time
listener for every connection (it's kept only for when the bridge isn't running, so the usual
path has no new socket); a "Finish connecting" button on the page instead of the automatic
forward (Chrome would then keep the page in Back history, but every user pays a click for a
case where the code couldn't be used).

### 4. Storage
The access token and secret go to `<config>/dig/oauth` with the same rules as `dig/token`
(written atomically, mode 0600, never logged, shown only as the username). A personal token and
an OAuth connection are exclusive: saving one removes the other. Disconnect removes both.

### 5. UI
Options ▸ Discogs… leads with Connect with Discogs; the token field moves under "Use a personal
token instead". While waiting: "Waiting for Discogs in your browser…" with Cancel. Errors say
what happened (Discogs refused, the browser didn't come back in 5 minutes, Discogs can't be
reached). The Connect to Discogs dialog and the setup panel's Discogs step call the same flow.

### 6. The consumer key and secret: built in at release time
`crates/dig/src/discogs/` reads them with `option_env!("DISCOGS_CONSUMER_KEY")` and
`option_env!("DISCOGS_CONSUMER_SECRET")`. The release scripts read them from the Keychain
(`security find-generic-password -s diggr-discogs-key -w`, likewise `diggr-discogs-secret`)
through one shared helper: `make-dmg.sh` and `make-windows.sh` export them, and `make-linux.sh`
passes them into its container with `docker run -e`. A build without them (from source,
`cargo test`, clippy, the wasm check) has no Connect with Discogs button, and "Use a personal
token" is the only way. Tests use fake keys. Nothing secret is in the repository.

### 7. diggr.es never holds the secret
Every Discogs request is signed with the consumer secret, so a server holding it would have to
relay every request: a round trip on the path, a shared rate limit, a single point of failure,
and anyone could still have requests signed through it. The site only hosts the callback page.

## Risks / Trade-offs

- **The consumer secret is public** (it's in the source and the binary) → someone could make
  requests as "Diggr", but only with their own user's authorization; Discogs' terms for desktop
  apps are read in task 1, and the key can be reset from the developer settings if abused.
- **Discogs refuses PLAINTEXT** → HMAC-SHA1 is the fallback; task 1 decides before any code.
- **diggr.es is down** → connecting fails (requests of connected users don't depend on it); the
  waiting state says Discogs didn't answer, and the personal token still works.
- **Diggr closes during Authorize** → the browser shows its own connection error; the user opens
  Diggr and connects again (the waiting state's hint says so).
- **The callback route weakens the bridge** → it takes no pairing key, but only answers while a
  connection waits, for the one matching `oauth_token`, once, behind the host check.
- **A release built without the keys** → no Connect with Discogs button; `scripts/release.sh`
  checks the helper found both keys before building.
- **Users with a personal token** → unchanged; the token keeps working.

## Migration Plan

Existing tokens keep working. Connecting with Discogs replaces a stored token. Rollback: the
previous release reads `dig/token` only; an OAuth-only user would have to paste a token again.

## Open Questions

Answered by task 1.3 (`findings.md`): PLAINTEXT is accepted, the per-request callback and its
`?port=` are kept, and the OAuth rate limit is 60 a minute. What Discogs sends on Deny was not
checked; the design doesn't depend on it.
