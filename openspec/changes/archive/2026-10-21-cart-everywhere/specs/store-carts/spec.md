## MODIFIED Requirements

### Requirement: Store cart pill
An entry sent from Juno, Beatport, Traxsource or Bleep, or linked to a Juno or Bleep product through its Discogs release, SHALL show a cart pill on its row, drawn as the Discogs cart pill is (see `discogs-cart`, Add to cart):
- **+ CART** when its item is not in the user's basket on that store: clicking it adds the item;
- **IN CART** when it is, reading **REMOVE** in the SOLD colour while hovered: clicking it takes the item out.

Beatport and Traxsource baskets can't be reached (design D1): their pill SHALL queue nothing and take the fallback of Store cart outcomes at once. The item SHALL be the track for Beatport and Traxsource, the record's product for Juno, and for Bleep the record in one format: a Bleep record sold in more than one format SHALL open a menu of its formats with their prices, and one format SHALL be added directly. When the record is also known on another store, the click SHALL open the Buy chooser (see `buy-chooser`) instead, with the store's item under KNOWN. Entries without a store item SHALL show the pill `buy-chooser` describes. In a seller crate, a copy row's Discogs pill SHALL take precedence. A click on the pill SHALL NOT select, play, drag or open its row; its tooltip SHALL name the action, the store and the price ("Add to your Beatport cart, $1.69"). The pill SHALL change within one frame of the click, before the store answers.

#### Scenario: Add a Juno record
- **WHEN** the user clicks + CART on an entry sent from a Juno chart, known on no other store, with the extension running
- **THEN** the pill reads IN CART within the frame, the product is in the user's Juno basket, and the main window says "Added to your Juno cart"

#### Scenario: Beatport opens the page
- **WHEN** the user clicks + CART on an entry sent from a Beatport chart, known on no other store
- **THEN** no action is queued, the track's page opens on beatport.com at once, and the main window says the basket can't be reached from Diggr

#### Scenario: Bleep formats
- **WHEN** the user clicks + CART on a Bleep entry whose record is sold on vinyl at £22.99 and digital at £7.99
- **THEN** a menu offers "Vinyl £22.99" and "Digital £7.99", and choosing one adds that format

#### Scenario: Remove
- **WHEN** the user clicks REMOVE on an IN CART Juno entry
- **THEN** the product is taken out of the Juno basket and the pill reads + CART

#### Scenario: Juno and Discogs
- **WHEN** the user clicks + CART on a Juno entry linked to Discogs release 555
- **THEN** the Buy chooser opens with Juno and Discogs under KNOWN
