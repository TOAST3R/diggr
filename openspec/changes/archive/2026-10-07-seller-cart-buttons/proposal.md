## Why

In a seller crate the cart is what you are there for, but adding a copy takes a right-click, a menu and sometimes a submenu, and the CART badge only says what is already in. A button on the row itself makes it one click, both ways. With buttons there, the cart items in the right-click menus are redundant. The menus also carry Render show…, a video export that has nothing to do with digging and is rarely used from the player; it stays on the command line.

## What Changes

- **Cart buttons in seller crates.** In a seller crate, the CART badge becomes a clickable pill, drawn in the badge style at the same place:
  - **+ CART** on a copy that is not in the cart: one click adds it (the same Add to cart as today, with the same outcomes);
  - **IN CART** on a copy in the cart, turning into a red **REMOVE** while hovered: one click takes it out;
  - on every unsold copy row, and on a record row whose record has exactly one unsold copy (the pill acts on that copy);
  - a record row with several unsold copies keeps the plain CART badge as a summary when one of them is in the cart;
  - sold copies keep SOLD and get no pill. Without a token, the pill opens Connect to Discogs.
- **CART badge elsewhere is unchanged.** Wantlist, collection, digging crates and the flat view of a seller crate keep the plain badge and its tooltip.
- **BREAKING (UI): no cart items in the right-click menus.** The copy row's menu loses Add to cart / Remove from cart (Open on discogs.com stays). Track and record rows lose Add to cart (‹price›), the Add to cart ▸ submenu, and Remove from cart.
- **BREAKING (UI): Render show… is removed from the player.** The playlist menu item, its options dialog, the background render with its progress line and Cancel show render, and the remembered size, frame rate and title card settings all go, in every crate. `diggr --render-show` and the offline renderer are kept as they are.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `discogs-cart`: "Add to cart" moves from the right-click menus to the + CART / IN CART pill in seller crates; "CART badge" says a seller crate shows the pill where it can act and the badge elsewhere.
- `record-view`: "Copy rows" shows the pill instead of the CART badge; "Record row" shows the pill for a record with one unsold copy.
- `playlist`: "Entry context menu" no longer lists Render show.
- `show-render`: "Render from the player" is removed; the command line stays the only way to render.

## Impact

- **`crates/ui`:**
  - `app.rs`: the pill (drawing, hover, click) next to `entry_badges`, in `copy_row` and `record_row`; the copy row's menu; the Render show menu item, dialog, job, progress line, `Action::RenderShow` / `CancelRender`, and `show_renderer` in the app's context;
  - `app/digging.rs`: the cart items in `dig_entry_menu`;
  - `render_job.rs`: removed (the dialog, `RenderJob`, `ShowRenderer`, `RenderRequest`); `size_ok` moves to `visuals::render`;
  - `settings.rs`: `render_size`, `render_fps`, `render_overlay` dropped (old settings files still load: serde ignores them);
  - `help.rs`: the copy-row line, and a line for the pill.
- **`crates/visuals`:** `render.rs` loses `BackgroundRenderer` and its job (only the player used them) and its test; keeps `render_show`, `Options` and `size_ok`.
- **`apps/native`:** `main.rs` no longer passes a show renderer to the GUI.
- **README:** the cart section (the pill instead of right-click), Render show from the player removed, the test count.
- **Tests:** `seller_tests.rs` (CART counts, pill add and remove, menus without cart items); the render dialog tests go with `render_job.rs`.
- No new dependency; nothing on the playback path.
