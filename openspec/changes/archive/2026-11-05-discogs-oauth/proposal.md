## Why

Connecting a Discogs account means opening Discogs' developer settings, generating a personal
access token, copying it and pasting it into Options ▸ Discogs…. It works, but it's a
developer's procedure, and it's the step where people give up. Discogs supports OAuth: the app
can open Discogs in the browser, the user clicks Authorize, and the app is connected.

## What Changes

- **Connect with Discogs**: a button in Options ▸ Discogs…, in the Connect to Discogs dialog and
  in the setup panel's Discogs step (`welcome-setup`). It asks Discogs for a request token,
  opens the authorization page in the default browser, and receives the answer through the
  callback page `https://diggr.es/discogs/callback`, which forwards it to the bridge on
  127.0.0.1 (or to a one-time local listener when the bridge is off). It exchanges it for an access token and secret.
  The username shows as with a personal token.
- **Every Discogs request is signed** with OAuth 1.0a when the account was connected this way;
  a personal token still works as today and stays available as "Use a personal token instead".
- **Stored like the token**: the access token and secret in the config folder, readable only by
  the user, never shown or logged; Disconnect deletes them.
- **The app is registered with Discogs** as "Diggr" (a maintainer step): its consumer key and
  secret are built into release builds from the maintainer's Keychain, never committed.

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
- `crates/dig/src/bridge/`: a `/discogs` route, without a pairing key, only while a connection waits.
- `scripts/`: the release scripts read the consumer key and secret from the Keychain.
- The diggr.es repository: the static callback page (its own change).
- `crates/ui/src/app/digging.rs`: the Connect with Discogs button, waiting state, errors.
- No new dependency expected (`getrandom` is there for the nonce).
- A Discogs developer application registered by the maintainer.
