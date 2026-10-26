## ADDED Requirements

### Requirement: No Buy pill on what you own
An entry whose record is owned (see `discogs-cart`, "No cart pill on what you own") SHALL show no Buy pill, whatever stores it is known on, so neither the one-store action nor the Buy chooser can be reached from its row.

#### Scenario: Owned wantlist entry
- **WHEN** release 555 is on the wantlist, known only on Discogs, and the collection holds another pressing of its master
- **THEN** its row shows OWNED and no pill, and no marketplace page can be opened from it

#### Scenario: Not owned
- **WHEN** release 556 is on the wantlist, known only on Discogs, and not owned
- **THEN** its row shows + CART, and clicking it opens `https://www.discogs.com/sell/release/556`
