## Context

`crate_sidebar` (`crates/ui/src/app.rs`) paints the sidebar as a fixed grid: `rows = floor(h / row_h)`. The Discogs group (DISCOGS, wantlist, collection, LABELS, TOP SELLERS, FRIENDS and their crates) is built bottom-up and `truncate`d to fit. The user's own crates take what is left with `.take(top_rows - 1)`. Anything past the end is dropped without a sign. There is no scroll state.

The fold flags live in three files: `Settings.labels_folded` (`settings.ron`), `SellerList.folded` and `FriendList.folded` (dig's config files). Nothing ties them together, so an accordion can't be enforced across them.

The playlist list already has row-based scrolling: `pl_scroll`, `scroll_rows(acc, first, max, dy, row)` with a carry-over accumulator for trackpads, and a skinned thumb. The sidebar can reuse `scroll_rows` as is.

None of the structs use `deny_unknown_fields`, so removing a field still loads files that have it.

## Goals / Non-Goals

**Goals:**
- No crate is ever unreachable from the sidebar, at any playlist height.
- At most one of LABELS / TOP SELLERS / FRIENDS open, held in one setting.
- Headings never move off screen, except in the single-pane fallback for very short sidebars.
- The shown crate is always found and visible after it changes, whatever changed it.

**Non-Goals:**
- Type-to-filter inside a group.
- Remembering scroll offsets across restarts.
- Reordering crates by dragging in the sidebar.
- A new skin sprite for the scrollbar: it is painted from skin colours. (The CR toggle icon is the only new sprite.)
- Discogs, label, seller or friend crates in the CRATES panel.

## Decisions

### One piece of fold state in settings

```rust
enum SideGroup { Labels, Sellers, Friends }
// Settings
side_open: Option<SideGroup>,   // the Discogs accordion
crates_folded: bool,            // CRATES, independent
```

`labels_folded`, `SellerList.folded` and `FriendList.folded` are deleted. Old files keep loading because serde skips unknown fields. The open group is a sidebar preference, not dig data, so it belongs in `settings.ron`.
*Alternative:* keep the three bools and clear the other two on each toggle. Rejected: three files that must agree, and partly written files could leave two groups open.

CRATES stays out of the accordion: the user's crates are where they work, and the Discogs groups are where they browse. Folding CRATES whenever a Discogs group opens would hide the drop targets they use most.

### Toggle actions merge

`Action::ToggleLabelsFold`, `SellerAction::ToggleFold` and `FriendAction::ToggleFold` become `Action::ToggleSideGroup(Option<SideGroup>)`, where `None` means CRATES. That also removes the `#[cfg(not(wasm32))]` around the friends and sellers toggles, since the toggle no longer touches dig state. Clicking the open group sets `side_open = None`, and clicking another group sets it to that group.

### Layout is a pure function

```rust
struct SideLayout { crates_rows: usize, group_rows: usize, single: bool }
fn side_layout(rows: usize, fixed: usize, crates_len: usize, group_len: usize) -> SideLayout
```

`fixed` counts the rows that are always shown: the CRATES heading, "+ New crate" and, when the Discogs group exists, DISCOGS, the pinned wantlist/collection rows and every present group heading. `crates_len` is 0 when CRATES is folded, and `group_len` is 0 when no group is open.

- `free = rows - fixed`. Each pane wants its length. If both fit, each gets its length.
- Otherwise each gets `min(len, free / 2)`, and the rows one pane leaves unused go to the other.
- If `free < 2` while a pane is open and longer than `free`, `single = true`: every row, headings included, goes into one list that scrolls as a whole (same order as the full layout).

Being pure makes it unit-testable without egui (rows 8…40 × lengths 0…100: panes never exceed `free`, never exceed their length, and every leftover row is used).

```
 ┌──────────────────────────┐
 │ ⏷ CRATES (23) ────────── │ fixed
 │ Playlist              12 │ ┐ pane A  (crates_rows, offset crates_scroll)
 │ …                        │ ┘
 │ + New crate              │ fixed
 │ DISCOGS ──────────────── │ fixed
 │ ◉ Wantlist / Collection  │ fixed (pinned)
 │ ⏵ LABELS (14) ────────── │ fixed
 │ ⏷ TOP SELLERS (31) ───── │ fixed
 │   Seller: …              │ ┐ pane B  (group_rows, offset group_scroll)
 │   …                      │ ┘
 │ ⏵ FRIENDS (9) ────────── │ fixed (headings below the open group stay below it)
 └──────────────────────────┘
```

### Scrolling reuses the playlist's pattern

The app gets `side_crates_scroll`, `side_group_scroll` (row offsets) and one shared accumulator. When the pointer is over a pane, the frame's `smooth_scroll_delta.y` goes through `scroll_rows` for that pane. Offsets are clamped every frame to `len - pane_rows`, which covers crates being deleted and the window being resized. Switching the open group resets `side_group_scroll` to 0, unless a reveal sets it.

The scrollbar is a painted rect 3 px wide (× skin scale) at the pane's right edge: the track in `lerp(pl_bg, pl_text, 0.12)` and the thumb in `pl_owned`. It is drawn only when `len > pane_rows`, with a thumb height of at least one row. Clicking or dragging the track jumps to and drags the thumb, like the playlist's. The crate count text moves left by the bar's width when it is shown, so they don't overlap.
*Alternative:* `egui::ScrollArea` inside a child UI. Rejected: the sidebar paints by absolute row and hit-tests drops by row rect. A ScrollArea would need the whole row drawing rewritten as widgets, and its look doesn't match the skin.

### Reveal on change, at one place

The app keeps `side_revealed: CrateId`. At the start of `crate_sidebar`, if `crates.shown_id() != side_revealed`:
1. if the shown crate is a label, seller or friend crate, `side_open` is set to its group (an accordion switch, saved like a click); if it is one of the user's crates and CRATES is folded, CRATES unfolds;
2. the matching pane's offset is set so the crate's row is in view, scrolling as little as possible (the playlist's `scroll_to` rule);
3. `side_revealed = shown_id`.

Checking the shown id at one place covers every way a crate gets shown (sidebar click, title-bar menu, bridge, new crate, friend promoted to the top, crate deleted → another shown) without touching each call site. It only fires on change, so it never fights the user's own scrolling. The pinned wantlist and collection need no reveal.
*Alternative:* reveal in `Action::ShowCrate`. Rejected: many paths call `crates.show` directly.

### Edge auto-scroll while dragging

While `pl_drag_from.is_some()` and the pointer is inside a pane's top or bottom row, that pane scrolls one row every 120 ms (an `Instant` stored on the app, with a repaint request while it's active). The drop target is the row under the pointer after the scroll, so the edge rows themselves stay valid targets.

### Busy mark on folded headings

The sidebar already knows `busy` for each seller and friend (`sidebar_sellers`, `sidebar_friends`). When TOP SELLERS or FRIENDS is not open and any of its crates is busy, the heading text ends with " …", drawn in the heading colour. Its tooltip adds "Digging…" (sellers) or "Refreshing…" (friends). LABELS gets no mark: the sidebar has no per-label busy state today, and adding one is out of scope.

### CRATES panel in the EQ slot

The player column is main, then the waveform (optional), then the EQ (275 × 116 skin px). The panel takes the EQ's slot at the same size, so switching never resizes the window or moves the playlist.

```
 ┌───────────── 275 ─────────────┐
 │ MAIN  … [WAVE][EQ][CR][PL][SYNC]
 ├───────────────────────────────┤
 │ CRATES ───────────────── [x]  │  title bar: eq_titlebar's frame, text "CRATES"
 │  • Playlist                12 │ ┐
 │    Friday                   4 │ │ ~7 rows at pl_row_h (13 px)
 │    …                          │ │ one flat list + "+ New crate", scrolls
 │    + New crate                │ ┘
 └───────────────────────────────┘
```

- **State:** a new `show_crates: bool`, kept next to `show_eq` rather than replacing both with an enum, so old `settings.ron` files load unchanged. Turning one on turns the other off. If a hand-edited file has both on, the EQ wins. `player_height` adds `eq_size.1` when either is on.
- **Contents:** the same `mine` set as the sidebar's CRATES (not wantlist or collection, not label, seller or friend crates), in creation order, then "+ New crate". No headings and no accordion: with 7 rows a flat list is the clearest. It ignores `crates_folded`: folding is a sidebar thing.
- **Shared code:** the sidebar's per-row drawing and interaction (count, • and play/pause mark, dimmed unreadable crates, tooltip, click / right-click menu / Delete key, drop target, edge auto-scroll) moves into one `crate_row` function used by the sidebar and the panel. The panel's scroll uses its own offset with the same `scroll_rows` and scrollbar.
- **Reveal:** on a change of shown crate (the same `side_revealed` check), if the shown crate is one of the user's, the panel scrolls to it.
- **Background and title bar:** the list is painted from skin colours (`pl_bg` and friends, as the sidebar is). The title bar is a generated `crates_title` sprite ("DIGGR CRATES", made like `wave_title`), optional like it, with the EQ's close button, which hides the panel.
- **Available in wide mode too:** the panel is the user's choice. Nothing switches by itself when the playlist crosses 600 px. In wide mode it just shows the same crates as the sidebar.

*Alternative:* tabs "EQ | CRATES" in the EQ title bar. Rejected: they need new title-bar sprites as well, and they don't let the slot be empty while the EQ toggle still works.
*Alternative:* the panel also lists Discogs groups with an accordion. Rejected for now: at 7 rows the always-present headings alone fill it. Discogs crates stay in the sidebar and the title-bar menu.

### Making room for the CR toggle

The toggle row is full: volume 107–175, WAVE 177, EQ 197, PL 217, SYNC 238–268. The volume slider shrinks to 48 px (107–155), and WAVE moves to 157 and EQ to 177. CR goes at 197, while PL and SYNC stay. The `tog_cr` icon (11 × 8, like `tog_eq` / `tog_pl`) is a small stack of records, drawn in `generate.rs` with `_on`/`_off` variants like the others. The skin is regenerated with `cargo run -p ui --bin skin-gen`, which the skin test checks.
*Alternative:* narrow SYNC. Rejected: its label must stay SYNC and is already tight.

### Crate menu: groups and scrolling

`crate_menu` today partitions only on `discogs()`, so label, seller and friend crates land among the user's own. It becomes: the user's crates, then the Discogs heading with wantlist and collection, then the "Labels", "Top sellers" and "Friends" headings (small weak labels, like the existing "Discogs" one) with their crates in the sidebar's order. The crate list sits in an `egui::ScrollArea::vertical()` with `max_height` = 2/3 of the app window. The New / Rename / Delete and Refresh items stay below it, outside the scroll, so they are always reachable. No accordion: a menu is opened to pick one crate, and the shown crate's row is scrolled into view when the menu opens (`scroll_to_me`).

## Risks / Trade-offs

- [Upgrade resets the folds once: users who had TOP SELLERS folded see every group closed] → the default `side_open: None` is the compact state, so nothing floods the sidebar. The CHANGELOG mentions it.
- [Reveal can open a group the user just closed (e.g. a seller crate is shown while TOP SELLERS is closed)] → it fires only when the shown crate changes, which is a deliberate act. Closing a group never changes the shown crate.
- [Very short sidebars (< fixed + 2 rows) lose pinned headings in single-pane mode] → it is the only layout that keeps every crate reachable at that size. It switches back automatically as the window grows.
- [Edge auto-scroll can overshoot on a slow frame] → it is time-based (120 ms per row), not per frame.
- [A 48 px volume slider has coarser steps with the mouse] → ↑/↓ and the wheel still step by 5%, and the drag maps the same 0–100% range.
- [Users who keep the EQ open lose it while browsing crates] → one click on EQ brings it back, and its settings are untouched (only the section is hidden).
- [Removing `folded` from the dig files makes a downgrade show those groups unfolded] → harmless: unfolded is the old default.

## Migration Plan

No data migration: the new setting defaults to everything closed except CRATES (`side_open: None`, `crates_folded: false`). The old fields are ignored on read and dropped on the next save.

## Open Questions

None.
