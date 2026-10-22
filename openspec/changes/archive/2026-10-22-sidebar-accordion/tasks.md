## 1. One fold state

- [x] 1.1 Add `SideGroup { Labels, Sellers, Friends }`, `side_open: Option<SideGroup>` and `crates_folded: bool` to `Settings` (`crates/ui/src/settings.rs`); remove `labels_folded`. Unit test: a `settings.ron` holding `labels_folded: true` loads, with `side_open: None`.
- [x] 1.2 Remove `folded` from `SellerList` (`crates/dig/src/sellers.rs`) and `FriendList` (`crates/dig/src/friends.rs`). Unit tests: old files holding `folded: true` still load with their sellers and friends.
- [x] 1.3 Replace `Action::ToggleLabelsFold`, `SellerAction::ToggleFold` and `FriendAction::ToggleFold` with `Action::ToggleSideGroup(Option<SideGroup>)` (`None` = CRATES): clicking the open group closes it, clicking another opens it alone, and the settings are saved. Headless test: open LABELS, then TOP SELLERS → LABELS closed. Click TOP SELLERS → all closed. Restart → state kept.

## 2. Layout

- [x] 2.1 Write `side_layout(rows, fixed, crates_len, group_len) -> SideLayout` as a pure function with unit tests: both fit; one overflows and takes the other's leftover; both overflow → half each; `free < 2` → `single`; panes never exceed `free` or their length, swept over rows 8–40 × lengths 0–100.
- [x] 2.2 Rewrite `crate_sidebar` around it: a CRATES heading with ⏵/⏷ and its count, "+ New crate" and every heading always drawn, pinned wantlist/collection, only the open Discogs group's crates, each pane drawn from its row offset, and the single-pane fallback. Remove `bottom.truncate` and `.take(top_rows - 1)`. Headless test: 31 sellers in a 20-row sidebar → every heading drawn, no crate list longer than its pane.

## 3. Scrolling

- [x] 3.1 Add `side_crates_scroll`, `side_group_scroll` and an accumulator. Feed wheel/trackpad delta over each pane through `scroll_rows`, clamp the offsets every frame, and reset the group offset when the open group changes. Headless test: scrolling over TOP SELLERS reaches the 31st seller, and CRATES doesn't move.
- [x] 3.2 Paint the 3 px scrollbar (track and thumb from skin colours, minimum one row) only when a pane overflows. Click or drag on the track moves the offset. Move the count text left of the bar. Headless test: drag the thumb to the bottom → last seller visible.
- [x] 3.3 Edge auto-scroll while dragging entries: hovering the first or last row of an overflowing pane steps it one row per 120 ms (stored `Instant`, request repaint). Headless test from the spec scenario "Drop on a crate scrolled out of view".

## 4. Reveal and busy mark

- [x] 4.1 Add `side_revealed: CrateId`. When `crates.shown_id()` differs, open its group (or unfold CRATES), scroll its pane minimally to show it, then record it. Headless tests: a seller shown via `crates.show` while LABELS is open → TOP SELLERS opens and the row is in view. A user scroll is kept until the shown crate changes.
- [x] 4.2 Busy mark: a closed TOP SELLERS or FRIENDS heading with a busy crate ends in " …", and the tooltip says "Digging…" or "Refreshing…". Headless test with a seller marked busy.

## 5. CRATES panel and crate menu

- [x] 5.1 Skin: in `generate.rs`, shrink `volume` to 48 px, move `wave_toggle` to 157 and `eq_toggle` to 177, add `cr_toggle` at 197 and the `tog_cr` on/off icon. Register the sprites in `skin/mod.rs`. Run `cargo run -p ui --bin skin-gen` and check that the skin test passes. Check that a volume drag still covers 0–100%.
- [x] 5.2 Add `show_crates` to `Settings` and `Action::ToggleCrates`: turning on EQ or CRATES turns the other off, and a file with both on resolves to the EQ. Make `player_height` count the slot for either. Draw the CR toggle lit from `show_crates` and the EQ toggle from `show_eq`. Unit tests: exclusivity, an old `settings.ron` loads with the EQ, the height is unchanged by a switch.
- [x] 5.3 Pull the sidebar's per-crate row drawing and interaction into a shared `crate_row` function (marks, count, tooltip, click and menu, Delete key, drop target, edge auto-scroll). The sidebar uses it with no visible change (existing sidebar headless tests still pass).
- [x] 5.4 Draw the CRATES panel in the EQ slot: the EQ title bar frame with "CRATES" and a close button, the user's crates plus "+ New crate" through `crate_row`, its own scroll offset, the scrollbar and reveal. Headless tests from the spec: only the user's crates, 20 crates scroll to the end, pick and drop at a 275 px playlist, restart keeps it.
- [x] 5.5 Rework `crate_menu`: the user's crates, then Discogs, Labels, Top sellers and Friends groups (headings only when not empty), inside a `ScrollArea` with `max_height` of 2/3 of the window, with the shown crate scrolled into view. New / Rename / Delete and the refresh items stay below the scroll. Headless test with 5 + 14 + 31 + 9 crates: grouped, and the New / Rename / Delete items reachable.

## 6. Tests, docs, checks

- [x] 6.1 Move the fold assertions in `crates/ui/src/app/headless_tests/seller_tests.rs` (and any friend/label fold tests) to `settings.side_open`. Remove the checks of the old fields.
- [x] 6.2 Update README (CRATES panel and CR toggle; sidebar: CRATES fold, one Discogs group open at a time, scrolling) and its test count. Add `## Unreleased` entries to CHANGELOG.md with the `changelog` skill, mentioning that groups start closed once after the upgrade.
- [x] 6.3 Run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check` and the wasm `cargo check -p audio -p platform`. Run the app with 30+ sellers and a short playlist window to check scrolling, the accordion, reveal, the CRATES panel and the crate menu by eye.
