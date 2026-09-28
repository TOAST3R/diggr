## Why

When digging, the first things you read on a record are its catalog number and its tempo. Today an entry reads "Artist - Title". The catalog number is buried in a " · A1 · LT-012 · 1994 · …" tail, and the BPM isn't shown anywhere, although analysis already knows it.

## What Changes

- Every entry's name reads **`(catno) Artist: Title (N BPM)`**, in the playlist and in the main window's title line.
  - `(catno)` appears only when the entry has a catalog number (Discogs entries). Local files have no prefix.
  - `Artist: ` appears only when there is an artist.
  - `(N BPM)` appears once the track's tempo is known, and is absent until then.
- **BPM normalised into a DJ range:** the analysed tempo is folded by halving or doubling into 88–176 BPM, then rounded to a whole number. For a track with tempo changes, the longest steady-tempo stretch is used.
- **Tempo on the entry:** each entry keeps the BPM once any analysis of its audio knows it: the preview preparation, the playing track's analysis, or the score cache. The BPM is saved with the crate, so it shows at once after a restart.
- **Shorter tail:** the catalog number is removed from the ` · …` details after the title line (side, year and for-sale stay), since it now leads the name.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `playlist`: the entry display format changes, and entries carry a tempo.
- `player-window`: the title line uses the new name format, and its Discogs tail loses the catalog number.

## Impact

- `crates/ui/src/playlist.rs`: `Entry::display_name`, a new `bpm` field on `Entry` and `SavedEntry` (optional, so old crates still load).
- `crates/ui/src/format.rs`: `title_line`, `origin_details`, a new `dj_bpm` fold.
- `crates/ui/src/app.rs`, `crates/ui/src/app/digging.rs`: feed BPMs to entries from preview preparation, playing-track analysis and cache hits.
- `crates/dig/src/prepare.rs`: report each prepared preview's tempo back to the UI.
- `analysis`: a helper returning a score's dominant tempo (the longest `TempoSegment`).
- No playback-path changes.
