# Tasks

## 1. Skin

- [x] 1.1 Check how a skin file with extra or missing sprite and layout names loads. Answer in design.md's footer decision. Verify: `a_skin_file_from_before_the_footer_went_still_loads` loads a skin that still carries `pl_opts`, `pl_info` and `pl_bottom_h: 38`.
- [x] 1.2 In `skin/generate.rs`, add `btn_opts*` (9×9 gear), `tog_group_*` (17×12 ▤) and `pl_bottom_edge` (1×12); drop the footer's sprites, `btn_group*`, `tog_wave_*`, `seek_*` and the `pl_group`, `pl_opts`, `pl_info` layout entries and `pl_bottom_h`; widen the volume to 108. Verify: `cargo run -p ui --bin skin-gen` regenerates `assets/skin/default/` and the skin-on-disk test passes.

## 2. Band

- [x] 2.1 Mini player: ▤, EQ, spectrum and SYNC end the LCD row at 4 px from the edge; volume (108), shuffle and repeat end the button row at the same edge on one centre line; no waveform button. Verify: `the_buttons_fit_in_the_mini_player` (gaps, right edges, centre line, 17 title characters) and the verdict-flash test.
- [x] 2.2 In `window_buttons`, `CORNER_W` is 39: minimize (x 4), the gear (x 15, only with `has_options()`) and close (x 26), each taking clicks only inside its own square. Verify: `the_band_has_minimize_and_close` (gaps do nothing, close quits, corner drag starts a window drag) and `the_corner_gear_opens_the_discogs_dialog`.
- [x] 2.3 The waveform is always shown: remove `show_waveform`, `ToggleWaveform`, the `W` key (and its help entry) and the band's seek bar. Verify: `the_waveform_is_always_in_the_band`, and an old `settings.ron` with `show_waveform: false` loads.

## 3. Playlist

- [x] 3.1 Remove ▤ from the title bar and the footer from `playlist_section`; draw the `pl_bottom_edge` under the last row; `playlist_chrome` is title + filter + edge. Verify: layout tests (25 rows at 720 × 435) and `the_playlist_has_no_footer_and_the_list_has_the_crate_menu`.
- [x] 3.2 Draw the "selected/total" readout in a sunken box right after the search field, within `filter_bar`'s width budget. Verify: `the_time_readout_follows_the_search_field_and_counts_the_whole_crate` and `the_time_readout_gives_way_first_as_the_bar_narrows` (width sweep 275 → 1,000).

## 4. Grouping

- [x] 4.1 `CrateInfo::is_grouped` is `grouped.unwrap_or(true)`, and a new crate's list opens grouped. Verify: `grouping_is_remembered_and_every_crate_defaults_to_it` and `grouping_is_remembered_per_crate`.
- [x] 4.2 Show the column header for a grouped crate with no album. Verify: `a_grouped_crate_without_albums_keeps_its_column_header` and `has_albums_agrees_with_the_album_keys`.

## 5. Docs and checks

- [x] 5.1 Fix the Purpose lines of `openspec/specs/playlist-filters/spec.md` and `openspec/specs/style-filter/spec.md`, which say "footer", to say the filter bar. Verify: `grep -n footer openspec/specs/*/spec.md` shows only text this change's deltas replace.
- [ ] 5.2 Update README (the layout tour, keys without W, the test count) and add CHANGELOG `## Unreleased` entries with the `changelog` skill. Verify: both diffs mention each moved control, the waveform, and the grouping default.
- [ ] 5.3 Run the four checks from AGENTS.md (`cargo test --workspace`, clippy `-D warnings`, `fmt --check`, the wasm check of `audio` and `platform`) and `openspec validate ui-tidy --strict`. Verify: all clean.
- [ ] 5.4 Look at the real app (`run` skill): the band's corner, both rows of the mini player, the filter bar at 275 px and full width, the bottom edge. Verify: screenshots show no overlap and no footer.

## Workflow follow-up

- Archive with `/opsx:archive` once applied, which folds the deltas into `openspec/specs/`.
