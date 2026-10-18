## ADDED Requirements

### Requirement: Cart requests
A paired extension SHALL be able to wait for the app's pending store cart actions, and to report their results. A wait SHALL return at once when an action is pending, and otherwise SHALL be held open up to 25 s, then answered empty. Each action given SHALL hold an id, the store (Juno, Beatport, Traxsource or Bleep), the kind (add or remove) and the item's ids, and no address. A result SHALL name an action id given and not yet answered, an outcome (added, already in, removed, sold out, not logged in, failed) and optionally the basket read from that store as item ids with prices (at most 500 items, prices ≤ 20 characters, no control characters). A result naming no pending action, or breaking any rule, SHALL be refused and change nothing. Like every request but pairing, both SHALL need the pairing key, and both SHALL be answered within 100 ms except a wait held open.

#### Scenario: Picked up at once
- **WHEN** the extension is waiting and the user clicks + CART on a Juno entry
- **THEN** the wait returns that action within 100 ms

#### Scenario: Forged result
- **WHEN** a request reports a result for an action id the app never gave
- **THEN** it is refused, and no badge changes

#### Scenario: Unpaired
- **WHEN** an unpaired client asks for pending cart actions
- **THEN** it is refused, and no action is given out
