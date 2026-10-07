## MODIFIED Requirements

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
