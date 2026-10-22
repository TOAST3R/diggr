## Why

The crate sidebar is a fixed grid of rows: when its crates don't fit, the ones at the end are silently left out (FRIENDS first, then sellers, then the user's own crates and "+ New crate"). A digger with 30 Top Sellers or a long list of crates can't reach them from the sidebar at all, and with LABELS, TOP SELLERS and FRIENDS all unfolded the groups crowd each other out.

## What Changes

- **LABELS, TOP SELLERS and FRIENDS become an accordion.** Opening one closes the other two. Clicking the open heading closes it, so all three can be closed. Which one is open is remembered across restarts.
- **The user's own crates get a CRATES heading** that folds on its own, independently of the accordion. The wantlist and collection crates stay pinned under DISCOGS and never fold.
- **Nothing is left out any more.** Every heading, "+ New crate" and the two pinned Discogs crates are always visible. The CRATES list and the open Discogs group each scroll by row (wheel or trackpad) and show a thin scrollbar when they hold more than they can show. Without enough room for the two-pane layout, the whole sidebar scrolls as one list.
- **The shown crate is revealed:** whenever the shown crate changes (sidebar, title-bar menu, the browser bridge adding a seller or label, a friend promoted to the top…), its group opens and the crate is scrolled into view.
- **Dragging entries near a pane's top or bottom edge scrolls it**, so every crate can be a drop target.
- **A folded or closed TOP SELLERS or FRIENDS heading shows that a dig or refresh is running inside it.**
- **New CRATES panel in the EQ's place.** A CR toggle on the main window, between EQ and PL, swaps the EQ section for a panel of the same size. The panel lists the user's own crates (no Discogs, label, seller or friend crates), then "+ New crate", in one flat list that scrolls. The EQ and the panel are never shown together. This gives the narrow playlist, which has no sidebar, a crate list that is always visible, without taking any of the playlist's width. To make room for the toggle, the volume slider shrinks from 68 to 48 skin px.
- **The title-bar crate menu gets groups and scrolls:** label, seller and friend crates are listed under LABELS, TOP SELLERS and FRIENDS headings after the Discogs crates, not mixed into the user's crates. The menu scrolls instead of growing past the window.
- The three old fold flags (`labels_folded` in `settings.ron`, `folded` in the sellers and friends files) are no longer read. Every group starts closed once after the upgrade. Old files still load.

Out of scope: type-to-filter inside a group, remembering scroll positions across restarts, reordering crates in the sidebar.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `crates`: "Crate sidebar" covers the CRATES heading, the Discogs accordion, scrolling panes, revealing the shown crate and the busy mark. "Drop entries on a crate" covers edge auto-scroll while dragging. "Crate menu" groups label, seller and friend crates and scrolls. New requirement "Crates panel".
- `player-window`: "Transport and sliders" adds the crates toggle to the toggle row, with a shorter volume slider.
- `friend-crates`: "Friends list from Discogs" no longer saves whether FRIENDS is folded in the friends file (the sidebar setting holds it).

## Impact

- **`crates/ui/src/app.rs`:** `crate_sidebar` is rewritten around a pure layout function (rows → pane sizes and offsets), with per-pane scroll offsets, scroll input, a painted scrollbar, edge auto-scroll while dragging, and reveal-on-shown-change. `SideRow` gains a CRATES heading. `Action::ToggleLabelsFold` and the seller/friend `ToggleFold` actions become one "toggle group" action.
- **`crates/ui/src/settings.rs`:** `labels_folded` is replaced by `side_open: Option<SideGroup>` and `crates_folded: bool`. A new `show_crates: bool` is added, never on at the same time as `show_eq`.
- **`crates/ui/src/layout.rs`:** `player_height` counts the EQ slot when either the EQ or the crates panel shows.
- **`crates/ui/src/skin/generate.rs` + `assets/skin/default/`:** new `tog_cr` icon and `cr_toggle` rect, `volume` 68 → 48 px, WAVE and EQ toggles moved 20 px left. The skin is regenerated with `skin-gen`.
- **`crate_menu`:** grouped headings inside an `egui::ScrollArea` with a maximum height.
- **`crates/dig/src/sellers.rs`, `crates/dig/src/friends.rs`:** `folded` is removed from `SellerList` and `FriendList` (serde ignores it in old files).
- **Tests:** headless sidebar tests in `crates/ui/src/app/headless_tests/seller_tests.rs` move to the new state, plus unit tests for the layout function.
- **Docs:** README (sidebar), CHANGELOG `## Unreleased`.
- No change to playback, analysis, the bridge protocol or the skin files.
