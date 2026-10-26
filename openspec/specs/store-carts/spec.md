# store-carts Specification

## Purpose
Buying from the stores where you dig: a cart pill on Juno and Bleep entries that puts the item in your real basket on that store through the paired browser extension, in your browser's session; the outcomes and the fallback opening the store's page (always, for Beatport and Traxsource, whose baskets need an account); and the baskets kept in `store-carts.ron` for IN CART at launch. Never checks out, pays or empties a basket.
## Requirements
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

### Requirement: Store cart actions through the browser
The app SHALL perform store cart actions only through the paired browser extension, in the user's browser session (see `browser-bridge`, Cart requests). Each action SHALL be queued with the store and the item's ids, rebuilt from the entry, and SHALL never carry an address. The app SHALL NEVER check out, pay, or empty a store basket, and SHALL make no request to the stores itself.

#### Scenario: Only ids
- **WHEN** the user adds a Bleep format
- **THEN** the queued action holds the store and the format id only, and the app makes no request to bleep.com

### Requirement: Store cart outcomes
Each store cart action SHALL end as one of:
- added, or already in the basket: IN CART, counted as added;
- removed: + CART;
- sold out or unavailable: SOLD on that entry, and the main window says so;
- anything else (no extension picked it up within 3 s, the user isn't logged in, the store refused or answered unexpectedly, no result within 10 s, or the store is one whose basket can't be reached, see design D1): the pill SHALL return to its state before the click, the item's page SHALL open on the store in the default browser, and the main window SHALL say the basket couldn't be reached and the page was opened instead.

#### Scenario: No extension
- **WHEN** the user clicks + CART on a Juno entry while no paired extension is running
- **THEN** after at most 3 s the pill reads + CART again, the product's page opens on juno.co.uk, and the main window says it was opened there instead

#### Scenario: Not logged in
- **WHEN** the extension reports that the user isn't logged in to Bleep
- **THEN** the release's page opens on bleep.com, and the main window says to log in there

### Requirement: Store CART badge
The app SHALL keep, per store, the last basket the extension read, in `store-carts.ron` in the cache folder, shown at once at the next launch. Items the app added SHALL count as in the basket until a later read says otherwise. Any entry in any crate whose store item is in a basket SHALL show the CART badge, with the store and price in its tooltip ("In your Beatport cart, $1.69"); for a Juno or Bleep record, every track of it SHALL show it. A store whose basket was never read SHALL add no badge of its own beyond items added from the app.

#### Scenario: Added in the browser
- **WHEN** the user adds a Juno record on juno.co.uk, and the extension later reads the Juno basket
- **THEN** every entry of that record shows CART

#### Scenario: At launch
- **WHEN** the app starts with a Juno product in the cached basket
- **THEN** its entries show CART in the first frame, before any read

### Requirement: Store checkout pill
A Juno or Bleep entry whose item is in that store's basket SHALL show the `PAY »` checkout pill after its IN CART pill or CART badge (see `discogs-cart`, Checkout pill). A click SHALL open the store's basket page in the default browser: `https://www.juno.co.uk/cart/` for Juno and Bleep's basket page for Bleep. Its tooltip SHALL name the site ("Check out on juno.co.uk"). Beatport and Traxsource entries SHALL show no `PAY »`, since their baskets can't be read. An owned store entry SHALL follow `discogs-cart` "No cart pill on what you own".

#### Scenario: Juno basket
- **WHEN** a Juno entry's product is in the Juno basket
- **THEN** its row shows IN CART then `PAY »`, and clicking `PAY »` opens `https://www.juno.co.uk/cart/`

#### Scenario: Owned Juno record
- **WHEN** a Juno entry is linked to Discogs release 555, the collection holds release 555, and its product isn't in the basket
- **THEN** its row shows OWNED and no cart pill

