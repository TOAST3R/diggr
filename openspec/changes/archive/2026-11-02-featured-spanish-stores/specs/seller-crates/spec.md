## ADDED Requirements

### Requirement: Featured record stores
Once per install, on the first launch where the Top Sellers file doesn't say it was done, the app SHALL make sure the list holds these six sellers, each with a crate named as shown and recognised as that seller's crate. Those it lacks SHALL be added with an empty crate that hasn't been dug: all six, in this order, on a fresh install; after the user's own sellers, in this order, on an existing list.

| Crate | Discogs seller |
|---|---|
| Seller: Melting Pot · Madrid | logon |
| Seller: Rotor · Madrid | rotordiscos |
| Seller: Rhythm Control · Barcelona | djbrucelee |
| Seller: KH Vinyl Store · Valencia | KHVinylStore |
| Seller: Mita · Gijón | MITA_Record_Store |
| Seller: Adepta · Online | ADEPTA_STORE |

A seller already in the list under one of these usernames (ignoring case) SHALL keep its place, crate and dug copies, and its crate SHALL be renamed as shown unless the user already renamed it. Nothing else in an existing list SHALL change. The app SHALL save the list at once with a mark that this was done, so it happens only once. It SHALL NOT make any network request and SHALL NOT delay launch by more than 5 ms. The featured sellers SHALL behave as any other seller: double-click digs, they can be refreshed, renamed and removed, and a removed one SHALL NOT come back. The first list from purchases SHALL still run once when a token is saved, adding its sellers after the featured ones and skipping any seller already in the list.

#### Scenario: Fresh install
- **WHEN** the app is launched for the first time, with no token
- **THEN** TOP SELLERS lists Melting Pot · Madrid, Rotor · Madrid, Rhythm Control · Barcelona, KH Vinyl Store · Valencia, Mita · Gijón and Adepta · Online, none of them dug

#### Scenario: Double-click without a token
- **WHEN** the user double-clicks Melting Pot · Madrid with no token
- **THEN** the Connect to Discogs dialog opens

#### Scenario: Digs the shop's stock
- **WHEN** the user, with a token, double-clicks Rotor · Madrid
- **THEN** the app counts rotordiscos' copies for sale and offers Dig or Narrow down, as for any seller

#### Scenario: Existing user updates
- **WHEN** the config folder already holds a Top Sellers file listing decks.de and logon, and the app is updated and launched
- **THEN** TOP SELLERS lists decks.de, then logon as Melting Pot · Madrid (its crate renamed, its place kept), then Rotor · Madrid, Rhythm Control · Barcelona, KH Vinyl Store · Valencia, Mita · Gijón and Adepta · Online

#### Scenario: A name the user chose is kept
- **WHEN** the user had renamed logon's crate to "Jazz shop" before updating
- **THEN** it is still called "Jazz shop" and is still logon's crate

#### Scenario: Only once
- **WHEN** the app has been launched once since the update, the user removes Mita · Gijón, and the app is restarted
- **THEN** Mita · Gijón is not added back

#### Scenario: Purchases after the featured shops
- **WHEN** a token is saved, and the user's purchases rank logon and decks.de first
- **THEN** the list keeps the six featured sellers first, then decks.de, and logon appears once, as Melting Pot · Madrid
