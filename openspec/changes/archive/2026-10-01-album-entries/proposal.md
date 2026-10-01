## Why

Digging works record by record, but the playlist only shows tracks. You can't see which record a track is from, you can't get rid of a whole record you've judged in one move, and you can't see its sleeve, which is often how a digger recognises a record.

## What Changes

- **Album on every entry:**
  - Discogs entries take the release title (a master takes its own title).
  - Local files take their album tag, which the decoder already reads and the playlist now keeps.
  - Entries dug before this change get their album from the disk cache when their crate is shown. **No extra requests.**
- **Album in the row:** "12. (LT-012) Nightcraft: Glasshouse · Glasshouse EP (124 BPM)".
  - The album is left out when it is the same as the title (singles).
  - It's the first part cut when the row is narrow.
  - In the wide layout it gets its own **Album** column, which can be sorted, resized and hidden.
- **Album in the tooltip**, as an "Album" line.
- **Right-click shows the album:** while an entry's menu is open, every other entry of the same album is tinted. The selection does not change. The menu gains:
  - **Remove album (N tracks)**;
  - **Select album**.
- **Cover on hover:** the tooltip of a Discogs entry shows the release's cover, 96 px.
  - It's fetched in the background from the thumbnail address the release data already holds, and cached on disk.
  - Building the tooltip still never reads files or makes requests. A cover that isn't loaded yet appears when it arrives.
- Out of scope: a maximum price. The Discogs API reports only the count and lowest price for a release, which the tooltip already shows.

## Capabilities

### New Capabilities
- `album-entries`: the album on entries, which entries form an album, the right-click tint, Remove album and Select album, and covers on hover.

### Modified Capabilities
- `playlist`: the album in the single-line row, the Album and cover in the entry tooltip, and the new items in the entry menu.
- `playlist-columns`: the Album column.

## Impact

- `crates/dig`:
  - the release title and the thumbnail address (`images[0].uri150`, or a listing item's `thumb`) into `RecordInfo`;
  - a cover fetcher for `i.discogs.com`, separate from the API rate limiter, with a disk cache under the cache dir.
- `crates/ui`:
  - `playlist.rs`: `Origin.album`, `Origin.cover`, `Entry.album` for local files, the album key;
  - `format.rs`: the row name and tooltip lines;
  - `columns.rs`: `Field::Album`;
  - `app.rs`: the tint, the menu items, the cover in the tooltip, a small texture cache.
- The `image` dependency gains the `jpeg` feature.
- Help panel, README (cover cache location, test count).
- Independent of `crateamp-controls`. Both add an `Origin` field filled from the cache when a crate is shown. Whichever is built second reuses the other's backfill.
