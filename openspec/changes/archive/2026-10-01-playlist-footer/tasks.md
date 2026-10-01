## 1. Sidebar without a switch

- [x] 1.1 Remove the ☰ button, `Action::ToggleSidebar`, `Settings::crate_sidebar` and the `btn_side` sprite; the sidebar shows exactly when it fits (600 px or maximized); tests: a `settings.ron` with `crate_sidebar: false` loads and the sidebar shows when maximized, widening from 400 to 700 px brings it, the existing sidebar tests pass without the toggle

## 2. Footer

- [x] 2.1 Skin: `pl_plus` and `pl_menu` (18 × 18, normal and pressed) and `bpm_clear` (7 × 7) in `generate.rs`, the ADD/REM/SEL/MISC/OPT sprites removed, the layout's `pl_plus`, `pl_menu`, `pl_bpm` and a right-aligned `pl_info`; regenerate with `skin-gen` (its test stays green)
- [x] 2.2 The `+` and `≡` buttons and their menus (as in the design, Show all tempos only while a range is set), the time readout right-aligned beside the grip at any width; headless tests (the footer has only `+` and `≡`, Add folder… and Sort ▸ BPM work from them, Show all tempos appears only while filtered)

## 3. BPM control in the footer

- [x] 3.1 Move the filter from its row to the footer: "BPM" when it fits, a fixed 40 px slider (nearest-handle drag), the range text, × while a range is set, double-click to clear; the list loses the filter row (`pl_bar` and its row in `pl_visible_rows` go); "NO TRACKS MATCH" with its tooltip; headless tests (dragging a handle in the footer sets the range, × and double-click clear it, the first list row is entry 1, the slider is 40 px at 275 and 700 px and stays clear of the time's box; the time fits its LCD box)

## 4. Options menu

- [x] 4.1 A right-click area under the main window's controls and under the maximized strip, opening Options (Double size/Classic size, Spectrogram, Discogs…, Browser…), with its tooltip; Control-click on macOS as for entries; headless tests (right-click on the track info opens it and Spectrogram works from it, right-click on the volume slider doesn't, the strip opens it when maximized)

- [x] 4.2 No crate menu from the title bar while the sidebar shows (the menu stays when narrower); the sidebar's right-click offers Rename crate… and Delete crate… for that crate; headless tests (title click opens nothing beside the sidebar and the menu when narrow, Delete crate… from the sidebar)
- [x] 4.3 Sidebar: Control-click opens a crate's menu on macOS (without showing the crate), and Delete or Backspace after a click on a crate deletes it (asking first when it has entries; never the Playlist crate); a click or cursor move in the list hands Delete back to the entries; headless test
- [x] 4.4 Playlist's sidebar menu: Clear crate (with why it can't be renamed or deleted) instead of the greyed items; `Action::ClearCrate`; headless test
- [x] 4.5 Live playing mark: the player's play/pause sprite on the playing crate only while it plays or is paused, none when stopped (sidebar and crate menu), with a tooltip on every crate; tests updated
- [x] 4.6 The user's Discogs collection crate: a `collection` flag in the crate index (set when the app makes it, when its own collection is listed into a crate, and once for crates named "Collection: …"), pinned at the bottom of the sidebar under DISCOGS in amber with a record icon, last in the crate menu under Discogs; tests (flag saved and migrated, pinned row, the user's crates and + New crate on top)

## 5. Docs and checks

- [x] 5.1 README (the footer, the sidebar without ☰, the BPM control, Options ▸ … everywhere OPT ▸ … was, the test count), help panel
- [x] 5.2 Run the app at 275 px, 700 px and maximized to check the footer and the Options menu; fmt, clippy, the workspace tests, the wasm check, `--startup-time`
