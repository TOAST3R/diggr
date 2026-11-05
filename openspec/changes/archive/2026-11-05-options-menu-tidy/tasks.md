# Tasks

## 1. Size keys

- [x] 1.1 Replace `Action::ToggleScale` with `Action::SetScale(u8)` (clamped to 1–2, saving only when the value changes). Verify with a unit test: SetScale(2) twice saves once and leaves `settings.scale == 2`.
- [x] 1.2 In `window_key`, map Cmd+`Equals` / Cmd+`Plus` to `SetScale(2)` and Cmd+`Minus` / Cmd+`Num0` to `SetScale(1)`. Verify with headless tests: Cmd+= from 1× gives 2×, Cmd+= again stays at 2×, Cmd+− and Cmd+0 give 1×, and playback state is unchanged.
- [x] 1.3 Turn off egui's keyboard zoom (`zoom_with_keyboard = false`) in `DiggrApp::build`. Verify with a headless test: after Cmd+=, `ctx.zoom_factor()` is still 1.0.
- [x] 1.4 Add a "Cmd+= / Cmd+− / Cmd+0" row ("double size / classic size") to the window keys in `help.rs`. Verify with the help test that H lists it.

## 2. Options menu

- [x] 2.1 Remove the size and Spectrogram items from `options_items`. Verify by rewriting the gear test (`app.rs` ~10858) and `a_right_click_on_the_mini_player_opens_the_options`: the menu doesn't show "Double size (2×)", "Classic size (1×)" or "Spectrogram (S)", and in the dig tests it shows Discogs…, Browser… and Set up Diggr….
- [x] 2.2 Without digging, don't open an empty menu: the mini player's right-click opens nothing and has no tooltip, and the gear isn't drawn, though its slot is kept. Verify with a test in the non-dig rig: a right-click on the title line shows no popup, and the footer's other buttons keep their x positions (`the gear comes first` still holds for the layout).
- [x] 2.3 Update `help.rs` "Mini player: right-click" to "Options: Discogs…, Browser…, Set up Diggr…". Verify the help tests pass.

## 3. Docs

- [x] 3.1 In README, update the Options description (around lines 152–154 and 206) and the Spectrogram section (line 1233, drop "or Spectrogram (S) in Options"), list the size keys with the other keys, and update the test count in Tests. Verify with `grep -n "Spectrogram (S)\|Double size" README.md`, which should only find the key and spectrum-button mentions.
- [x] 3.2 Add the entries to `## Unreleased` in CHANGELOG.md with the `changelog` skill (size on Cmd+= / Cmd+− / Cmd+0, a shorter Options menu, no more blurry zoom).

## 4. Checks

- [x] 4.1 Run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, the wasm check for `audio`/`platform` and `cargo xwin check`, all clean.
- [ ] 4.2 Run the app: Cmd+= and Cmd+− switch between 2× and 1× with crisp pixels and no fractional zoom, the gear's menu shows only Discogs…, Browser… and Set up Diggr…, and the spectrum button and S still open the spectrogram.

## Workflow follow-up

- Archive with `/opsx:archive` once merged. That folds the player-window and spectrogram-window deltas into `openspec/specs/`.
