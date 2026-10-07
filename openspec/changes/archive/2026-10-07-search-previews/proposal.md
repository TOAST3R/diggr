## Why

Many records have no videos on Discogs, new releases especially. AF069 "Hidden Soul EP" (Analogical Force, 2026) has five tracks with titles and durations, but no clip on its vinyl release, its FLAC release or its master. Today such a record is one "no clip" row titled after the record, so a digger can neither see its tracks nor hear them. The tracks are usually on YouTube anyway, uploaded by the label or the artist. The user's own yt-dlp can find them by search, so no Discogs request and no API key are needed.

## What Changes

- **Tracks instead of one "no clip" row.** A record whose details have no usable clip but do have a tracklist becomes one entry per track: "Artist - Title" (the track's own artist when credited), its side and its duration from the tracklist. Each entry waits with the status "to search". A record with neither clips nor a tracklist stays one "no clip" row, as before.
- **Search near the playhead.**
  - **When:** when an entry waiting to be searched enters the download-ahead window (the armed entry, the playing one and the next 3), the preview worker asks the user's yt-dlp for up to 5 search results for "artist title". No download happens at this step; the search only lists videos.
  - **Which result:** the best one is kept only when its title contains the track's title, the record's artist (or the track's) appears in its title or channel, and its duration is within 10 s or 5 % of the tracklist's (when the tracklist gives one).
  - **Found:** the video becomes the entry's clip. It downloads and plays like any other, and the entry's tooltip says it was found by search, naming the video.
  - **Not found:** the entry becomes unavailable, "not found by search".
  - **Limits:** one search runs at a time. Only entries in the window are searched, so a 600-record label never triggers 600 searches.
- **Search results are remembered** in the cache folder, by release and track position, found or not found. Not found is retried after 7 days.
- **Safe invocation.** A search runs yt-dlp with the same rules as a download (an argument list, never a shell, `--ignore-config`), with a query built only from the tracklist's text. Control characters are removed, it's at most 120 characters, and it's passed as one argument after `--` with the `ytsearch5:` prefix. Only an 11-character video id from the results is ever kept, as for Discogs clips.
- **Twins.** In a crate filled from a page, a vinyl record whose tracks are searched still has priority over its digital twin (see `discogs-intake` "Vinyl first"). A twin with no clip leaves, so a track is searched once, for the vinyl release.
- Out of scope: searching other sites (Bandcamp, SoundCloud), searching tracks missing from a record that has some clips, and choosing among results by hand.

## Capabilities

### New Capabilities
- `preview-search`: tracks of records without clips, searching for them near the playhead, the rule for trusting a result, remembering results, and how found tracks are shown.

### Modified Capabilities
- `discogs-intake`: "Expanding a page into tracks" now says that a record with no usable clip but a tracklist becomes one entry per track waiting to be searched. Only a record without a tracklist is a single "no clip" entry.
- `preview-fetch`: "Safe invocation" also covers a search: what may be passed to yt-dlp, and that only an 11-character id from its results is used.

## Impact

- **`crates/dig`:**
  - `discogs/matching.rs`: track entries for a record with no clip (`Outcome::Tracks`);
  - `preview/`: a search request and result in the worker, the yt-dlp search arguments and output parsing, the trust rule, a `searches` cache.
- **`crates/ui`:**
  - `playlist.rs`: `WaitKind::Search`, and on `Origin` the searched clip's title, saved;
  - `app/digging.rs`: entries from `Outcome::Tracks`; sending searches for searchable entries in the window; on a result, set the clip (then the usual download) or mark it unavailable;
  - `format.rs`: the tooltip line ("Found by search: ‹video title›").
- README (tracks of records without clips, how they're found, the cache file, the test count), help panel.
- No new dependency. Each search is one yt-dlp run (about 1–2 s), outside the audio path, at the preview worker's low priority.
