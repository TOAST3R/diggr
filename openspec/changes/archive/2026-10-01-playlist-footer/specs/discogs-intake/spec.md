## MODIFIED Requirements

### Requirement: Discogs account
The user SHALL be able to enter a Discogs personal access token in Options ▸ Discogs…. The app SHALL check the token with Discogs and show the account's username, or say that the token was rejected. The token SHALL be stored only in the user's config folder, in a file only the user can read, and SHALL never be shown in full or written to a log. Without a token, pages SHALL still be expanded, at the unauthenticated rate limit, and actions that need an account (adding to the wantlist, reading a private wantlist) SHALL say that a token is needed.

#### Scenario: Valid token
- **WHEN** the user enters a valid token
- **THEN** Options ▸ Discogs… shows "Connected as ‹username›", and later requests use the authenticated rate limit

#### Scenario: Rejected token
- **WHEN** Discogs rejects the token
- **THEN** the dialog says the token was rejected, and the token is not saved

### Requirement: Filters
Every send SHALL apply two filters, both on by default. Their defaults can be changed in Options ▸ Discogs…, and each send can override them:
- vinyl only: records with no vinyl format are left out;
- skip passed: clips the user has passed on are left out.

Pasting SHALL use the defaults.

#### Scenario: Vinyl only
- **WHEN** a label with 10 vinyl releases and 5 CD-only releases is sent with vinyl only on
- **THEN** only the 10 vinyl releases add entries

#### Scenario: Skip passed
- **WHEN** a label is sent again after the user passed on 20 of its clips
- **THEN** those 20 clips are not added
