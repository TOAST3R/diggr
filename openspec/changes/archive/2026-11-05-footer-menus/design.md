# Design

## Context

See proposal.md (Why). Today:

- **The footer** is drawn in `crates/ui/src/app.rs`, in the playlist's bottom bar.
  `widgets::button(…, "pl_plus", …)` opens a popup with four items. Import M3U… pushes
  `Action::AddFiles`, the same action as Add files…. Export M3U… pushes `Action::ExportM3u`.
- **`export_m3u()`** always writes `self.crates.shown()`.
- **The list menu** is `crate_items(ui, actions)`. The empty space below the rows and each row's
  Crate ▸ submenu both draw it.
- **The crate's right-click menu** is drawn in one place for the sidebar and the title-bar crate
  menu. It returns early for the Playlist crate (Clear crate and a note) and for label crates
  (`label_menu`).
- **The skin is generated.** `skin/generate.rs` draws the `pl_plus` and `pl_opts` sprites and
  places them at `R::new(11, 12, 18, 18)` and `R::new(32, 12, 18, 18)`. `skin/mod.rs` lists
  them among the required sprites and layouts. `layout.rs` keeps both on the left when the
  playlist widens, and a test checks that. A test also checks that the committed
  `assets/skin/default/` matches the generator's output.
- **The empty crate hint** is built from `HintLine`s whose links carry a `HintTo`. A click is
  dispatched where the list is drawn.

## Goals / Non-Goals

**Goals:**
- Every action the `+` menu offered stays reachable, from the list menu and the crate menu.
- One export routine serves both the list menu (the shown crate) and the crate menu (any crate).

**Non-Goals:**
- No change to the gear, the Options menu, ⌘O or drag-and-drop.
- No change to what M3U import and export read and write (`files::m3u_entries`, `write_m3u`).
- No new keyboard shortcut for export.

## Decisions

**Export takes the crate.** `Action::ExportM3u` becomes `Action::ExportM3u(CrateId)`, and
`export_m3u(id)` calls `crates.load(id)` before reading `crates.get(id)`. Loading reads the
crate's file without showing it, so the shown crate, the selection and playback are untouched.
The list menu passes `shown_id()` and the crate menu passes its own crate. If the crate can't be
read, the item is drawn disabled, as Rename crate… is for an unreadable crate. Alternative: show
the crate, then export it. Rejected, because the spec says exporting from the menu doesn't show
the crate.

**The list menu gains two groups, placed after Sort/Group and before the carts.**
- Add files… and Add folder… go in a group behind the same `is_locked` check as Remove selected
  and Clear crate.
- Export M3U… is in its own group and always shown.

The existing rule that a separator is drawn only between groups that show an item keeps the
collection crate's menu tidy. Add files… shows "⌘O" (Ctrl+O off macOS) as its shortcut text.

**The crate menu gets Export M3U… in every branch.**
- For own, seller and friend crates and the Discogs crates, it goes just before Rename crate…
  (or before Remove seller…).
- In the Playlist branch, it goes after Clear crate.
- In `label_menu`, it is the last item, as the label-crates spec orders it.

A small `export_item(ui, id, readable, actions)` helper keeps the three places the same.

**The hint link is a new `HintTo::AddFiles`.** It dispatches `Action::AddFiles`, which already
adds to the shown crate. The tall hint gets an `Inline` line before the "OR DROP FILES · PRESS H
FOR HELP" line, only when the crate isn't locked. The short hint is unchanged: it has no room,
and it already says "OR DROP FILES".

**The footer loses `+`, and the gear moves into its slot.**
- `generate.rs` stops drawing `pl_plus` and `pl_plus_p` and places `pl_opts` at
  `R::new(11, 12, 18, 18)`.
- `skin/mod.rs` drops `pl_plus` from its sprite and layout lists.
- The `layout.rs` test checks only `pl_opts`.

Then `cargo run -p ui --bin skin-gen` writes the skin again. Alternative: keep the sprite in the
skin and just not draw it. Rejected, because the skin would carry a dead button, and the skin
test would hide that.

**Import M3U… is not replaced by anything.** The file dialog behind Add files… already filters
for audio files and `.m3u`/`.m3u8` together. The help panel and README say Add files… opens
playlists.

## Risks / Trade-offs

- [Users looking for `+` won't find Add files…] → ⌘O and drag-and-drop are unchanged. The empty
  crate shows the link right where a new crate is empty. The changelog's Changed entry says
  where the items went.
- [Export from the crate menu loads a crate that wasn't loaded yet] → Loading is a file read on
  the UI thread, the same as when the crate is clicked, and never touches the audio path. It
  happens only when the user picks the item.
- [A skin that someone else made still has `pl_plus`] → Only the generated default skin
  exists, and the app ignores extra sprites.
