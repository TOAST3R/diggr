## 1. Structured status

- [x] 1.1 Add `WaitKind` / `UnavailableKind` to `EntryStatus`, with `note()` rendering the same wording; unit tests
- [x] 1.2 Saved-crate format: store each kind's wording and read it back to its kind (`Other` for unknown text); a test with an old crate file
- [x] 1.3 Replace the string statuses in `app/digging.rs` with kinds; the `dig` crate's text reasons are converted where they enter the UI; the existing dig tests pass

## 2. Icons

- [x] 2.1 `skin-gen`: listed, queued, needs yt-dlp, unavailable and other icons; regenerate the skin; the skin test passes
- [x] 2.2 `row_look` draws the icon (or the download bar) in the duration slot; headless test: 40% shows a bar and no text

## 3. Tooltip

- [x] 3.1 A pure `entry_details(entry, marks, now) -> Vec<(label, value)>` with unit tests (Discogs, local, unknown fields left out, "fetched 3 h ago")
- [x] 3.2 Show it with `on_hover_ui` on each row

## 4. Context menu

- [x] 4.1 Add Play/Arm and Remove; the selection rules (inside vs outside the selection); headless tests for both removal cases
- [x] 4.2 Add Open release on Discogs and Copy Discogs link (release, or master for a master-only origin); hide them without an origin; unit test for the URLs

## 5. Docs and checks

- [x] 5.1 Shortcuts help: an icon legend and the menu items; README and test count
- [x] 5.2 Run fmt, clippy, the workspace tests and the wasm check
