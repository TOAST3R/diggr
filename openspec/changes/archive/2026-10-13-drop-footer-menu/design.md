## Context

The playlist footer (`app.rs`, "Bottom bar") draws three skin buttons, `pl_plus` (+), `pl_menu` (≡) and `pl_opts` (the gear), each opening an `egui::Popup::menu`. The ≡ popup holds the crate's actions: Select all/none/Invert, Remove selected and Clear crate (hidden in locked crates), Sort ▸, Open cart on discogs.com (seller crates), Group by record and Export M3U…. Most also exist elsewhere: Cmd+A, Delete/Backspace, the column headers, ▤ and Shift+G. Four don't: Select none, Invert selection, Clear crate (outside Playlist), Open cart; and neither does Sort while the header is hidden (grouped, or no columns). A row's right-click opens `entry_menu` (play, remove, filters, Send to crate…).

## Goals / Non-Goals

**Goals:**
- No ≡ button in the footer; both M3U items under +.
- Every ≡ item still reachable, from the list itself.

**Non-Goals:**
- New keyboard shortcuts for Select none or Invert.
- Showing the column header while grouped.
- A real M3U import (Import M3U… keeps opening the file dialog, which reads M3U files).

## Decisions

### The crate's items move to the list's right-click
The ≡ popup's body becomes `crate_items(&self, ui, actions)`. It is used in two places:
- **On a row:** `entry_menu` ends with a separator and `ui.menu_button("Crate", |ui| self.crate_items(ui, actions))`.
- **On the list's empty space:** a right-click (or Control-click, via `opens_context_menu`) there opens `crate_items` directly. The empty space is the area below the last row, or the whole list of an empty crate.

*Why:* the right-click is where the user already acts on the list, and it reaches every item at any width and in grouped view, which is where Sort had no other route. *Alternative:* put the items in the title bar's crate switcher (`crate_menu`). Rejected: that menu picks a crate, it doesn't act on the shown one. *Alternative:* drop Select none, Invert and Open cart. Rejected: removing a button must not remove a feature.

### Export M3U… joins +
`+` holds Add files…, Add folder…, a separator, Import M3U… and Export M3U…. Its hover text becomes "Add · M3U".

### Separators only between non-empty groups
In `crate_items`, a separator is drawn only before a group that has an item. A locked crate then shows Select all/none/Invert, a separator, Sort ▸, Group by record (and Open cart), with no empty band like the one in the old ≡ menu.

### The gear takes ≡'s slot
The skin drops the `pl_menu` rect and sprites, and `pl_opts` moves left to x=32, beside `+`. The skin is generated: change `skin/generate.rs` and `skin/mod.rs`, then run `cargo run -p ui --bin skin-gen`. The skin test checks it. `layout.rs`'s wide-playlist test lists `pl_plus` and `pl_opts` only.

## Risks / Trade-offs

- [Users who knew ≡] → The README and the hover text on + say where things went. A right-click is the usual place for a list's menu.
- [The row menu grows by one line] → It is one submenu at the end, after Send to crate.
- [A full list has no empty space to right-click] → Every row's menu has Crate ▸.
