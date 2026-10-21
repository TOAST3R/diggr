## MODIFIED Requirements

### Requirement: Add to cart
The cart SHALL be reached through a cart pill on the row, drawn where the CART badge would be, with the badges' font, height and corner radius, and a fixed width that fits its widest label:
- **+ CART** on an unsold copy that is not in the cart: clicking it adds that copy to the user's Discogs cart;
- **IN CART** on a copy in the cart, which SHALL read **REMOVE** in the SOLD colour while hovered: clicking it takes that copy out of the Discogs cart.

The pill SHALL be shown on every unsold copy row, and on the record row of a record with exactly one unsold copy, where it acts on that copy. A record row with several unsold copies SHALL show a pill that opens the record, reading IN CART when one of its copies is in the cart. Every other entry and record row with a record behind it SHALL show a pill that acts as `buy-chooser` says. Sold copies SHALL show SOLD and no pill. A click on the pill SHALL NOT select, play or drag its row, and its tooltip SHALL name the action and the copy ("Add €9.00 · VG+ / VG to your cart") or the store ("Open on Discogs marketplace").

The right-click menus of copy, track and record rows SHALL NOT offer Add to cart or Remove from cart.

Every add SHALL be one request. The pill SHALL change within one frame of the click, without waiting for Discogs. The main window SHALL then say what happened ("Added to your cart"). Without a token, clicking a pill that adds a copy SHALL open the Connect to Discogs dialog (see `discogs-write`).

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
- **WHEN** a seller crate holds a closed record with unsold copies at €9.00 and €12.00, the €9.00 one in the cart
- **THEN** its record row shows an IN CART pill, clicking it opens the record, and each of its copy rows shows its own pill

#### Scenario: Track row
- **WHEN** a wantlist crate is shown flat
- **THEN** each track row shows a + CART pill

#### Scenario: Double-click on the pill
- **WHEN** the user double-clicks the + CART pill of a copy row
- **THEN** the copy is added and removed again, and its listing does not open

#### Scenario: No cart items in menus
- **WHEN** the user right-clicks a copy row, a track row or a record row in a seller crate
- **THEN** the menu offers neither Add to cart nor Remove from cart, and the copy row's menu still offers Open on discogs.com
