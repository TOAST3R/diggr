## Why

A digger thinks in records, but the playlist shows tracks. A wantlist or a collection of 300 records becomes 1,000 rows of clips, with the same label, catalog number and year repeated on every row and no sleeve in sight. The sleeve is often how a record is recognised. Showing a crate as one row per record, with its cover, that opens to show its tracks, makes those crates readable. It also suits any dig crate.

## What Changes

- **Group by record**, a view of any crate:
  - each record (album, see `album-entries`) is one **record row**: the cover on the left, then artist – album, catalog number, year, track count, and the OWNED and ✓ marks;
  - a record row **opens to show its tracks** (▸ / ▾) as indented track rows; double-clicking a track row plays it;
  - entries without an album (a local file with no album tag) stay single rows;
  - a record with only one entry is a record row that plays directly, with nothing to open.
- **Toggle:**
  - a new **▤ button in the playlist title bar**, left of ⇔, lit while on;
  - **Shift+G**;
  - a "Group by record" checkbox in the ≡ menu.

  The choice is remembered **per crate**. The Discogs crates ("Wantlist: ‹user›", "Collection: ‹user›") are grouped by default, and every other crate is flat by default.
- **Grouping gathers the crate:** turning it on moves each record's entries together, after the record's first entry, keeping their order, in the way a sort reorders a crate. Playback, shuffle, pre-warm and export then follow what you see, with no hidden second order. While grouped, new clips of a record already in the crate join it.
- **Acting on records:**
  - clicking a record row selects all its entries, so Remove, Send to crate and the Discogs items act on the record;
  - dragging a record row moves the whole record, in the list or onto a crate in the sidebar;
  - double-clicking or Enter on a record row plays its first playable track;
  - the keyboard moves over the rows you see, → opens a record and ← closes it.
- **The playing record:** a closed record row that holds the playing track shows that track's name and the play mark. P shows it.
- **Covers on record rows:** fetched for the record rows in view, top first, with the same image host, the same pace (at most 4 per second) and the same disk cache as the tooltip covers. Only rows in view are fetched, so scrolling a 1,000-record collection doesn't flood the network.
- **Filters and columns:**
  - a record whose tracks are all hidden by the BPM filter is hidden, and a partly hidden one says "2 of 4";
  - in columns, the record row spans the width and track rows use the columns;
  - sorting reorders the tracks, then gathers records again.
- Out of scope: covers for local files (embedded art), a grid or "shelf" layout, and grouping by label or artist.

## Capabilities

### New Capabilities
- `record-view`: the grouped view, covering record rows and track rows, opening and closing, the toggle and its per-crate memory, gathering, actions on records, the keyboard, the playing record, and how it works with filters, columns and sorting.

### Modified Capabilities
- `album-entries`: covers are also fetched for record rows in view, not only after a 250 ms hover, under the same host, pace and cache rules.
- `playlist`: the ▤ title bar button and the ≡ menu item.
- `player-window`: Shift+G and Space in the keyboard shortcuts.

## Impact

- `crates/ui`:
  - `playlist.rs`: gathering (a stable reorder by album key), and inserting next to a record while grouped;
  - `crates.rs`: per-crate `grouped: Option<bool>` (saved, default by crate kind);
  - `app.rs`: a row model for the list (record rows of two row heights, track rows), drawing, hit-testing, scrolling, keyboard, drag;
  - `app/covers.rs`: requests for the rows in view, and a larger texture cache;
  - `skin/generate.rs`: the ▤ sprite (normal, pressed, lit), regenerated `assets/skin/default/`;
  - `help.rs`.
- `crates/dig/src/cover.rs`: a short ordered queue of wanted covers, replaced each frame by the rows in view, instead of a single newest request.
- Builds on `discogs-write` for the wantlist crate kind. If `discogs-write` isn't built yet, only "Collection: ‹user›" is grouped by default.
- README (the view, ▤, Shift+G, the test count).
