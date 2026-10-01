## 1. Album on entries

- [x] 1.1 `dig`: the release or master title and the thumbnail address (`images[0].uri150`, a master falling back to its main release; a listing item's `thumb`) into `RecordInfo` and `Listed`; add images to the fixtures; tests
- [x] 1.2 `Origin.album` and `Origin.cover` (saved, omitted when empty); `Entry.album` from `TrackInfo.album` for local files, never overwriting an origin; tests (survives a copy and a restart, tags don't overwrite)
- [x] 1.3 Fill albums and cover addresses for older entries from the disk cache when their crate is shown, with no request; test against a recording transport that sees no request

## 2. Showing the album

- [x] 2.1 `format::entry_name` with the album (" · Album", left out when equal to the title); the album cut first in a narrow row; unit tests
- [x] 2.2 `Field::Album` column (between Title and BPM, hideable, sorts case-insensitively with empties last); the title cell without the album; tests
- [x] 2.3 "Album" line in `format::entry_details`; test

## 3. Albums on right-click

- [x] 3.1 The album key (release, else master; local: lower-cased artist + album) and `Playlist::album_of`; unit tests (same release, master without release, same name different artist, no key)
- [x] 3.2 The tint while the menu is open, with the selection unchanged; headless test
- [x] 3.3 Remove album (N tracks) and Select album (enabled, disabled, absent; the playing entry finishes; the cursor goes to the clicked entry); headless tests

## 4. Covers

- [x] 4.1 `image` with `jpeg` and `webp`; `CoverFetcher` in `dig`: one low-priority thread, the image host only, at most 4 per second, newest request replaces a waiting one, decode, scale to ≤ 150 px, PNG under `<cache>/covers/`, failures remembered for the session, back-off on 429; tests with a fake transport and a `TestDir`
- [x] 4.2 A stale address (403/404): refetch the release JSON once through the normal limiter, then retry the cover; test
- [x] 4.3 UI: `CoverCache` (LRU of 48 textures, memory only); a request after a 250 ms rest; the cover beside the details, with an empty 96 pt frame until it loads; repaint on arrival; headless tests (no file read or request while building the tooltip, sweep fetches only the last row)

## 5. Docs and checks

- [x] 5.1 README (album, Remove album, covers, the covers cache folder), help panel, test count
- [x] 5.2 Run the app to try albums, the tint and covers; fmt, clippy, the workspace tests, the wasm check, `--click-test` and `--startup-time`
