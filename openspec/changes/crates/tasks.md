## 1. Entry model (`crates/ui/src/playlist.rs`, `files.rs`)

- [ ] 1.1 Add `source`, `Origin` and the `Waiting`/`Unavailable` statuses to `Entry` and `SavedEntry`, all with serde defaults; verify with a unit test that a `playlist.ron` written by the current format still loads unchanged
- [ ] 1.2 Add the producer API (`add_waiting`, `set_status`, `set_audio`, `set_unavailable`) and make `set_info` update only the duration of an entry with an origin; verify with unit tests that tags never overwrite an origin's artist and title, and that `set_audio` turns a waiting entry Pending
- [ ] 1.3 Build `play_order` over playable and waiting entries, and the engine queue from its playable ones; verify with unit tests that waiting and unavailable entries are skipped by next, previous, shuffle and repeat, that an entry joins the queue after `set_audio`, and that its place in a shuffled order doesn't change when it does
- [ ] 1.4 Make M3U export write an entry's source URL whenever it has one (even after its audio arrived) and leave unavailable entries out; verify with a unit test (and that the existing local-file round trip still passes)

## 2. Crate store (`crates/ui/src/crates.rs`)

- [ ] 2.1 Implement `Crates`: the index, lazy loading, shown and playing ids, and create/rename/delete with the name rules (1–40 characters, unique ignoring case; Playlist can't be renamed or deleted); verify with unit tests in temp dirs
- [ ] 2.2 Save each crate atomically within 2 s of a change and on quit, and handle an unreadable crate file (listed, not loaded, one message) and a damaged index (rebuilt from the crate files); verify with temp-dir tests, and add `crates/` to the README's "Where files are kept" table
- [ ] 2.3 Migrate `playlist.ron` into the Playlist crate on first launch, leaving the file untouched; verify with a 300-entry fixture test (same entries, order and current entry; old file byte-identical)
- [ ] 2.4 Implement Send to crate: copy in order with fresh ids, skip duplicates by origin clip or file path, and leave the source unchanged; verify with unit tests

## 3. Player wiring (`crates/ui/src/app.rs`)

- [ ] 3.1 Make the window edit the shown crate and build the engine queue from the playing crate; starting a track makes its crate the playing one, and deleting the playing crate stops playback; verify with headless tests that switching crates leaves the engine queue untouched and that starting a track in another crate re-targets next and previous
- [ ] 3.2 Arm a waiting entry on double-click: show "Waiting for ‹title›" with its status, start it within 100 ms of `set_audio`, and cancel when another track starts; verify with a test that copies a fixture file into place mid-playback against `ManualSink`
- [ ] 3.3 Verify skipping is gapless: in a `crates/ui/tests/` test against `ManualSink`, track 3 is followed by track 5 without a gap while track 4 is waiting, and track 4 is not marked failed
- [ ] 3.4 Make Eject and command-line files replace only the Playlist crate (shown, played); verify with a test that another crate is unchanged, and describe the Playlist crate in the README
- [ ] 3.5 Draw waiting and unavailable rows dimmed with their status text, keep failed rows red, and make the title line use an origin entry's own artist and title; verify with headless egui tests

## 4. Title bar and menus (skin generator, `app.rs`)

- [ ] 4.1 Make the skin generator draw a caption-less `pl_top` plus the `pl_title_fill` strip, then regenerate the committed skin; verify with the skin tests (committed skin matches the generator)
- [ ] 4.2 Draw the crate name in the title bar (folded to uppercase, cut to fit); a click opens the crate menu and a drag moves the window; verify with headless egui click and drag tests
- [ ] 4.3 Build the crate menu (switch, New crate…, Rename crate…, Delete crate… with confirmation) on the preset name dialog, and add Send to crate to the entry menu; verify with headless tests, and document the menu and Send to crate in the README and in the help panel's mouse section

## 5. Integration checks

- [ ] 5.1 Measure `winamp-native --startup-time` with 50 crates of 500 entries and confirm it stays under 300 ms; record the numbers in the README's launch note
- [ ] 5.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check of `audio` and `platform`; confirm the README's test count matches
