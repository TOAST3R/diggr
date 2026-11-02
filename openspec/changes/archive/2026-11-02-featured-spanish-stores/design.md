## Context

Top Sellers (`seller-crates`) is a `SellerList` in `<config>/dig/sellers.ron`: each `Seller` holds a Discogs username, the crate of its copies and a `Source` (`Purchases` or `Added`). `seeded` records that the one-time fill from purchases ran (`dig_first_sellers`, triggered by `read_account_lists` once a token exists). A seller crate is found by the seller mark on the crate (`set_seller`), so it keeps working after a rename. A missing or broken file loads as an empty, unseeded list.

The six shops and their usernames were confirmed with the user (record shops in Madrid, Barcelona, Valencia and Gijón, plus Adepta, which is online only).

## Goals / Non-Goals

**Goals:**
- A fresh install shows the six shops under TOP SELLERS on first launch, with readable names.
- Zero network requests and no extra delay at launch.
- Existing users' lists are untouched.

**Non-Goals:**
- Updating the featured list remotely, or adding shops to existing users' lists on update.
- Logos, photos, addresses or a map in the app (the website covers that).
- A separate sidebar group for featured shops.

## Decisions

1. **A `featured` flag in `sellers.ron`, like `seeded`.** While it is false (a fresh install, or a list saved before this feature), startup applies the featured list once and saves with the flag true, so it never happens twice and a removed shop stays removed. The field has a serde default, so old files load, and an older build ignores it.
   *Alternative:* "no file yet" as the test. It was the first design, but it left existing users with `logon` under its username and none of the other shops, and a user asked for exactly that to change.
   *Note:* a broken file loads as an empty list with the flag false, so it gets the six shops again. A broken file is rare and the shops are harmless.
2. **Existing sellers.** For a featured username already in the list (compared ignoring case), its crate is renamed to `Seller: ‹Shop› · ‹City›` only if it still has the default name (`Seller: ‹username›`), so a name the user chose is kept; a taken name leaves it alone. Its place in the list and its source don't change. Featured shops the list lacks go after the user's sellers, in the featured order.

3. **Readable names come from the crate name, not a new field.** Each crate is created as `Seller: ‹Shop› · ‹City›` and marked with its username through `set_seller`, which is the path a renamed seller crate already uses. The sidebar, dialogs and refresh code need no change. The longest name, "Seller: Rhythm Control · Barcelona", is 34 characters, under `MAX_NAME` (40). Adepta's city is "Online".
   *Alternative:* a `display_name` on `Seller`. Every place that shows a seller would need to read it. Rejected.

4. **`Source::Featured`.** It records where the seller came from, as the other two do, so tests and later code can tell them apart. Behaviour is the same as `Added`.

5. **The list is a const in `dig::sellers`**: `FEATURED: [(&str, &str, &str); 6]` with shop, city and username. It's compiled in, so it works offline with nothing to download.

6. **The purchase fill is unchanged.** `dig_first_sellers` already skips usernames that are in the list and appends the rest, and `seeded` stays false until it runs. A user who buys from `logon` keeps one Melting Pot crate, under its featured name.

7. **Order:** on a fresh install, the order in the table in the proposal (Madrid, Barcelona, Valencia, Gijón, then the online shop), ahead of any purchase sellers; on an existing list, the missing ones in that order after its sellers.

## Risks / Trade-offs

- [A shop closes or changes its Discogs username] → Dig shows Discogs' error as for any seller. Fix the const in the next release. Existing installs keep the old entry until the user removes it.
- [The purchase fill keeps "the first 10 sellers"; skipped featured usernames could make it add fewer than 10 new ones] → Acceptable. The featured shop is still in the list.
- [Renaming reads the crate from disk] → A seller crate can hold thousands of entries, and `Crates::rename` loads it. It happens once, only for the featured shops the user already has, and a shop never dug has an empty crate. Fine.
- [Shops didn't ask to be listed] → The user is contacting each shop before release. The app shows only the shop's name, city and its public Discogs stock.

## Migration Plan

Existing `sellers.ron` files load as before, with the flag false, and are migrated at the first launch as in decision 2. Rollback: release without the const. Lists already created keep their crates as ordinary sellers (`Featured` must stay deserialisable, or a rolled-back build would load such a file as broken and empty. Keep the variant in any rollback).

## Open Questions

None.
