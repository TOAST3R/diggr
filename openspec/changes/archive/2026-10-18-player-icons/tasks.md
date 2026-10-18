## 1. Remove Eject and the channel indicators

- [x] 1.1 Remove `Action::Eject`, `Open::Replace` and the replace branch of `add_paths` in `crates/ui/src/app.rs`; keep the command-line `replace_playlist` path; update the test that opened with `Open::Replace` to cover the command line only, and add one checking Cmd+O (`Open::Add`) on the shown Playlist crate adds
- [x] 1.2 Stop drawing `mono`/`stereo` and the `eject` button in the main section
- [x] 1.3 In `skin/generate.rs`, drop the `eject`, `mono*`, `stereo*` sprites and layout rects

## 2. Icons in the skin

- [x] 2.1 Draw wave, EQ and playlist toggle icons (`tog_wave`, `tog_eq`, `tog_pl`, on/off) as pixel art on `button_face`, about 17×12, lit in amber
- [x] 2.2 Draw shuffle (on/off, pressed) and repeat (off/all/one, pressed; one with a small 1) icons, about 22×15
- [x] 2.3 Add a 12 px `sync` toggle-row sprite set (off/on, pressed) from `text_button`, still labelled SYNC
- [x] 2.4 Move the layout rects: toggles and SYNC in one row across 177..268 at y=58; shuffle and repeat right-aligned to 268 in the transport row
- [x] 2.5 Run `cargo run -p ui --bin skin-gen` and commit the regenerated `assets/skin/default/`

## 3. Wire the main section

- [x] 3.1 Draw SYNC in the toggle row and shuffle/repeat at their new rects; add tooltips naming each icon (repeat's says off, all or one)
- [x] 3.2 Check by eye at 1× and 2× (`cargo run --release -p diggr`) that the icons read and nothing overlaps

## 4. Docs and checks

- [x] 4.1 README: remove the Eject mentions (Playlist crate, double-click retry), say opening files always adds, and update the test count
- [x] 4.2 Run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check` and the wasm check for `audio`/`platform`
