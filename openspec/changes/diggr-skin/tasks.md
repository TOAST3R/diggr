## 1. LCD colour from the skin

- [x] 1.1 `skin::Colors` gains `lcd` with `#[serde(default = "lcd_green")]` (0,236,0); unit test: a skin RON without `lcd` loads with green
- [x] 1.2 `app.rs`: the six `color([0, 236, 0])` and the two derived dims read `colors.lcd` (title line, kbps/kHz, BPM control, filter controls, cart switch, footer info); the app still draws green with the current skin (no visible change yet)

## 2. Default skin palette and title bars

- [x] 2.1 `generate.rs`: replace the palette constants with the graphite/amber values in design §1 (panels, bevels, title bar, LCD on/dim, accent, buttons, EQ knobs silver with an amber tick); `Colors` for the default skin: `lcd`, `pl_text`, visualizer, `eq_curve`, `pl_owned` off-white
- [x] 2.2 `titlebar` and the playlist title pieces draw groove lines (design §3) instead of stripes; caption in amber; the playlist caps and tiled middle line up at any width
- [ ] 2.3 Crate-label sprites `pl_label_l`, `pl_label_fill`, `pl_label_r` in the atlas and `skin.ron`; `pl_title` becomes dark ink (30,26,20)
- [x] 2.4 `cargo run -p ui --bin skin-gen`; `committed_assets_match_the_generator` passes

## 3. Crate label

- [ ] 3.1 `draw_crate_name` draws left cap, stretched fill and right cap around the (cut) name, centred in the bar; falls back to `pl_title_fill` when the skin has no label sprites; unit test that a skin without the label sprites still loads and that the label width follows the cut text

## 4. Idle line and verdict flash

- [ ] 4.1 `idle_line()` ("PASTE A DISCOGS LINK · CMD+V · OR DROP FILES", CTRL+V off macOS) used by `engine_status` for a ready device; unit test (both platforms' text via a helper taking the modifier name; opening and error states unchanged)
- [ ] 4.2 `flash_text(verdict, count)` (WANTED, UNWANTED, PASS, OWNED; count appended above 1) with unit tests; `flash: Option<(String, Instant)>` on the app; the title line shows it centred, unscrolled, for 1.5 s, then the normal line
- [ ] 4.3 `dig_want`, `dig_unwant`, `dig_pass`, `dig_collect` set the flash only when they changed something; headless dig tests: Y on a new record → "WANTED"; N on a wanted record → no flash; want 3 selected → "WANTED 3"; N while playing → "PASS" and the next track starts

## 5. Tune and document

- [ ] 5.1 Run the app and check the main, EQ and playlist windows at 1× and 2×: amber legibility in the playlist, the OWNED badge next to the current track, groove lines on a stretched playlist; adjust the palette values in `generate.rs` and regenerate; save before/after screenshots for the PR
- [ ] 5.2 README (the player's look, idle line, verdict flash, test count) and the help panel if it describes the look; run `cargo test --workspace`, clippy, fmt and the wasm check
