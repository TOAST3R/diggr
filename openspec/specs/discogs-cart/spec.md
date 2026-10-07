# discogs-cart Specification

## Purpose
Puts copies from seller crates in the user's real Discogs cart and takes them out, marks everything in the cart with CART in every crate, and lets a seller crate show and play only what is in the cart before the user pays on discogs.com.
## Requirements
### Requirement: Add to cart
With a Discogs token, right-clicking a copy row SHALL offer Add to cart, which adds that copy to the user's Discogs cart. Right-clicking a track or a record row of a record with copies SHALL offer:
- Add to cart (‹price› ‹condition›) when the record has one unsold copy;
- Add to cart ▸ with one item per unsold copy ("€9.00 · VG+ / VG") when it has several.

When the copy is in the cart, its item SHALL read Remove from cart, and SHALL take it out of the Discogs cart. Every add SHALL be one request, whatever the number of copies in it. The CART badge SHALL change within one frame of the action, without waiting for Discogs. The main window SHALL then say what happened ("Added 2 to your cart; 1 sold"). Sold copies SHALL NOT offer Add to cart. Without a token, the items SHALL open the Connect to Discogs dialog (see `discogs-write`).

#### Scenario: One copy
- **WHEN** the user right-clicks the €9.00 VG+ copy of release 123456 and chooses Add to cart
- **THEN** the copy is in the user's Discogs cart, and its row and its record row show CART

#### Scenario: Several copies from a track
- **WHEN** the user right-clicks a track of a record with copies at €9.00 and €12.00
- **THEN** the menu offers Add to cart ▸ "€9.00 · VG+ / VG" and "€12.00 · VG+ / VG+"

#### Scenario: Remove
- **WHEN** the user right-clicks a copy that is in the cart
- **THEN** the item reads Remove from cart, and choosing it takes the copy out of the Discogs cart and removes its CART badge

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

A copy whose listing is in the cart SHALL show a CART badge, in a colour distinct from OWNED. The record row of a record with a copy in the cart SHALL show it too. Any entry in any crate, including the wantlist and collection crates, whose release has a copy in the cart SHALL show CART. Its tooltip SHALL name the seller and the price ("In your cart from decks.de, €9.00"). Copies added or removed on discogs.com SHALL be reflected at the next read.

#### Scenario: Added on discogs.com
- **WHEN** the user added a copy of release 777 from decks.de on discogs.com, and then refreshes "Seller: decks.de"
- **THEN** that copy shows CART in the crate

#### Scenario: Wantlist entry
- **WHEN** release 555 is on the wantlist and a copy of it from Housevinyl.nl is in the cart
- **THEN** its entries in the wantlist crate show CART, and the tooltip says "In your cart from Housevinyl.nl, €14.00"

#### Scenario: At launch
- **WHEN** the app starts with a cached cart snapshot holding 3 copies
- **THEN** those copies show CART in the first frame, before any request

### Requirement: Cart filter
In a seller crate with at least one copy in the cart, the playlist footer SHALL show a CART switch labelled with the number of copies in the cart from that seller and their subtotal in the seller's currency ("CART 3 · €41.20"). While it is on, only records with a copy in the cart SHALL be shown and played, combined with the other filters (see `record-filters`). The ☰ menu SHALL offer Show cart only, which does the same at any width, and Open cart on discogs.com, which opens `https://www.discogs.com/sell/cart` in the default browser. The switch SHALL be remembered per crate across restarts.

#### Scenario: Check the cart
- **WHEN** "Seller: decks.de" holds 400 records, 3 of them with a copy in the cart, and the user turns CART on
- **THEN** only those 3 records are shown, only their tracks play, and the title bar shows 3 of 400

#### Scenario: Empty cart
- **WHEN** no copy of the shown seller crate is in the cart
- **THEN** no CART switch is shown

