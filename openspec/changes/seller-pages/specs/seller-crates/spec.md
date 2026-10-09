## MODIFIED Requirements

### Requirement: Seller pages
A seller's page (`/seller/‹name›` and any page under it, such as `/seller/‹name›/profile` or `/seller/‹name›/mywants`; `/sell/list` with a `seller=‹name›` parameter; or `/user/‹name›` with no further path), with or without filters in its address, sent by pasting it in the player window or from the browser extension SHALL add that seller, as Add seller does, or refresh it if it is already in the list. Either way, its crate SHALL be shown. It SHALL NOT add tracks to any other crate.

#### Scenario: Paste a seller page
- **WHEN** the user presses Cmd+V with `https://www.discogs.com/seller/logon/profile` copied, and logon isn't in the list
- **THEN** logon is added at the top of TOP SELLERS, its crate is shown, and the dig flow opens

#### Scenario: Already there
- **WHEN** the same page is pasted and logon is in the list, last dug 3 days ago
- **THEN** "Seller: logon" is shown and refreshed

#### Scenario: Seller's catalogue
- **WHEN** the user pastes `https://www.discogs.com/sell/list?seller=decks.de&style=Techno`, and decks.de isn't in the list
- **THEN** decks.de is added at the top of TOP SELLERS, its crate is shown, and no tracks are added anywhere

#### Scenario: Another page of the shop
- **WHEN** the user pastes `https://www.discogs.com/seller/decks.de/feedback`
- **THEN** it is taken as the seller decks.de, like its profile page
