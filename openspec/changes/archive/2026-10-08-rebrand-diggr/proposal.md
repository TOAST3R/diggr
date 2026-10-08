## Why

The app is called **Diggr**, but the code, folders, binary and window still carry the working name, and the window title even reads the name of another product, which is a trademark risk. The macOS release is next (`macos-release`), and its bundle id, URL scheme and DMG name become permanent once installed, so the rename has to land first. Today the app is installed on one machine only, so nobody else's data needs moving.

## What Changes

- **Name everywhere a user sees it:** the window title, the name the player reports to the browser extension (`dig::APP_NAME`), the extension's manifest and placeholder, the help panel, README and AGENTS.md all say **Diggr**. The README's "‹App›" placeholder becomes Diggr.
- **BREAKING: folders.** Config and cache live in a `Diggr` folder (`~/Library/Application Support/Diggr`, `~/Library/Caches/Diggr`; `diggr` on Linux), named in one place in `platform`. The app does not look for the old folders: the one existing install is moved by hand (see design).
- **BREAKING: env vars.** Every env var takes the `DIGGR_` prefix (config and cache dirs, Discogs token for tests, frame stats, auto fullscreen, auto quit, visual bench).
- **BREAKING: binary.** The desktop package and binary become `diggr` (`cargo run -p diggr`, `diggr --render-show …`), and the app type becomes `DiggrApp`.
- **Outward names:** the Discogs User-Agent becomes `Diggr/‹version› +https://github.com/TOAST3R/diggr` (the GitHub repo is already renamed); temp folders become `diggr-tests`.
- **No reference to the old name anywhere in the repo:** code, comments, docs, living specs, the active `macos-release` change and the archived changes. Descriptions of the look say "a classic 2000s desktop player" instead. A test keeps it that way. Git history is left as it is.
- **`macos-release`:** its "until the rebrand" caveats go: `Diggr.app`, `io.github.toast3r.diggr`, `diggr://`.

## Capabilities

### New Capabilities

- `app-identity`: the product name, the folder names, the env var prefix, the binary name, what the extension and Discogs are told, and the rule that the repo holds no other product's name.

### Modified Capabilities

- `show-render`: "Render a track's show to video" names the `diggr` binary.
- `classic-skin`: "Original default skin" says no third-party skin art, without naming a product.
- `streaming-analyzer`: "Cache" names `DIGGR_CACHE_DIR`.

## Impact

- **`crates/platform`:** the app folder name constant; `testing::TestDir` root.
- **`crates/ui`:** `DiggrApp`, env vars, the config path, comments; `settings.rs`, `app.rs`, `app/*.rs`, `skin/`, `columns.rs`, `format.rs`, `lib.rs`.
- **`crates/analysis`, `crates/visuals`, `crates/audio`, `crates/dig`:** cache and visuals folders, env vars, `APP_NAME`, User-Agent, comments, tests reading `DIGGR_DISCOGS_TOKEN`.
- **`apps/native`:** package `diggr`, window title, eframe app id, usage text, comments. `Cargo.lock`.
- **`extensions/chrome`:** manifest name, short name, title; the placeholder.
- **Docs:** README, AGENTS.md, help panel, `openspec/config.yaml`, specs, `macos-release`, archived changes.
- **Your machine (by hand, once):** move the two folders, reload the extension, then rename the local checkout and its Claude memory folder.
- No new dependency; nothing on the playback path changes.
