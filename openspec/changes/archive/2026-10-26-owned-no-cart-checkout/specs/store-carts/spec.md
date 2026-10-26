## ADDED Requirements

### Requirement: Store checkout pill
A Juno or Bleep entry whose item is in that store's basket SHALL show the `PAY »` checkout pill after its IN CART pill or CART badge (see `discogs-cart`, Checkout pill). A click SHALL open the store's basket page in the default browser: `https://www.juno.co.uk/cart/` for Juno and Bleep's basket page for Bleep. Its tooltip SHALL name the site ("Check out on juno.co.uk"). Beatport and Traxsource entries SHALL show no `PAY »`, since their baskets can't be read. An owned store entry SHALL follow `discogs-cart` "No cart pill on what you own".

#### Scenario: Juno basket
- **WHEN** a Juno entry's product is in the Juno basket
- **THEN** its row shows IN CART then `PAY »`, and clicking `PAY »` opens `https://www.juno.co.uk/cart/`

#### Scenario: Owned Juno record
- **WHEN** a Juno entry is linked to Discogs release 555, the collection holds release 555, and its product isn't in the basket
- **THEN** its row shows OWNED and no cart pill
