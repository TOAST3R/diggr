# Proposal

## Why

The playlist footer's `+` menu is a fourth place to add music: ⌘O, drag-and-drop and Play on an
empty crate already open files, and its Import M3U… does exactly what Add files… does. Only
Export M3U… has no other home. Fewer menus make the app simpler to learn. The crate's actions
already have a home: the list menu, and the crate's own right-click menu.

## What Changes

- **BREAKING (UI):** the playlist footer loses its `+` button and menu. It keeps the gear and the
  time readout. The gear moves into the first slot.
- The list menu (a right-click on the list's empty space, or Crate ▸ on a row's menu) gains:
  - Add files… and Add folder…, in crates the user can edit;
  - Export M3U…, in every crate.
- The crate's right-click menu, in the sidebar or the title-bar crate menu, gains Export M3U….
  It writes that crate, which need not be the shown one. This applies to every kind of crate,
  the Playlist and label crates included.
- Import M3U… goes away as a separate item. Add files… already opens `.m3u` and `.m3u8`
  playlists, and dropping one still imports it.
- The empty crate hint gains an **Add files from your computer** link, in crates the user can
  edit. It does what Add files… does.
- The skin generator no longer draws the `+` button. The skin is generated again.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `playlist`: "Playlist footer" (no `+`; gear and time only), "List menu" (Add files…,
  Add folder…, Export M3U…), "Empty crate hint" (the Add files from your computer link).
- `crates`: "The Playlist crate" (how music is added, without ADD), and a new requirement that
  every crate's right-click menu offers Export M3U….
- `label-crates`: "Label crate menu" (its exact item list gains Export M3U…).
- `player-window`: "Transport and sliders" (the No Eject scenario names the list menu, not the
  footer's ADD).

## Impact

- `crates/ui/src/app.rs`:
  - the footer (`pl_plus` and its popup);
  - `crate_items` (the list menu) and the crate right-click menu;
  - `label_menu`;
  - `export_m3u`, which takes the crate to write;
  - the empty crate hint and its link list.
- `crates/ui/src/skin/generate.rs`, `crates/ui/src/skin/mod.rs`, `crates/ui/src/layout.rs`, and
  the generated `assets/skin/default/`, via `cargo run -p ui --bin skin-gen`.
- Headless UI tests for the footer, the list menu, the crate menus and the hint.
- `README.md`: the footer and M3U lines, and the test count. `CHANGELOG.md` gets a Changed
  entry.
- No change to audio, analysis or dig. Nothing on the playback path.
