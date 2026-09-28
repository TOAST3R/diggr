## 1. Skin

- [ ] 1.1 `skin-gen`: remove the balance sprites and layout entries; add `wave_btn` / `wave_btn_on` at the balance track's rectangle
- [ ] 1.2 `skin-gen`: dimmed title-bar variants for main, EQ and playlist
- [ ] 1.3 `skin-gen`: split `pl_top` / `pl_bottom` into left cap, tileable fill and right cap; regenerate `assets/skin/default/` and update `REQUIRED_SPRITES`; the skin test passes

## 2. Layout and resize

- [ ] 2.1 Settings: add `playlist_width` (default and minimum `pl_width`); remove `balance`; test that an old settings file with `balance` loads
- [ ] 2.2 `window_size` for two columns (width = main + playlist; height = max(left, playlist)); unit tests for all show/hide combinations
- [ ] 2.3 Place the sections: the left column at x = 0, the playlist at x = main width; paint the empty space under the left column; tile the playlist chrome to its width
- [ ] 2.4 The corner handle resizes in 2D (free width, row-step height, clamped to at least the left column's height); clamp the restored width to the monitor
- [ ] 2.5 Remove `Action::Balance` and the startup `set_balance`; wire the waveform button to the `show_waveform` toggle; headless test that a click toggles it and lights the button

## 3. Focus and keys

- [ ] 3.1 `Focus` enum on `App`; primary clicks set it, Tab toggles, player at launch; draw lit or dimmed title bars
- [ ] 3.2 Route ↑/↓: volume when the player has focus or in fullscreen, cursor when the playlist has focus; headless tests for both
- [ ] 3.3 Playlist cursor (`EntryId`): ↑/↓, Shift-extend, PgUp/PgDn, Home/End, Enter; first press lands on the playing entry; the cursor outline; scroll to keep it visible; unit tests in `playlist.rs`
- [ ] 3.4 The cursor survives dig replacement and reordering (test with `replace` and `move_entry`)
- [ ] 3.5 `P` shows the playing entry; follow on track change only if the previous playing entry was visible; tests

## 4. Docs and checks

- [ ] 4.1 Update the shortcuts help panel (`help.rs`) with Tab, P, and the focus-dependent ↑/↓
- [ ] 4.2 Update README (layout, keys, the balance removal, test count)
- [ ] 4.3 Check the launch time (`--startup-time` < 300 ms); run fmt, clippy, the workspace tests and the wasm check
