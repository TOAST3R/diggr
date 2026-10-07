## Context

- **The cart is per copy.** `SellerAction::CartAdd(Vec<listing>)` and `CartRemove(listing)` (`app/sellers.rs`) act on listings, not releases. They already update the badge within the frame and report the outcome in the main window. This change only adds a new way to trigger them; the actions stay the same.
- **Three row kinds show CART in a seller crate** today, all through `entry_badges` (`app.rs`):
  - the record row (`record_row`, from `dig_marks(first).cart`);
  - the copy row (`copy_row`, from `copy_in_cart(listing)`);
  - the track row (flat view, or a track under an open record).
- **Two menus offer the cart.** `copy_row` has its own context menu (Add/Remove from cart, Open on discogs.com), and `dig_entry_menu` (`app/digging.rs`) adds Add to cart (‹price›) or Add to cart ▸ to track and record rows of a seller crate.
- **Clicks inside a row already work this way:** `record_row` registers the row's `interact` first, then the ⏵/⏷ mark's own `interact`, which takes its clicks (egui gives the click to the widget added last). The pill uses the same pattern.
- **Render show from the player** is `ui::render_job` (dialog, `RenderJob`, `ShowRenderer`, `RenderRequest`, `size_ok`), implemented in `visuals::render::BackgroundRenderer` and passed in by `apps/native/main.rs`. The command line (`apps/native/render.rs`) uses only `visuals::render::{render_show, Options, size_ok}`.

## Goals / Non-Goals

**Goals:**
- One click adds a copy to the cart or removes it, on the row, in a seller crate.
- The row still reads the same: the pill sits where the badge was and is as tall as the badge.
- No ambiguity: a pill always acts on exactly one listing.
- Render show leaves the player completely, and the command line works as before.

**Non-Goals:**
- Pills outside seller crates, or on track rows.
- A choice of copy from a record row with several copies (open the record instead).
- Keyboard shortcuts for the cart.
- Any change to the cart actions, their outcomes, the CART switch or the snapshot.

## Decisions

### 1. Which rows get a pill

| Row (seller crate, grouped) | Shows |
|---|---|
| copy row, unsold, not in cart | **+ CART** pill |
| copy row, unsold, in cart | **IN CART** pill (REMOVE on hover) |
| copy row, sold | SOLD badge, as today |
| record row, exactly one unsold copy | pill for that copy |
| record row, 2+ unsold copies | CART badge if any is in the cart, else nothing |
| record row, all sold | SOLD, as today |
| track row (open record or flat view) | CART badge, as today |

A record with one unsold copy and some sold ones counts as one unsold copy. Track rows keep the badge: a pill on every track of an album would repeat the same button three to eight times, and the record row right above already has it.

*Alternative considered:* a pill on every record row, with a popup to pick the copy when there are several. Rejected because it needs two clicks, the same as opening the record, and the copy rows show more about each copy than a popup could.

### 2. The pill is drawn like a badge and is fixed-width

A `cart_pill(ui, painter, at, listing, in_cart, …) -> (f32, Response)` helper next to `entry_badges`:
- **Same geometry as a badge:** the small font (0.78 × row font) and the same padding and corner radius, so a row with a pill is as tall as a row with a badge.
- **Fixed width:** the widest of its three labels ("+ CART", "IN CART", "REMOVE"), so the price column on copy rows stays aligned and hovering doesn't shift the text.
- **Looks:**
  - **+ CART:** an outline in `CART_FILL` with `CART_FILL` text, filled lightly on hover.
  - **IN CART:** filled `CART_FILL` with background-coloured text, as the badge is today.
  - **REMOVE:** IN CART while hovered, filled `SOLD_FILL`.
- **Cursor and tooltip:** the pointing hand on hover. The tooltip names the action and the copy ("Add €9.00 · VG+ / VG to your cart", "Remove from your cart").
- **Drawing order:** it is drawn first among the badges (OWNED doesn't occur in a seller crate's copy rows; on a record row OWNED, then the pill, then SOLD, then format, in today's order with the pill where CART was).

*Alternative considered:* an `egui::Button`. Rejected because its frame, font and height don't match the skin's rows, and it would make the row taller.

### 3. Click handling

The pill's `ui.interact(rect, Id::new(("pl_cart_pill", listing)), Sense::click())` is registered **after** the row's own `interact`, as the ⏵/⏷ mark is. So:
- a click on the pill doesn't select the row, play it, or start a drag;
- a double-click on the pill doesn't open the listing (the row's `double_clicked` never fires, because the pill took the pointer);
- a click → `DigAction::Seller(CartAdd(vec![listing]))` or `CartRemove(listing)`, pushed to `actions` like the menu did.

Without a token, the same actions already open Connect to Discogs (`discogs-write`), so the pill needs nothing extra. A second click while the add is pending sends the opposite action, because the state flipped within the frame. That is the expected toggle, and it matches the menu.

### 4. Menus

- **`copy_row`:** the context menu keeps Open on discogs.com and loses the two cart buttons.
- **`dig_entry_menu`:** the whole "A seller crate: its copies go in the cart" block goes. Nothing else in that menu changes.

### 5. Render show leaves the UI

The player-side render is removed, and the offline renderer stays:

**Removed:**
- `crates/ui/src/render_job.rs` and `pub mod render_job`;
- in the app: `show_renderer` in the app context and in `App`, `render_job`, `render_dialog`, `render_looks`, `open_render_dialog`, `render_dialog_ui`, the progress line, `Action::RenderShow` / `CancelRender`, and the menu item. `entry_menu` loses its `rendering` parameter;
- `settings.rs`: `render_size`, `render_fps`, `render_overlay` and their clamping. `Settings` is `#[serde(default)]` and doesn't deny unknown fields, so an existing `settings.ron` still loads and the old keys are dropped at the next save;
- `visuals::render::{BackgroundRenderer, Job}` and the `background_jobs_finish_pause_and_cancel` test (only the player used them);
- in `apps/native/main.rs`: the `show_renderer: Some(…)` argument.

**Kept:**
- `size_ok` moves from `ui::render_job` into `visuals::render`, where the command line already imports it from;
- `visuals::render` stops importing from `ui::render_job`;
- `render_show`, `Options`, the frame-exact and determinism tests, and `--render-show` are untouched.

*Alternative considered:* hide the item only in Discogs crates. Rejected: the user wants the feature out of the player altogether, and the background job is the only reason `visuals` implements a `ui` trait for rendering.

## Risks / Trade-offs

- **[Accidental clicks]** A pill puts a real Discogs write one click away, where the row used to need a right-click first. → It acts on one copy, the outcome is announced, and the opposite click undoes it. The app still never empties the cart.
- **[Multi-copy records need an extra step]** Adding a copy of a record with several copies now means opening the record. → The record row shows "3 copies €9.00–€18.00" on its second line, so it's clear there is a choice to make.
- **[Flat view has no way to add]** With the menu items gone, a seller crate in the flat view can't add to the cart. → Seller crates open grouped. The flat view still shows CART, and a track's tooltip lists the copies. The `discogs-cart` delta says so.
- **[Users of Render show in the player]** It is gone from the GUI. → The README points to `--render-show`, which has every option the dialog had.
- **[Hit area on small rows]** At 1× skin scale the pill is about 8 px tall. → The interact rect is the pill grown to the row's full height (the badge stays drawn at its size), so it's easy to hit.
