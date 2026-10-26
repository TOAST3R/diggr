## 1. One layout

- [x] 1.1 `app.rs`: the band layout (`maximized_layout`) becomes the only layout, laid out from the window's size whatever the OS state; delete `normal_layout`, the main section, the player-column waveform section with its title bar, the EQ section in the column, the CRATES panel, the playlist's column placement, resize handle and toggle, `Action::ToggleMaximized`, `set_maximized`, `max_seen`, Shift+P, both ⇔ buttons and the playlist's close button
- [x] 1.2 `layout.rs`: remove the player column, window sizing and playlist width/rows from settings; keep the band height and the playlist under it (rename what says "maximized"); tests updated
- [x] 1.3 `apps/native/src/main.rs`: always a maximized first frame; `diggr --startup-time` stays under 300 ms
- [x] 1.4 `settings.rs`: drop `playlist_maximized`, `show_playlist`, `show_crates`, `vis`, `playlist_width`, `playlist_rows`; keep `show_eq` (EQ window open), add the EQ window's position; the old-file test loads a file holding all the dropped fields and `show_eq: true`
- [x] 1.5 `spectrum.rs` and the mini visualizer removed (and its tap reading, if nothing else uses it)

## 2. Mini player and band

- [x] 2.1 Skin generator: 17 × 12 icons (off/on) for shuffle, repeat (repeat one with its small 1), EQ, waveform and spectrum; mini-player layout entries (LCD-row icons at its right end; shuffle at x 174 and repeat at x 193 in the button row; no ⇔); minimize and close for the band's corner; drop sprites only the classic main window used; regenerate with `cargo run -p ui --bin skin-gen`
- [x] 2.2 Mini player: shuffle and repeat icons (lit while on, tooltips), EQ / waveform / spectrum buttons on the LCD row (lit as specified), the title line narrowed to fit before them; headless test that every control of the button row is inside 275 pixels without overlap, and tests for each new button
- [x] 2.3 The verdict flash drawn on the mini player's title line (moved from the main section); headless test with Y
- [x] 2.4 Minimize and close at the band's top-right corner (close quits); the waveform ends before them; dragging the band outside its controls moves a non-maximized window
- [x] 2.5 Seek bar in the band while the waveform is hidden (seeks on release); headless test at 75%

## 3. EQ window

- [x] 3.1 Move the EQ section's drawing into its own viewport (immediate, like Insights; an `Area` where viewports are embedded): the skin's EQ section at its size × scale, title bar drags, × closes, above the app's window; actions fed back each frame; `show_eq` and position remembered
- [x] 3.2 Tests: the EQ button opens and closes it with the settings unchanged, a band change in it reaches the engine, an old `show_eq: true` opens it

## 4. Tests, help and docs

- [x] 4.1 Move or remove every test that draws the classic layout (grep `normal_layout`, `show_playlist`, `show_crates`, `ToggleMaximized`, `mini_`, main-section rects such as `at("seek")`, `at("vis")`, `pl_resize`); keep the behaviour they check where it still exists
- [x] 4.2 `help.rs`: drop Shift+P and the playlist/crates toggles; mention the mini player's buttons where the help names main-window controls
- [x] 4.3 README (layout, mini player, EQ window, keys, settings, test count) and CHANGELOG `## Unreleased` → Changed (changelog skill): what moved where, what's gone
- [x] 4.4 Launch the app: it opens full screen; restoring through the OS keeps the layout; EQ, waveform and spectrum buttons, shuffle, repeat, minimize and close work; note anything off in design.md

## 5. Checks

- [x] 5.1 `grep -rn "normal_layout\|playlist_maximized\|ToggleMaximized\|show_crates\|CratesPanel\|Shift+P" crates apps README.md` finds nothing left over
- [x] 5.2 `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check
