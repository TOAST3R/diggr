# discogs-cart Specification

## Purpose
Puts copies from seller crates in the user's real Discogs cart and takes them out, marks everything in the cart with CART in every crate, and lets a seller crate show and play only what is in the cart before the user pays on discogs.com.
## Requirements
### Requirement: Add to cart
In a seller crate, the cart SHALL be reached through a cart pill on the row, drawn where the CART badge would be, with the badges' font, height and corner radius, and a fixed width that fits its widest label:
- **+ CART** on an unsold copy that is not in the cart: clicking it adds that copy to the user's Discogs cart;
- **IN CART** on a copy in the cart, which SHALL read **REMOVE** in the SOLD colour while hovered: clicking it takes that copy out of the Discogs cart.

The pill SHALL be shown on every unsold copy row, and on the record row of a record with exactly one unsold copy, where it acts on that copy. A record row with several unsold copies, and every track row, SHALL show no pill. Sold copies SHALL show SOLD and no pill. A click on the pill SHALL NOT select, play, drag or open its row, and its tooltip SHALL name the action and the copy ("Add €9.00 · VG+ / VG to your cart").

The right-click menus of copy, track and record rows SHALL NOT offer Add to cart or Remove from cart.

Every add SHALL be one request. The pill SHALL change within one frame of the click, without waiting for Discogs. The main window SHALL then say what happened ("Added to your cart"). Without a token, clicking the pill SHALL open the Connect to Discogs dialog (see `discogs-write`).

#### Scenario: One copy
- **WHEN** the user opens "Glasshouse EP" in a seller crate and clicks + CART on the €9.00 VG+ copy of release 123456
- **THEN** the copy is in the user's Discogs cart, its pill reads IN CART within the frame, and its row is not selected

#### Scenario: Remove
- **WHEN** the user hovers the IN CART pill of a copy in the cart
- **THEN** it reads REMOVE in red, and clicking it takes the copy out of the Discogs cart and the pill reads + CART

#### Scenario: Record with one copy
- **WHEN** a seller crate holds a record with one unsold copy at €9.00 and one sold copy
- **THEN** its record row shows + CART, and clicking it adds the €9.00 copy without opening the record

#### Scenario: Record with several copies
- **WHEN** a seller crate holds a record with unsold copies at €9.00 and €12.00, the €9.00 one in the cart
- **THEN** its record row shows the CART badge and no pill, and each of its copy rows shows its own pill

#### Scenario: Double-click on the pill
- **WHEN** the user double-clicks the + CART pill of a copy row
- **THEN** the copy is added and removed again, and its listing does not open

#### Scenario: No cart items in menus
- **WHEN** the user right-clicks a copy row, a track row or a record row in a seller crate
- **THEN** the menu offers neither Add to cart nor Remove from cart, and the copy row's menu still offers Open on discogs.com

### Requirement: Cart outcomes
Each copy in an Add to cart SHALL end as one of:
- added: it shows CART;
- already in the cart: it shows CART, counted as added;
- no longer for sale: it shows SOLD, and the main window says so;
- anything else (an unexpected answer, Discogs unreachable): its listing SHALL open on discogs.com in the default browser, and the main window SHALL say the cart couldn't be reached and the listing was opened instead.

The app SHALL NEVER empty the user's cart or a seller's cart.

#### Scenario: Sold meanwhile
- **WHEN** the user adds a copy that was sold since the last refresh
- **THEN** it shows SOLD, no CART badge, and the main window says it is no longer for sale

#### Scenario: Cart unavailable
- **WHEN** Discogs answers the cart request with an unexpected error
- **THEN** the copy's listing opens on discogs.com, and the main window says it was opened there instead

### Requirement: CART badge
The app SHALL know the user's Discogs cart from a cached snapshot, read with a token at launch (behind other work), after every dig or refresh of a seller crate, and after every cart action. The snapshot SHALL be saved in the cache folder and shown at once at the next launch.

A copy whose listing is in the cart SHALL show it, in a colour distinct from OWNED: as the IN CART pill where a pill is shown (see "Add to cart"), and as a CART badge everywhere else. The record row of a record with a copy in the cart SHALL show it too. Any entry in any crate, including the wantlist and collection crates, whose release has a copy in the cart SHALL show CART. Its tooltip SHALL name the seller and the price ("In your cart from decks.de, €9.00"). Copies added or removed on discogs.com SHALL be reflected at the next read.

#### Scenario: Added on discogs.com
- **WHEN** the user added a copy of release 777 from decks.de on discogs.com, and then refreshes "Seller: decks.de"
- **THEN** that copy shows IN CART in the crate

#### Scenario: Wantlist entry
- **WHEN** release 555 is on the wantlist and a copy of it from Housevinyl.nl is in the cart
- **THEN** its entries in the wantlist crate show the CART badge, not a pill, and the tooltip says "In your cart from Housevinyl.nl, €14.00"

#### Scenario: Flat seller crate
- **WHEN** a seller crate is shown flat and a copy of one of its records is in the cart
- **THEN** that record's track rows show the CART badge and no pill

#### Scenario: At launch
- **WHEN** the app starts with a cached cart snapshot holding 3 copies
- **THEN** those copies show IN CART or CART in the first frame, before any request

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

