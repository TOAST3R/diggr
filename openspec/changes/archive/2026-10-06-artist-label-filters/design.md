## Context

The style filter (`style-filter`, archived in `discogs-crates-cleanup`) put a second predicate into `Playlist::shows` through a `Shown` snapshot, so every consumer follows it: the list, the `Rows` cache, play order, previews ahead, selection, the cursor, the title bar count and `P`. It keeps `styles_on: BTreeSet<String>`, saved in `SavedPlaylist.styles`, and `Playlist::styles()` counts records per style, cached in the app by crate and revision. The footer draws it through `style_control`, after `bpm_control` returns where it ended.

What artists and labels need, from the code:
- `Entry.artist` is the **track's** credit when the tracklist names one, else the release's (`matching.rs`). Nothing on the entry holds the record's own credit, so a compilation's tracks all differ.
- `Origin.label` is the release's first label (`first_label`), the same on every track of a record.
- `RecordInfo.artist` is the release's (or master's, or listing item's) credited artist, joined as Discogs shows it (`join_artists`). It is already in hand where `origin()` builds an entry's origin, and in the cache-only `Command::Backfill` answer.
- Footer room at classic width (275 px): the BPM control's core is 94 skin pixels, exactly the room between `pl_bpm` (x 77) and the time box (x 171). STYLES, ARTISTS and LABELS together need about 125 px, so they fit from about 400 px wide, or when the crate has no tempos.

## Goals / Non-Goals

**Goals:**
- Artist and label filters that reuse the style filter's model, matching, persistence and list UI, with no new path for play or display.
- One rule for which controls the footer shows, and a way to reach every filter at any width.
- The record's credited artist on every Discogs entry, including older crates, without requests.

**Non-Goals:**
- Splitting "A & B" into names, a track's own artists, labels after the first.
- These filters outside the wantlist and collection crates.
- Year, format or for-sale filters (the model would take them later).

## Decisions

### 1. One facet model instead of three copies

`Playlist` replaces `styles_on` with `picked: [BTreeSet<String>; 3]`, indexed by a `Facet` enum (`Style`, `Artist`, `Label`). Per facet:
- `values(e)`: an iterator of the entry's values (style: its comma-separated styles; artist: `origin.artist`; label: `origin.label`; empty ones skipped);
- `filter(facet)`: the picked set when some entry still has one of its values, else `None` (so a value gone from the crate stops filtering, as for styles);
- `counts(facet)`: records per value, the most first, then by name (the existing `styles()` generalised: distinct album keys, entries of no album counted one each).

`Shown` holds `bpm` and `[Option<&BTreeSet<String>>; 3]`. `shows(e)` passes the BPM range and, for each set filter, needs one of `e`'s values to be picked. `set_style`/`style_on`/`clear_styles` become `set_pick(facet, …)`/`picked(facet, …)`/`clear_picks(Some(facet) | None)`.

*Why:* the three filters differ only in where the values come from. One model keeps OR within a facet, AND across facets and BPM, persistence, and the "gone value stops filtering" rule identical by construction. *Alternative:* add `artists_on` and `labels_on` beside `styles_on`. Rejected: three copies of the same logic and of its tests.

**Saved form:** `SavedPlaylist` keeps `styles` (so crates saved by the previous version load their style filter) and adds `artists` and `labels`, each `#[serde(default, skip_serializing_if = "BTreeSet::is_empty")]`.

### 2. The record's artist goes on the origin

`Origin` gains `artist: String` (`skip_serializing_if = "String::is_empty"`), filled from `RecordInfo.artist` in `origin()`. `records_to_backfill` also asks for records whose `artist` is empty, and `backfill()` fills it, from the disk cache only, as for album, cover and styles. A record whose credit is empty (no data) simply has no artist value: it is hidden while an artist filter is set, like an entry with no style.

The record row's first line uses `origin.artist` when set, else the first entry's artist (local albums, and older entries until backfilled). The entry's own `artist` (shown in single-line rows and columns) doesn't change.

*Why the origin:* every track of a record gets the same value, which makes the artist filter act per record, as chosen. It is also the right credit for record rows.

### 3. Footer layout: three tiers, decided from widths

After `bpm_control` (which returns its end), a `filter_controls` step measures, in skin pixels with a 5 px gap:
1. **chips + buttons**: the style chips (when there are ≤ 20 styles) followed by ARTISTS and LABELS, when all of it fits up to the time box;
2. otherwise **buttons**: STYLES, ARTISTS and LABELS, when they fit;
3. otherwise **nothing**: the ☰ entries remain.

A facet with fewer than two values in the crate has no chip or button (as styles today). A button reads its name, plus the number picked when its filter is set ("ARTISTS 2"), and is lit while set. A double-click on a button clears its facet, and a click opens its list.

*Why tiers rather than per-control fallbacks:* the order of controls never changes, and the rule is easy to test at fixed widths. Dropping a single button while its neighbours stay would make the footer layout depend on which filter has the longest label.

### 4. One list popup, three facets

`facet_list(ui, facet)` is the existing STYLES popup generalised: a search field (case-insensitive contains, one search text per facet), checkboxes with record counts in a scroll area of at most 320 points, and Clear for that facet. The footer buttons anchor it under themselves. The ☰ entries open it as a popup anchored at the ☰ button, stored as `open_facet: Option<Facet>` so it stays open across frames until a click outside it.

### 5. ☰ entries

Filter by style…, Filter by artist… and Filter by label… are shown in the wantlist and collection crates when the facet has at least two values. Show all records clears the three facets (not the BPM range, which keeps Show all tempos) and is shown only while one of them filters. Show all styles is removed, since Show all records covers it. `P` on a hidden playing entry still clears every filter, now all four.

## Risks / Trade-offs

- [Exact credit strings split one artist across several values ("Theo Parrish" and "Theo Parrish & Marcellus Pittman")] → Chosen by the user. The search field finds both, and picking both shows both. Splitting can come later on the same facet model.
- [Records dug before the record artist existed show no artist until backfilled] → The backfill runs when the crate is shown, from the cache. A record not in the cache keeps an empty artist and is hidden only while an artist filter is set.
- [Counting hundreds of artists and labels for 5,000 entries every frame] → Counts are cached by crate and revision (one cache per facet), like the style counts. Picking a value keeps the 16 ms budget, tested on a 5,000-entry crate in a release build.
- [Old style-only crate files] → `styles` keeps its name and meaning, and the new fields default to empty.

## Migration Plan

No migration step. Crate files gain optional `artists` and `labels` selections and an optional `artist` on origins. Older builds ignore them, and older files load unfiltered by artist and label.

## Open Questions

None.
