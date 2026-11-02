## 1. The featured list (`crates/dig/src/sellers.rs`)

- [x] 1.1 Add `Source::Featured`, and a `FEATURED` const with shop, city and username for the six stores, in the proposal's order
- [x] 1.2 (superseded by 2.3: a `featured` flag instead of "no file yet")
- [x] 1.3 Unit tests: a list holding a `Featured` seller survives a restart; every featured crate name (`Seller: ‹Shop› · ‹City›`) fits in `MAX_NAME`

## 2. First launch (`crates/ui/src/app/digging.rs`, `sellers.rs`)

- [x] 2.1 On startup: create each featured crate by its display name, mark it with its seller (`set_seller`), push a `Seller` with `Source::Featured`, and save the list
- [x] 2.2 Check that `dig_first_sellers` appends purchase sellers after the featured ones and skips a featured username (no code change expected)
- [x] 2.3 Replace "no file yet" with a `featured` flag in `SellerList` (serde default); one startup pass while it is false: rename the featured sellers already in the list (default-named crates only), add the missing ones after the user's, set the flag, save
- [x] 2.4 Unit test: a file without the flag loads with it false; with it, it survives a restart

## 3. Headless tests (`crates/ui/src/app/headless_tests/seller_tests.rs`)

- [x] 3.1 Fresh install: the six crates under TOP SELLERS in order, undug, with no request sent to the intake worker
- [x] 3.2 Existing `sellers.ron` (decks.de and logon): logon renamed in place, the other five added after, the flag saved
- [x] 3.3 Remove Mita · Gijón, restart: five featured sellers, and Mita isn't added back (fresh and migrated lists)
- [x] 3.4 Token saved with purchases ranking logon and decks.de: featured six first, then decks.de; logon once, as Melting Pot · Madrid
- [x] 3.5 Double-click Melting Pot · Madrid with no token opens Connect to Discogs; with a token, double-clicking Rotor · Madrid counts rotordiscos' copies
- [x] 3.6 A featured seller's crate the user renamed keeps its name; a featured name already taken leaves the seller alone

## 4. Docs and checks

- [x] 4.1 README (Top Sellers: the featured stores on a fresh install) and CHANGELOG
- [x] 4.2 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, the wasm `cargo check` all pass
- [x] 4.3 Manual check: launch with an empty config folder and see the six shops; double-click one with a token and dig it
- [x] 4.4 Before release: the user confirms every shop agreed to be listed
