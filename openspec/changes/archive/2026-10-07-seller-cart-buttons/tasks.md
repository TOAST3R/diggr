## 1. The cart pill

- [x] 1.1 `app.rs`: `cart_pill` helper next to `entry_badges`: the badges' font, padding and corner radius; a fixed width for the widest of "+ CART" / "IN CART" / "REMOVE"; + CART outlined in `CART_FILL` (lightly filled on hover), IN CART filled `CART_FILL`, REMOVE filled `SOLD_FILL` while hovered; a hit area grown to the row's height; pointing-hand cursor; tooltip naming the action and the copy. It returns its width (with the gap) and the click. Unit test for the fixed width (the same for all three labels)
- [x] 1.2 `copy_row`: an unsold copy draws the pill instead of the CART badge (SOLD unchanged), registered after the row's `interact`; a click pushes `CartAdd(vec![listing])` or `CartRemove(listing)`; double-clicking the row outside the pill still opens the listing
- [x] 1.3 `record_row`: in a seller crate, a record with exactly one unsold copy draws that copy's pill where CART was, after the row's `interact` like ⏵/⏷; a record with several unsold copies keeps the CART badge; track rows and the flat view keep the badge
- [x] 1.4 Headless tests in `seller_tests.rs`:
  - + CART on a copy row adds it, and the pill reads IN CART within the frame without selecting the row;
  - hovering IN CART reads REMOVE, and clicking it removes the copy;
  - a one-copy record row adds that copy without opening the record;
  - a record row with two unsold copies shows the CART badge and no pill;
  - a sold copy shows SOLD and no pill;
  - a double-click on the pill doesn't open the listing;
  - unsold copy rows' prices start at the same x;
  - without a token, the pill opens Connect to Discogs;
  - the wantlist crate and the flat view still show the CART badge.
  Update the existing tests that count `"CART"` texts

## 2. Menus without cart items

- [x] 2.1 `copy_row`: the context menu loses Add to cart / Remove from cart and keeps Open on discogs.com
- [x] 2.2 `app/digging.rs` `dig_entry_menu`: remove the seller-crate block (Add to cart (‹price›), Add to cart ▸, Remove from cart)
- [x] 2.3 Headless tests: right-clicking a copy, track and record row in a seller crate offers no cart item; the copy menu still offers Open on discogs.com

## 3. Render show leaves the player

- [x] 3.1 Move `size_ok` into `visuals::render` (and its test); `visuals::render` stops importing `ui::render_job`; remove `BackgroundRenderer`, `Job` and `background_jobs_finish_pause_and_cancel`; `--render-show` still builds and its tests pass
- [x] 3.2 `crates/ui`: delete `render_job.rs` and `pub mod render_job`; remove `show_renderer` from the app context and `App`, `render_job`, `render_dialog`, `render_looks`, `open_render_dialog`, `render_dialog_ui`, the progress line, `Action::RenderShow` / `CancelRender`, the menu item, and `entry_menu`'s `rendering` parameter
- [x] 3.3 `settings.rs`: drop `render_size`, `render_fps`, `render_overlay` and their clamping; test that a `settings.ron` that still has those keys loads with every other setting kept
- [x] 3.4 `apps/native/main.rs`: stop passing a show renderer; headless test that no entry menu offers Render show (local and Discogs crates)

## 4. Docs and checks

- [x] 4.1 `help.rs`: replace the copy-row line ("open its listing / Add to cart, Remove from cart") with the pill (+ CART adds, IN CART removes) and double-click / right-click for the listing
- [x] 4.2 README: the cart section describes the pill instead of right-click; "Render a show to video" drops the player paragraph (Render show…, the dialog, Cancel show render) and keeps the command line; the test count in Tests
- [x] 4.3 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo check -p audio -p platform --target wasm32-unknown-unknown` all clean
- [x] 4.4 Run the app: in a seller crate, add and remove a copy with the pill, and check against the real cart on discogs.com; right-click menus show no cart items and no Render show
