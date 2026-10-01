## Why

The last change left the playlist busier than it needs to be. The sidebar has an on/off button nobody needs, the BPM filter takes a whole row of the list for one small slider, and the footer has five text buttons (ADD, REM, SEL, MISC, OPT). Most of their items are available elsewhere, MISC exists for one real item, and OPT mixes app settings with playlist actions.

## What Changes

- **Crate sidebar:** the ☰ button and its remembered setting go, and while the sidebar shows, the title bar no longer opens the crate menu (the sidebar's right-click renames and deletes). Narrower than 600 px, the title bar's menu stays, since nothing else switches crates there. The sidebar shows whenever the playlist is at least 600 px wide or maximized, and hides below that, where the title-bar crate menu still works. **BREAKING** for `settings.ron`: the `crate_sidebar` key is ignored.
- **BPM filter in the footer:** the row above the list goes. A compact control in the footer takes its place: `BPM ◂━●━━●━▸ 124-139 ×`, where × (shown only while a range is set) clears it. The title bar keeps `· 42/301`, and the list gets its row back.
- **Two footer buttons instead of five:**
  - **`+`**: Add files…, Add folder…, Import M3U…
  - **`≡`**: Select all / none / invert, Remove selected, Clear crate, Sort ▸, Show all tempos (when a range is set), Export M3U…
- **Options menu:** the app settings that were in OPT move to a right-click menu on the main window (and on the maximized strip): Double size, Spectrogram, Discogs…, Browser…. Every "OPT ▸ …" in the specs becomes "Options ▸ …".

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `playlist`: the footer's two buttons and their menus.
- `playlist-filters`: the BPM filter becomes a footer control.
- `crates`: the sidebar no longer has an on/off button.
- `player-window`: the Options menu.
- `playlist-columns`: sorting from ≡ ▸ Sort instead of OPT ▸ Sort.
- `spectrogram-window`, `discogs-intake`, `discogs-collection`, `preview-fetch`, `browser-bridge`: their dialogs are reached from Options instead of OPT.

## Impact

- `crates/ui`:
  - `app.rs`: the footer (buttons, menus, the BPM control, the time readout), the Options menu on the main window and the strip, and removing the filter row, the ☰ button and `Action::ToggleSidebar`;
  - `settings.rs`: `crate_sidebar` removed (old files still load);
  - `skin/generate.rs`: `+` and `≡` buttons in place of ADD/REM/SEL/MISC/OPT, and no sidebar button; the skin is regenerated with `skin-gen`;
  - `help.rs`.
- README (the footer, Options ▸ … everywhere OPT ▸ … appears, the test count).
- No change to audio, digging or what the filter does; only where its controls live.
