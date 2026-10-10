## MODIFIED Requirements

### Requirement: Cart filter
In a seller crate with at least one copy in the cart, the filter bar (see `filter-bar`) SHALL show a CART switch labelled with the number of copies in the cart from that seller and their subtotal in the seller's currency ("CART 3 · €41.20"), or, when it doesn't fit, the filter panel SHALL hold it. While it is on, only records with a copy in the cart SHALL be shown and played, combined with the search and the other filters (see `record-filters`). The ☰ menu SHALL offer Open cart on discogs.com, which opens `https://www.discogs.com/sell/cart` in the default browser. The switch SHALL be remembered per crate across restarts.

#### Scenario: Check the cart
- **WHEN** "Seller: decks.de" holds 400 records, 3 of them with a copy in the cart, and the user turns CART on
- **THEN** only those 3 records are shown, only their tracks play, and the title bar shows 3 of 400

#### Scenario: Empty cart
- **WHEN** no copy of the shown seller crate is in the cart
- **THEN** no CART switch is shown

#### Scenario: Narrow playlist
- **WHEN** a seller crate with 3 copies in the cart is shown 275 pixels wide
- **THEN** the filter bar shows FILTERS, and the filter panel holds the CART switch
