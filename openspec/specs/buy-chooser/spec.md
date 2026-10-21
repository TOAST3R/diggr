# buy-chooser Specification

## Purpose
Buying any dig entry's record: which stores it is known on (Discogs, Bandcamp, Juno, Bleep, Beatport, Traxsource) from what the app already holds, acting at once when that is one store, and otherwise a Buy chooser with a button per known store and a search link for each other one. The app makes no request to find a record on a store.
## Requirements
### Requirement: Known stores of a record
The app SHALL know a record on a store when the entry's origin or the app's memory links it there: a Discogs copy for sale in the crate; a Juno or Bleep product sent from that store or linked to the record's Discogs release by a store send; a Beatport or Traxsource page; a Bandcamp page; or a Discogs release. The app SHALL make no request to find a record on a store.

#### Scenario: Juno linked by release
- **WHEN** release 123456 is in the wantlist crate and an earlier Juno send linked product 1178909-01 to release 123456
- **THEN** the wantlist entry is known on Discogs and Juno

#### Scenario: Only Discogs
- **WHEN** an entry comes from a pasted Discogs label page with release 555 and no store has linked it
- **THEN** it is known only on Discogs

### Requirement: Pill with one known store acts at once
When a record is known on exactly one store, clicking its pill SHALL act on that store without a dialog: add a Discogs copy to the Discogs cart, or a Juno or Bleep item to that basket (see `discogs-cart`, `store-carts`); open the Beatport, Traxsource or Bandcamp page; or open the release's Discogs marketplace page (`https://www.discogs.com/sell/release/‹id›`) when no copy is known. The action SHALL start within one frame of the click.

#### Scenario: Wantlist entry
- **WHEN** the user clicks + CART on a wantlist entry known only as Discogs release 555
- **THEN** `https://www.discogs.com/sell/release/555` opens in the default browser and no dialog shows

#### Scenario: Bandcamp entry
- **WHEN** the user clicks + CART on an entry pasted from a Bandcamp album page
- **THEN** the album page opens in the default browser and no dialog shows

### Requirement: Buy chooser
When a record is known on no store or on two or more, clicking its pill SHALL open a dialog titled "Buy ‹artist› – ‹title›" listing:
- under KNOWN, one button per known store, showing the price or the number for sale when known; clicking one SHALL act as the one-store pill does and close the dialog;
- under SEARCH, a "‹store› ↗" link for each of Discogs, Bandcamp, Juno, Bleep, Beatport and Traxsource not under KNOWN, opening that store's search page for the record's artist and title in the default browser.

The dialog SHALL hold the keyboard until closed, SHALL close on Esc, on its close button or after a known store's action, and SHALL NOT pause or stop playback.

#### Scenario: Two stores
- **WHEN** the user clicks + CART on an entry known on Discogs (release 555, 12 for sale from €9.00) and on Juno (€11.49)
- **THEN** a dialog shows "Discogs · 12 for sale from €9.00" and "Juno · €11.49" under KNOWN, and Bandcamp, Bleep, Beatport and Traxsource links under SEARCH

#### Scenario: Search link
- **WHEN** the chooser for "Kerri Chandler – Rain" is open and the user clicks "Bandcamp ↗"
- **THEN** `https://bandcamp.com/search?q=Kerri%20Chandler%20Rain&item_type=a` opens in the default browser

#### Scenario: Keys held
- **WHEN** the chooser is open and the user presses N
- **THEN** nothing is passed, and the music keeps playing

#### Scenario: Known nowhere
- **WHEN** the user clicks + CART on an entry with an artist and title but no store link
- **THEN** the chooser shows no KNOWN section and six SEARCH links

### Requirement: Search addresses
The search address of each store SHALL be built from the record's artist (the track's artist when the record's is "Various" or empty) and title, with bracketed catalogue numbers removed and the query URL-encoded.

#### Scenario: Various artists
- **WHEN** the record artist is "Various" and the track artist is "Moodymann", record title "Black Mahogani [KDJ 33]"
- **THEN** the query is "Moodymann Black Mahogani"

