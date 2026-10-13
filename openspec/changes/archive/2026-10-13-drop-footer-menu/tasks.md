## 1. Footer

- [x] 1.1 Drop the `≡` button from the footer; move Export M3U… into the `+` popup under Import M3U…; `+` hover text "Add · M3U"
- [x] 1.2 Skin: drop `pl_menu` (rect and sprites) in `skin/generate.rs` and `skin/mod.rs`, move `pl_opts` to x=32, run `cargo run -p ui --bin skin-gen`; update `layout.rs`'s wide-playlist test

## 2. List menu

- [x] 2.1 Move the `≡` popup's body into `crate_items`, drawing a separator only before a group that has an item
- [x] 2.2 `entry_menu` ends with a separator and "Crate ▸" holding `crate_items`
- [x] 2.3 A right-click (or Control-click) on the list's empty space, or anywhere in an empty crate's list, opens `crate_items`
- [x] 2.4 Headless tests: the footer shows no `≡` and `+` holds both M3U items; Crate ▸ Sort ▸ BPM sorts a grouped crate; a right-click below the rows inverts the selection; the collection crate shows no Remove/Clear and no double separator; move `record_tests.rs` and `seller_tests.rs` (Open cart) off `pl_menu`

## 3. Docs and checks

- [x] 3.1 README: the footer, Sort, Group by record, M3U and Open cart sentences; the test count
- [x] 3.2 Run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo check -p audio -p platform --target wasm32-unknown-unknown`
