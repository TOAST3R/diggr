## Why

Connecting a Discogs account means opening Discogs' developer settings, generating a personal
access token, copying it and pasting it into Options ▸ Discogs…. It works, but it's a
developer's procedure, and it's the step where people give up. Discogs supports OAuth: the app
can open Discogs in the browser, the user clicks Authorize, and the app is connected.

## What Changes

- **Connect with Discogs**: a button in Options ▸ Discogs…, in the Connect to Discogs dialog and
  in the setup panel's Discogs step (`welcome-setup`). It asks Discogs for a request token,
  opens the authorization page in the default browser, receives the answer on a one-time local
  address (`http://127.0.0.1:‹port›/discogs`), and exchanges it for an access token and secret.
  The username shows as with a personal token.
- **Every Discogs request is signed** with OAuth 1.0a when the account was connected this way;
  a personal token still works as today and stays available as "Use a personal token instead".
- **Stored like the token**: the access token and secret in the config folder, readable only by
  the user, never shown or logged; Disconnect deletes them.
- **The app is registered with Discogs** as "Diggr" (a maintainer step): its consumer key and
  secret are built into the app.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `discogs-intake`: "Discogs account" gains connecting through Discogs' authorization page,
  with OAuth-signed requests; the personal token stays.
- `discogs-write`: "Connect to Discogs" leads with Connect with Discogs instead of the
  developer-settings steps (building on `welcome-setup`'s version of the requirement, so this
  change is archived after it).

## Impact

- `crates/dig/src/discogs/transport.rs`: `Request` carries either a personal token or OAuth
  credentials; `headers()` signs (PLAINTEXT over HTTPS, if Discogs accepts it, see design).
- `crates/dig/src/discogs/`: the three-step OAuth exchange; `crates/dig/src/config.rs`: storing
  the access token and secret.
- A one-time listener on 127.0.0.1 for the callback (`tiny_http`, already used by the bridge).
- `crates/ui/src/app/digging.rs`: the Connect with Discogs button, waiting state, errors.
- No new dependency expected (`getrandom` is there for the nonce).
- A Discogs developer application registered by the maintainer.
