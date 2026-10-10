## Why

A crate's filters are spread over three places: the BPM range, style chips, ARTISTS/LABELS/FORMATS buttons and the CART switch crowd the playlist footer, narrow playlists move them into ☰ (Filter by…, Show all tempos, Show all records, Show cart only), and there is no way to find a record by name at all. A digger in a 400-record seller crate who wants "that Parrish twelve" or "everything on Lowtide Tapes" has to scroll. One always-visible bar at the top of the list, with a live text search and every filter in it, makes narrowing a crate one place to look, at every width, and gives the footer back its classic look.

## What Changes

- **New filter bar**: a strip of skin chrome between the playlist title bar and the list (above the column header), always shown, in every crate (Playlist, crates, LABELS, TOP SELLERS, wantlist, collection). It is part of the frame: the window grows by its height, the list keeps its rows.
- **Live search** in the bar: typed words (all of them, any order, case and accents ignored) are matched against each entry's artist, title, record, record artist, label and catalogue number; the list, play order, previews ahead and the title bar's shown/all count follow on every keystroke. Matched text is highlighted in the rows. Cmd+F (Ctrl+F off macOS) focuses it; Esc clears the search; typing in it never triggers the letter shortcuts. The search is **not** remembered: switching crates or restarting clears it.
- **Filters move into the bar** (**BREAKING** for the UI layout): the BPM range, the style chips or STYLES button, ARTISTS, LABELS, FORMATS and the CART switch leave the footer for the bar, with the same width tiers; at the narrowest width they fold into one FILTERS button showing how many filters are set. A × at the bar's end clears the search and every filter.
- **One filter panel**: the style, artist, label and format lists (and the cart switch) become tabs of one panel, opened on the matching tab by each button, or on the first by FILTERS.
- **Menus slimmed**: ☰ loses Filter by style/artist/label/format…, Show all tempos, Show all records and Show cart only (the bar does all of it). ☰ keeps crate actions, including Open cart on discogs.com.
- **Entry right-click** gains "Only this artist", "Only this label" and "Only this style" where those filters are offered, which pick that value in its filter; in other crates it offers "Search ‹label›" / "Search ‹catno›", which put the text in the search.
- The footer goes back to `+`, `≡`, the gear and the time readout.

What a filter does to the list (what plays next, shuffle, previews ahead, P, record rows, counts, memory per crate) is unchanged.

## Capabilities

### New Capabilities
- `crate-search`: the live text search in the filter bar: matching, highlighting, focus and keys, not remembered, and how it combines with the filters.
- `filter-bar`: the bar itself: its place in the frame, what it holds at each width, the FILTERS button and tabbed panel, the clear-all ×, Cmd+F, and the entry right-click shortcuts.

### Modified Capabilities
- `playlist-filters`: the BPM control moves from the footer into the filter bar; "no row taken" and "clear of the time readout" are replaced; ≡ ▸ Show all tempos goes away.
- `style-filter`: the chips or STYLES button live in the filter bar; ☰ ▸ Filter by style… and Show all records go away; the list is the panel's Style tab.
- `record-filters`: "Filter controls" moves to the filter bar with a FILTERS fallback instead of ☰ items; "Filters combine" adds the search; the lists are tabs of one panel.
- `discogs-cart`: the CART switch is in the filter bar; ☰ ▸ Show cart only goes away.
- `playlist`: the footer holds `+`, `≡`, the gear and the time readout only; ≡ loses Show all tempos.
- `classic-skin`: the LCD-colour rule names the filter bar's controls, not the footer's.

## Impact

- **`crates/ui/src/skin/generate.rs`, `skin/mod.rs`, `assets/skin/default/`**: new `pl_filter_{l,fill,r}` sprites and layout entries, `pl_filter_h`; skin regenerated with `skin-gen`.
- **`crates/ui/src/layout.rs`**: playlist chrome height includes the bar.
- **`crates/ui/src/playlist.rs`**: `Shown` gains the search; a folded-text match per entry.
- **`crates/ui/src/app.rs`**: filter bar drawing and input, footer simplified, ☰ items removed, entry menu items, Cmd+F, row highlighting, one tabbed filter panel replacing the per-facet lists.
- **Tests**: headless helpers' list/footer offsets move by the bar's height; ~10 footer and ☰ filter tests rewritten for the bar; new search tests. Play-order tests are untouched.
- **Docs**: README (filter bar, Cmd+F, test count), help (H) keys.
- No change to audio, analysis, visuals, dig or the browser bridge. No new dependency.
