## ADDED Requirements

### Requirement: No cart pill on what you own
A record SHALL count as owned when the user's Discogs collection holds its release or another physical release of its master (see `discogs-collection`), whatever crate shows it. Every entry of the collection crate, and every entry sent from a Discogs collection page, SHALL count as owned. The rule SHALL hold in the collection crate even though no OWNED badge is drawn there.

A row whose record is owned and not in a cart SHALL show no + CART pill, no pill that opens a record of several copies, and no Buy pill (see `buy-chooser`). In a seller crate, the copy rows of an owned record SHALL show no pill. Its other badges (OWNED, SOLD) SHALL be drawn as before.

A row whose record is owned and in a cart SHALL keep its IN CART pill, which SHALL read REMOVE while hovered and take the item out when clicked. Its tooltip SHALL begin with "You already own ‹pressing›. " ("this pressing", "another pressing (AF001R, 2019)"), or "You already own this. " in the collection crate, before its usual text. The app SHALL NEVER take an item out of a cart because it is owned.

Without a token or a synced collection, only the collection crate and entries sent from a collection page SHALL count as owned.

#### Scenario: Collection crate
- **WHEN** the collection crate is shown and none of its records is in a cart
- **THEN** no row shows + CART or any other cart pill

#### Scenario: Owned in a label crate
- **WHEN** a label crate shows release 123456, and the collection holds release 123456
- **THEN** its rows show OWNED and no + CART pill

#### Scenario: Another pressing
- **WHEN** a label crate shows release 777 of master 42, and the collection holds release 778 of master 42 (AF001R, 2019)
- **THEN** its rows show OWNED and no + CART pill

#### Scenario: Seller crate copies of an owned record
- **WHEN** a seller crate holds an owned record with unsold copies at €9.00 and €12.00, none in the cart
- **THEN** neither the record row nor its copy rows show a cart pill

#### Scenario: Owned and in the cart
- **WHEN** a copy of release 777 from decks.de at €9.00 is in the cart, and the collection holds release 778 of the same master (AF001R, 2019)
- **THEN** its row shows IN CART, the tooltip begins "You already own another pressing (AF001R, 2019). ", and clicking REMOVE takes the copy out

#### Scenario: No token
- **WHEN** no Discogs token is set and a label crate is shown
- **THEN** its rows show + CART as before

### Requirement: Checkout pill
Every row that shows IN CART or CART SHALL show a `PAY »` pill right after it, with the badges' font, height and corner radius, in the CART colour as an outline (filled while hovered), and a fixed width that fits its label, "PAY »". Every row with a cart pill SHALL keep that width after it, blank when not in a cart, so going in or out of the cart never moves the row's text. A click SHALL open the cart of the row's store in the default browser, within one frame: `https://www.discogs.com/sell/cart` for a Discogs copy, or the store's basket page for a store item (see `store-carts`). For a CART badge with a Discogs copy in the cart, it SHALL open the Discogs cart. Its tooltip SHALL name the site ("Check out on discogs.com"). A click on it SHALL NOT select, play, drag or open its row. Rows not in a cart SHALL show no `PAY »`. The app SHALL NEVER check out or pay.

#### Scenario: After adding
- **WHEN** the user clicks + CART on a copy in a seller crate and the pill reads IN CART
- **THEN** a `PAY »` pill shows after it, and clicking it opens `https://www.discogs.com/sell/cart` without selecting the row

#### Scenario: Wantlist CART badge
- **WHEN** release 555 is on the wantlist and a copy of it from Housevinyl.nl is in the cart
- **THEN** its wantlist entries show CART then `PAY »`, with the tooltip "Check out on discogs.com"

#### Scenario: Not in a cart
- **WHEN** a row shows + CART
- **THEN** it shows no `PAY »` pill

## MODIFIED Requirements

### Requirement: Add to cart
The cart SHALL be reached through a cart pill on the row, drawn where the CART badge would be, with the badges' font, height and corner radius, and a fixed width that fits its widest label:
- **+ CART** on an unsold copy that is not in the cart: clicking it adds that copy to the user's Discogs cart;
- **IN CART** on a copy in the cart, which SHALL read **REMOVE** in the SOLD colour while hovered: clicking it takes that copy out of the Discogs cart.

The pill SHALL be shown on every unsold copy row, and on the record row of a record with exactly one unsold copy, where it acts on that copy. A record row with several unsold copies SHALL show a pill that opens the record, reading IN CART when one of its copies is in the cart. Every other entry and record row with a record behind it SHALL show a pill that acts as `buy-chooser` says, once per record: consecutive rows of the same record SHALL share one pill (or CART) and `PAY »`, centred vertically on the part of the run in view, and every row of the run SHALL keep the pill's slot blank so their titles line up; a run of two or more rows SHALL have a thin line above and below it. The tracks of an open record SHALL leave the pill to the record row. A store item bought per track (Beatport, Traxsource) has no record of its own and keeps its pill on its row. Sold copies SHALL show SOLD and no pill. A click on the pill SHALL NOT select, play or drag its row, and its tooltip SHALL name the action and the copy ("Add €9.00 · VG+ / VG to your cart") or the store ("Open on Discogs marketplace").

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

#### Scenario: Track rows
- **WHEN** a wantlist crate is shown flat, with release 1001's three tracks one after the other
- **THEN** one + CART pill shows, centred on the middle track, the three titles start at the same x, a line runs above the first and below the last, and clicking the pill selects no row

#### Scenario: Double-click on the pill
- **WHEN** the user double-clicks the + CART pill of a copy row
- **THEN** the copy is added and removed again, and its listing does not open

#### Scenario: No cart items in menus
- **WHEN** the user right-clicks a copy row, a track row or a record row in a seller crate
- **THEN** the menu offers neither Add to cart nor Remove from cart, and the copy row's menu still offers Open on discogs.com
