## 1. One name in the code

- [x] 1.1 `platform`: `APP_DIR` ("Diggr" on macOS and Windows, "diggr" elsewhere) with a unit test; `testing::TestDir` root `<temp>/diggr-tests`; `cargo check -p platform --target wasm32-unknown-unknown`
- [x] 1.2 Folders from `APP_DIR`: `ui::settings` (config), `analysis::cache` (cache), `visuals::library` (visuals), the dig temp fallback in `ui::app::digging`
- [x] 1.3 Env vars to `DIGGR_*`: config and cache dirs, frame stats, auto fullscreen, auto quit, visual bench, the Discogs token in `dig` tests
- [x] 1.4 Package `diggr` (`apps/native/Cargo.toml`, `Cargo.lock`), usage text and module docs, window title "Diggr", eframe app id "diggr"; the app type becomes `DiggrApp` across `ui` and `apps/native`
- [x] 1.5 `dig::APP_NAME = "Diggr"`; User-Agent `Diggr/‹version› +https://github.com/TOAST3R/diggr` and its test; bridge tests
- [x] 1.6 Extension: manifest name "Diggr bridge", short name and toolbar title "Diggr"; `PLACEHOLDER = "Diggr"`
- [x] 1.7 Comments that name the old product: describe the look ("classic 2000s desktop player") or the behaviour without the comparison (`ui` skin, app, columns, format, lib; `audio::eq`; `apps/native` interactive and main)

## 2. Docs and specs

- [x] 2.1 README: title, intro, commands (`-p diggr`, `diggr …`), folders table, env vars, "‹App›" → Diggr (the placeholder paragraph goes), test temp folder, roadmap wording; AGENTS.md likewise
- [x] 2.2 `openspec/config.yaml` product line; living specs not covered by this change's deltas (none expected after the deltas: check)
- [x] 2.3 `macos-release` (proposal, design, tasks, spec): `Diggr.app`, `io.github.toast3r.diggr`, `diggr://`, the `diggr` binary, no "until the rebrand"
- [x] 2.4 Archived changes: replace every occurrence (binary, folders, env vars, type, descriptions), meaning unchanged

## 3. Keep it clean

- [x] 3.1 `apps/native/tests/no_old_name.rs`: `git ls-files`, every text file, case-insensitive, the name built from two halves; fails with file:line; skips without git
- [x] 3.2 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo check -p audio -p platform --target wasm32-unknown-unknown`; README test count
- [ ] 3.3 By hand after merge: move the two folders, reload the extension, start `cargo run --release -p diggr` and check crates, token, sellers and the extension pairing are all there; then rename the checkout and its Claude memory folder
