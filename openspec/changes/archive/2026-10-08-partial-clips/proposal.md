## Why

A record whose Discogs page has fewer videos than tracks arrives as its videos only. Massive Attack's *Mezzanine* (release 5077187) has 11 tracks and one video, "Teardrop (Official Video)", listed twice, so sending it adds a single entry and the other ten tunes are never heard. Tracks are searched on YouTube only when a record has no clip at all; a record with some clips falls between the two rules.

## What Changes

- **Every track gets one entry.** A record expands to one entry per track of its tracklist: the track's clip when one matches, otherwise a track waiting to be searched (`preview-search`). Clips that match no track still come after, as today. *Mezzanine* gives 11 entries: Teardrop with its clip, ten to search.
- **Repeated clips count once.** A video listed more than once on a record adds one entry.
- **Remix credits fill gaps too.** For a remix credit, the tracks whose title names the remixer and have no clip wait to be searched.
- **Full-album clips are a fallback.** A clip that matches no track and whose title says "full album", or the record's only clip when it matches no track, is held back: the tracks are searched one by one, and the full-album clip is added after the record's entries only when one of those searches ends "not found by search". The held-back clip survives a restart.
- **Searches remembered by the track.** A search result is remembered by the track's artist and title, not by release and position, so another release of the same record (vinyl and CD twins), a resend or another crate never runs yt-dlp again for a track already searched. A searched video already in the crate is not added twice. Results already remembered under the old keys keep being used.
- **Sending twice adds no tracks twice.** A track waiting to be searched (or found by search) that the crate already holds is not added again.
- **A record says what it holds.** A grouped record row's tooltip says "11 tracks · 1 clip · 10 to search".
- Crates already filled are not changed; a record is completed when it is sent again.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `discogs-intake`: "Expanding a page into tracks" fills every track of a record, counts repeated clips once, applies to remix credits, holds back full-album clips, and keeps tracks to search from being added twice.
- `preview-search`: "Tracks waiting to be searched" covers tracks of records with some clips; "Trusting a result" adds the full-album fallback and skips a video the crate holds; "Search results are remembered" keys results by the track.
- `record-view`: "Record row" tooltip counts tracks, clips and tracks to search.

## Impact

- **`crates/dig`:** `discogs/matching.rs` (one plan per record: clip per track, tracks to search, extra clips, held-back full-album clip; repeated clip ids), `intake.rs` (`Outcome` carries clips and tracks together; `expand_next`), `preview/search.rs` (track key, remembered duration, fallback to old keys).
- **`crates/ui`:** `app/digging.rs` (applying a mixed outcome, vinyl-first twins with tracks to search, dedupe by search key, adding the full-album clip after a failed search), `playlist.rs` (`Origin` carries the held-back clip), `format.rs` / record row tooltip.
- **Tests:** `matching.rs` unit tests (Mezzanine shape, repeated clip, full album, remix), `crates/dig/tests/intake.rs`, `crates/ui/src/app/headless_tests/dig_tests.rs`. README test count.
- No new dependency, no extra Discogs request; searches stay near the playhead, one at a time, off the playback path.
