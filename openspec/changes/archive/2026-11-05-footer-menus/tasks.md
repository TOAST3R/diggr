# Tasks

## 1. Export any crate

- [x] 1.1 Change `Action::ExportM3u` to `Action::ExportM3u(CrateId)`. `export_m3u(id)` loads the crate if needed, proposes "‹crate name›.m3u8" and writes the whole crate. It doesn't show the crate or touch the selection or playback. Unit test: exporting a crate that isn't shown writes its entries and leaves the shown crate as it was. `cargo test -p ui` passes.

## 2. The list menu

- [x] 2.1 In `crate_items`, add a group with Add files… (shortcut text ⌘O, or Ctrl+O off macOS) and Add folder…, behind the same `is_locked` check as Remove selected and Clear crate. Add a group with Export M3U… for every crate. Both go after Sort/Group and before the carts, with separators only between groups that show an item. Headless tests:
  - the empty space's menu offers Add files…, Add folder… and Export M3U…, and no Import M3U… (a real dialog can't open in a headless test);
  - Crate ▸ Add files… on a row is offered;
  - the collection crate's menu has Export M3U… and no Add items (the "No empty band" scenario).

## 3. The crate menu

- [x] 3.1 Add an `export_item` helper. Export M3U… appears in every branch of the crate right-click menu:
  - own, seller, friend and Discogs crates: before Rename crate… or Remove seller…;
  - the Playlist branch: after Clear crate;
  - `label_menu`: last.

  It is disabled for a crate that can't be read. Headless tests:
  - "Friday"'s sidebar menu offers Export M3U… and the shown crate doesn't change (writing a crate that isn't shown is 1.1's test);
  - the Playlist crate's menu offers Clear crate and Export M3U…;
  - the label crate's menu offers Delete label…, Export to crate ▸, Refresh label and Export M3U…, and no Rename or Delete crate….

## 4. The empty crate hint

- [x] 4.1 Add `HintTo::AddFiles`, dispatching `Action::AddFiles`. The tall hint gets the Add files from your computer link before "OR DROP FILES · PRESS H FOR HELP", only in crates that aren't locked. The short hint is unchanged. Update the hint tests:
  - an empty crate you can edit shows the link (clicking it opens a real file dialog, which a headless test can't);
  - label crates show no hint at all, as before, so they show no link.

## 5. The footer and the skin

- [x] 5.1 Remove the `+` button and its popup from the playlist footer.
  - In `skin/generate.rs`, stop drawing `pl_plus` and `pl_plus_p`, and place `pl_opts` at `R::new(11, 12, 18, 18)`.
  - Drop `pl_plus` from the lists in `skin/mod.rs`, and update the `layout.rs` test to check only `pl_opts`.
  - Run `cargo run -p ui --bin skin-gen` to write `assets/skin/default/` again.
  - Update the footer tests ("Three buttons", "Options from the footer") and the test that lists the `+` menu's items.
  - `cargo test -p ui` passes, including the test that the skin matches its generator.
- [x] 5.2 Update the docs:
  - `README.md`: the footer line, the `+ ▸ Import/Export M3U…` line, a line saying Add files… also opens M3U playlists, and the test count.
  - `CHANGELOG.md`: a `### Changed` entry under Unreleased, written with the `changelog` skill, saying where Add files…, Add folder… and Export M3U… went.
  - Check that the help panel names no `+` menu.

## 6. Checks

- [x] 6.1 Run the four checks from AGENTS.md and `cargo xwin check`. All are clean.
- [x] 6.2 (manual) In the app: the footer shows only the gear and the time readout at 275 and at 700 pixels wide. Add files…, Add folder… and Export M3U… work from the list menu. Export M3U… from a crate's sidebar menu writes that crate. The empty crate's link adds files.

## Workflow follow-up

- Archive the change with `/opsx:archive` once it is merged.
