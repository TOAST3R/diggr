## 1. Bar in the frame (empty)

- [x] 1.1 Headless tests: one `LIST_TOP` (title bar + filter bar height) replacing the 12 hard-coded `PL_TOP + 20.0` offsets in `app.rs` test helpers and `record_tests.rs`; suite green before the bar exists
- [x] 1.2 Skin: `pl_filter_{l,fill,r}` sprites (`bar_piece` like the footer) and an inset for the search box in `generate.rs`; `pl_filter_h` in `SkinDef` (serde default 16); `pl_search` / `pl_filters` layout entries; add to `REQUIRED_SPRITES` / `REQUIRED_LAYOUT`; regenerate with `cargo run -p ui --bin skin-gen`; skin tests green (an older skin.ron without `pl_filter_h` still loads)
- [x] 1.3 Layout: `layout.rs` adds `pl_filter_h` to the playlist's chrome height; the playlist draws the bar under `pl_top_*` (normal and maximized layouts) and the list starts under it; headless test: same row count as before at 275 px, order title bar → bar → header → rows

## 2. Search model

- [x] 2.1 `Playlist` transient search (folded words, not in `SavedPlaylist`); `Shown` matches every word against artist, title, record, record artist, label, catno folded with `skin::fold`; `is_filtered`, `clear_filters` include it; cached folded haystack per entry, rebuilt when entries change; unit tests (AND of words, any field, "AME" finds "Âme", local album tag, empty search shows all, missing fields match nothing)
- [x] 2.2 Timing test: setting the search on 5,000 entries < 16 ms in release (best of three), like the existing facet test; play-order test: next follows the search, the hidden playing entry keeps playing

## 3. Search field

- [x] 3.1 Field in the bar's inset: invisible frameless `TextEdit` for input, text and caret drawn in the skin font and LCD colour; takes the room left, at least 60 px; showing another crate clears it; Cmd+F (command modifier) focuses it; Enter plays the first shown entry, ↓ hands the keyboard to the list cursor on the first shown entry, Esc clears and leaves
- [x] 3.2 Headless tests: typing "xcvfni" while playing leaves playback, fullscreen, verdicts and browser untouched and the field reads "XCVFNI"; Esc clears only the search; Cmd+F focuses; switching crates clears; P on a hidden playing entry clears search and filters
- [x] 3.3 Highlight: rows (entry and record rows) tint matched folded char ranges with `pl_current`; test on the glyph runs of a row for "parrish"

## 4. Filters move to the bar

- [x] 4.1 BPM control moves from the footer to the bar after the search field (label "BPM" when room, 40 px slider, range text); its own × goes, the bar's × (shown while search or any filter is set) clears everything; footer back to `+`, `≡`, gear, time; ≡ loses Show all tempos; rewrite `the_footer_bpm_control_sets_and_clears_the_range_without_taking_a_row` and `the_bpm_slider_is_short_and_fixed_and_the_time_stays_in_its_box` for the bar
- [x] 4.2 Filter-control tiers move to the bar (chips + buttons; buttons; else FILTERS ‹n›, lit while set), CART after them when it fits; drop the time-box clearance logic; rewrite `the_footer_shows_chips_then_buttons_then_nothing_as_it_narrows`, `the_style_filter_shows_chips_in_the_discogs_crates_only`, `many_styles_or_a_narrow_playlist_show_the_styles_button_and_its_list`, `a_lit_button_counts_its_picks_and_a_double_click_clears_them`, `a_dig_crate_offers_only_the_format_filter`; check the cart-switch seller tests
- [x] 4.3 One filter panel: tabs STYLE / ARTIST / LABEL / FORMAT (offered ones with ≥ 2 values) reusing the per-facet list content, plus the CART switch in seller crates with a cart; each button opens its tab, FILTERS the first set tab or the first; headless tests (LABELS opens LABEL tab, tab switch keeps picks, 275 px FILTERS → panel with CART)
- [x] 4.4 ☰ loses Filter by style/artist/label/format…, Show all records and Show cart only (keeps Open cart on discogs.com); rewrite `the_menu_opens_any_list_at_classic_width` and the ☰ clicks in `record_tests.rs` / `seller_tests.rs` that used removed items

## 5. Row shortcuts

- [x] 5.1 Entry/record right-click: Only this artist, Only this label, Only this style ▸ where those filters are offered (pick only that value); elsewhere Search ‹label› and Search ‹catno› (set the search, focus the field); headless tests (collection: Only this label lights "LABELS 1"; seller crate: Search Lowtide Tapes)

## 6. Docs and checks

- [x] 6.1 README (filter bar, search and its matching, Cmd+F, the panel, removed ☰ items, test count), help panel (Cmd+F)
- [x] 6.2 fmt, clippy, the workspace tests (`--no-fail-fast`), the wasm check, `--bench` and `--startup-time`; try the bar in the app on a seller crate and the collection crate at classic and wide widths
