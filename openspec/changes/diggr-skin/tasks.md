## 1. LCD colour from the skin

- [x] 1.1 `skin::Colors` gains `lcd` with `#[serde(default = "lcd_green")]` (0,236,0); unit test: a skin RON without `lcd` loads with green
- [x] 1.2 `app.rs`: the six `color([0, 236, 0])` and the two derived dims read `colors.lcd` (title line, kbps/kHz, BPM control, filter controls, cart switch, footer info); the app still draws green with the current skin (no visible change yet)

## 2. Default skin palette and title bars

- [x] 2.1 `generate.rs`: replace the palette constants with the graphite/amber values in design §1 (panels, bevels, title bar, LCD on/dim, accent, buttons, EQ knobs silver with an amber tick); `Colors` for the default skin: `lcd`, `pl_text`, visualizer, `eq_curve`, `pl_owned` off-white
- [x] 2.2 `titlebar` and the playlist title pieces draw groove lines (design §3) instead of stripes; caption in amber; the playlist caps and tiled middle line up at any width
- [x] 2.3 `cargo run -p ui --bin skin-gen`; `committed_assets_match_the_generator` passes

## 3. Empty crate hint and waveform title bar

- [x] 3.1 `empty_crate_hint(modifier)` drawn centred in an empty, non-Discogs crate's list (skin font, `pl_text`), CMD on macOS and CTRL elsewhere; unit test (wording, every glyph in the font, fits the narrowest list)
- [x] 3.2 `wave_title` sprite and `wave_titlebar` / `wave_close` layout in `generate.rs`; `layout::wave_title_h` counted in `player_height`; `wave_title` draws, dims, drags and closes; layout test updated (player column 304 with waveform and EQ)

## 4. Verdict flash

- [x] 4.1 `flash_text(verdict, count)` (WANTED, UNWANTED, PASS, OWNED; count appended above 1) with unit tests; `flash: Option<(String, Instant)>` on the app; the title line shows it centred, unscrolled, for 1.5 s, then the normal line
- [x] 4.2 `dig_want`, `dig_unwant` and `dig_pass` set the flash only when they changed something, and `dig_collected` on a successful add; headless dig tests: Y → "WANTED", Y again → "UNWANTED"; N while playing → "PASS" and the next track starts; N on a wanted record and Y on an owned one → no flash; Add to collection → "OWNED" (the count is covered by the `flash_text` unit test)

## 5. Tune and document

- [x] 5.1 Run the app and check the main, EQ and playlist windows at 1× and 2×: amber legibility in the playlist, the OWNED badge next to the current track, groove lines and the crate name on a stretched playlist; adjust the palette values in `generate.rs` and regenerate; save before/after screenshots for the PR
- [x] 5.2 README (the player's look, empty crate hint, waveform title bar, verdict flash, test count) and the help panel if it describes the look; run `cargo test --workspace`, clippy, fmt and the wasm check
