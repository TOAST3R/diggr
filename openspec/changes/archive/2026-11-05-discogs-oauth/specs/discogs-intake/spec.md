## MODIFIED Requirements

### Requirement: Discogs account
The user SHALL be able to connect a Discogs account in Options ▸ Discogs… in two ways:
- **Connect with Discogs** (offered first): the app SHALL open Discogs' authorization page in the default browser and receive the user's answer, for at most 5 minutes, through the callback page `https://diggr.es/discogs/callback`, which forwards it to the bridge on this machine; the bridge SHALL accept only the one answer it waits for. Before opening the authorization page, the app SHALL be listening for the answer: on the bridge, or, when the bridge isn't running, on a one-time local address on a free port; it SHALL tell Discogs the port it listens on. The callback page SHALL never hold the app's consumer secret. On Authorize it SHALL obtain an access token and secret, and sign every later Discogs request with them (OAuth 1.0a). Cancel, a refusal or the 5 minutes passing SHALL leave the account unconnected and say why.
- **A personal access token** ("Use a personal token instead"), as before.

The app SHALL check either with Discogs and show the account's username, or say that it was rejected. Credentials SHALL be stored only in the user's config folder, in files only the user can read, and SHALL never be shown in full or written to a log; connecting one way SHALL remove the other's credentials, and Disconnect SHALL remove both. Without an account, pages SHALL still be expanded, at the unauthenticated rate limit, and actions that need an account (adding to the wantlist, reading a private wantlist) SHALL say that an account is needed.

#### Scenario: Connect with Discogs
- **WHEN** the user clicks Connect with Discogs and clicks Authorize in the browser within 5 minutes
- **THEN** Options ▸ Discogs… shows "Connected as ‹username›", the browser tab says it can be closed, and later requests are signed and use the authenticated rate limit

#### Scenario: Not authorized
- **WHEN** the user closes the browser tab without authorizing, and 5 minutes pass
- **THEN** the dialog says Discogs didn't answer, the local address no longer answers, and nothing is saved

#### Scenario: Valid token
- **WHEN** the user enters a valid personal token
- **THEN** Options ▸ Discogs… shows "Connected as ‹username›", and later requests use the authenticated rate limit

#### Scenario: Rejected token
- **WHEN** Discogs rejects the token
- **THEN** the dialog says the token was rejected, and the token is not saved

#### Scenario: Denied
- **WHEN** the user clicks Deny on Discogs' authorization page
- **THEN** the callback page says Diggr wasn't allowed, nothing is saved, and the app stops waiting on Cancel or after 5 minutes

#### Scenario: The bridge is off
- **WHEN** the bridge isn't running (turned off, or its port taken) and the user clicks Connect with Discogs and authorizes
- **THEN** the answer reaches a one-time local address on a free port, the account is connected, and that address no longer answers afterwards

#### Scenario: Diggr closed during Authorize
- **WHEN** the user closes Diggr before clicking Authorize, then clicks it
- **THEN** the browser can't reach 127.0.0.1, nothing is saved, and the next Connect with Discogs starts over

#### Scenario: Another request on the local address
- **WHEN** a request that doesn't carry the expected request token reaches the local address while the app waits
- **THEN** it is answered "not found", and the app keeps waiting for the real answer
