## 1. One owned rule

- [x] 1.1 Add `DiggrApp::owns(e) -> Option<String>` (the owned pressing's text) (design D1): collection crate shown or entry sent from a collection page » `ThisPressing`, else `Dig::owned(e)`; `None` on wasm. Make `dig_marks` keep using the badge rule (no OWNED in the collection crate) and share the "sent from a collection page" check with `owns` instead of duplicating it
- [x] 1.2 Unit or headless test: `owns` is `Some` for every entry of the collection crate even with no token, `Some(Another)` for a label-crate entry whose master is owned, and `None` with no token outside the collection crate

## 2. Owned rows get no buying pill

- [x] 2.1 Track rows (`app.rs`, the `store_pill` filter): drop the pill when owned and not `in_cart`; keep IN CART when in the cart
- [x] 2.2 Record rows: no lone-copy pill, no several-copies opener and no store/Buy pill when owned, unless the copy or item is in a cart (then IN CART as today)
- [x] 2.3 Copy rows (`copy_row`): no pill on an unsold copy of an owned record unless it is in the cart (owned asked through the record's first entry: release and master)
- [x] 2.4 Owned-in-cart tooltip (design D3): prefix "You already own this (‹owned_text›). " (or "You already own this. " in the collection crate) on the Discogs `cart_pill_click` tooltip and on `item_pill`'s in-cart tip
- [x] 2.5 Headless tests in `store_cart_tests.rs` / `seller_tests.rs`: collection crate rows have no pill; an owned label-crate row shows OWNED and no pill; another pressing too; a seller crate's owned record has no record or copy pill; owned + in cart shows IN CART with the warning tooltip and REMOVE still removes; no token keeps + CART

## 3. Store cart pages

- [x] 3.1 Add `Store::cart_page() -> Option<&'static str>` in `dig` (Juno `https://www.juno.co.uk/cart/`, Bleep its basket page, Beatport/Traxsource `None`), with a unit test
- [x] 3.2 Confirm Bleep's basket page URL on the live site; fall back to `https://bleep.com/` if no stable page exists, and note it in design D5
- [x] 3.3 Replace `SellerAction::OpenCart` with an action carrying the cart to open (Discogs or a `Store`), opening `CART_PAGE` or `cart_page()` and saying "Couldn't open the browser: …" on failure

## 4. The » checkout pill

- [x] 4.1 `Badges` gains `checkout: Option<CartSite>`; `entry_badges` draws the narrow `»` pill after IN CART or CART (outline in `CART_FILL`, filled while hovered, fixed width fitting "»"; the slot is kept after every cart pill) and returns its rect beside the cart pill's
- [x] 4.2 Pick the site per row (design D4): the pill's store; for a CART badge, Discogs when the Discogs cart holds a copy of the release, else the store whose basket holds the item
- [x] 4.3 Register its click after the row's, as `cart_pill_click` does (no select, play, drag or open), with the tooltip "Check out on discogs.com" / "juno.co.uk" / "bleep.com", pushing the open-cart action
- [x] 4.4 Wire it on track rows, record rows (including the several-copies IN CART opener) and copy rows
- [x] 4.5 Headless tests: after + CART on a seller copy, `»` shows and opens `https://www.discogs.com/sell/cart` without selecting the row; a wantlist entry with a copy in the cart shows CART then `»`; a Juno IN CART entry's `»` opens `https://www.juno.co.uk/cart/`; a + CART row has no `»`; a Beatport entry never shows `»`

## 5. List menu

- [x] 5.1 Show "Open cart on discogs.com" in seller crates and in every crate while the Discogs cart snapshot holds a copy; "Open cart on juno.co.uk" / "bleep.com" while that basket holds an item; keep the separator rule
- [x] 5.2 Headless test: a label crate with 2 copies in the Discogs cart and an empty Juno basket offers only Open cart on discogs.com; with every cart empty, no Open cart item

## 6. One cart pill per record

- [x] 6.1 In the list, consecutive rows of a record share one cart mark (pill or CART, and `»`), centred on the run in view, with the slot kept on every row so titles line up and lines above and below the run; its clicks registered after the rows
- [x] 6.2 Headless test: a wantlist record of three tracks shown flat has one + CART, centred on the middle track, the titles line up, and its click selects no row

## 7. Docs and checks

- [x] 7.1 CHANGELOG `## Unreleased` entries (changelog skill): no + CART on records you own; `»` straight to checkout; Open cart from any crate
- [x] 7.2 README: update the Tests count, and any text describing the cart pill or the list menu
- [x] 7.3 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo check -p audio -p platform --target wasm32-unknown-unknown`
