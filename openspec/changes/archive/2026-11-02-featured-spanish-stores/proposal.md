## Why

A new user opens Diggr to an empty Top Sellers list: it only fills once they connect Discogs and have purchases to rank. Shipping a few hand-picked Spanish record stores that sell on Discogs gives every fresh install shops to dig straight away, supports the local scene Diggr comes from, and gives a reason to connect Discogs (a double-click without a token opens Connect to Discogs).

## What Changes

- **A fresh install starts with six featured record stores in Top Sellers**, each a seller crate named after the shop and its city, never dug until the user double-clicks it:

  | Crate name | Discogs seller |
  |---|---|
  | Seller: Melting Pot · Madrid | `logon` |
  | Seller: Rotor · Madrid | `rotordiscos` |
  | Seller: Rhythm Control · Barcelona | `djbrucelee` |
  | Seller: KH Vinyl Store · Valencia | `KHVinylStore` |
  | Seller: Mita · Gijón | `MITA_Record_Store` |
  | Seller: Adepta · Online | `ADEPTA_STORE` |

- **Once per install**, on the first launch of a version with this feature: a fresh install gets all six; a user who already has a list keeps it and gets two things. A seller already in it that is a featured shop (e.g. `logon`, bought from) has its crate renamed to the shop's name, unless the user renamed it. The featured shops it lacks are added after its sellers. Then a flag in the file says it's done, so a shop the user removes never comes back.
- **They are the user's like any seller**: removable, renamable, refreshable; a removed one never comes back.
- **The first list from purchases still runs** when a token is saved: its sellers come after the featured ones, and a featured shop the user buys from is not listed twice.
- No change to the extension, the bridge, the website, playback or analysis. The website's `/stores/` pages are a separate change in the `diggr_es` repo.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `seller-crates`: adds "Featured record stores": the list a fresh install starts with, and how it meets the first list from purchases.

## Impact

- **`crates/dig/src/sellers.rs`:** `Source::Featured`; the featured list (shop name, city, username) as a const; a `featured` flag in the list file. Tests.
- **`crates/ui/src/app/sellers.rs` / `app.rs`:** on startup, while the flag is unset, rename the existing featured sellers' crates, create the missing ones (named, marked with their seller), set the flag and save the list. Headless tests.
- **Docs:** README (Top Sellers), CHANGELOG.
- No new dependencies, no network at launch.
