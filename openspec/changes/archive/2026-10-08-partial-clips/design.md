## Context

`intake.rs::expand_next` turns a fetched record into one `Outcome`: `Clips` when `matching::entries` matched or kept any clip, `Tracks` (every track to search) only when it found none, `Unavailable` otherwise. The two are exclusive, so a record with one video for eleven tracks (*Mezzanine*, release 5077187) yields one entry. `ui/app/digging.rs` applies the outcome to the crate: vinyl-first twin handling, clip dedupe (`have.insert(clip)`), placeholders replaced by entries. Track searches (`preview-search`) run near the playhead, one at a time, and are remembered in `searches.ron` by `release/<id>/<position>`.

Discogs data for *Mezzanine*: 11 tracks (A1–D2), one video ("Teardrop (Official Video)", 4:45) listed twice; the master (23683) has the same single video, so the master gives nothing extra.

## Goals / Non-Goals

**Goals:**
- One entry per track of the tracklist, always; extra entries only for clips that match no track.
- A repeated video counts once; a full-album upload is a fallback, not a duplicate of the tracks.
- yt-dlp runs at most once per track, whatever release, page or crate it comes from.

**Non-Goals:**
- Changing crates filled before this change (they are completed on the next send).
- Using the master's clips to fill a release's gaps (no extra Discogs request).
- Splitting a full-album video into tracks by timestamps.
- Changing when or how fast searches run (still the download-ahead window, one at a time).

## Decisions

### 1. One plan per record instead of an either/or outcome
`matching` returns a `RecordPlan { tracks: Vec<PlannedTrack>, extra_clips: Vec<ClipEntry>, full_album: Option<ClipEntry> }`, where `PlannedTrack` is `Clip(ClipEntry)` or `Search(TrackEntry)`, in tracklist order. `Outcome::Clips` and `Outcome::Tracks` merge into `Outcome::Entries(RecordPlan)`; `Unavailable` stays. Clip ids are deduped before matching, so a repeated video can't become an unmatched extra.
*Alternative:* keep both variants and add `Mixed`. Rejected: three paths through `digging.rs` where one does, and "every track once" is easier to test on one value.

### 2. Remix credits use the same plan, filtered
For `Role::Remix`, both the clips and the tracks are filtered by `names_artist` before planning; tracks that survive and have no clip become `Search`. A record where the remixer's track has no clip and its title doesn't name them still adds nothing (as today).

### 3. Full-album clip detection
An unmatched clip is a full-album clip when its folded title contains "full album", or when it is the record's only clip (after dedupe) and matches no track. It is held back only when the plan has tracks to search; with nothing to search (no tracklist, or every track matched) it is an ordinary extra clip, so it is never lost. Detection is in `matching` so it is unit-tested. Side rips and mixes that aren't the only clip stay ordinary extra clips.
*Alternative:* a duration threshold (≥ 50 % of the tracklist). Rejected by the user: title and "only clip" are the signals.

### 4. The held-back clip lives on the record's entries
Each `Search` entry of the record carries `album_clip: Option<String>` (and its title) in its `Origin`, saved with the crate (serde default `None`, so old crates load). When a search ends "not found by search", `digging.rs` looks at the failing entry's `album_clip`; if no entry of the crate has that clip, it inserts one entry for it after the record's last entry (same album membership), then clears `album_clip` on that record's entries. A restart needs nothing special: the clip rides with the crate.
*Alternative:* a side table in the dig state. Rejected: it would need its own persistence and would drift from entries the user removes.

### 5. Search results keyed by the track
New key `track/<folded artist>/<folded title>` (`matching::fold`: case, accents and punctuation; no words dropped, so "Glasshouse (Lumen Remix)" stays its own track). `searches.ron` gains a `tracks` map from track key to a list of results, one per length; `Remembered::Found` gains the video's `duration` and `NotFound` the track's (serde defaults). A lookup is a hit only when the request's duration is unknown, the remembered duration is unknown, or they agree within the trust tolerance (10 s or 5 %); a new result replaces the one of the same length. `Found` / `NotFound` events carry that duration, and the UI applies them only to entries under the key whose length agrees. Saved crates keep their old `release/…/<pos>` `search_key`; the old `results` map is still read by that key, so old results keep working. New results are written under the track key only.
*Alternative:* `master/<id>/<pos>`. Rejected: CD and vinyl positions differ (1 vs A1), and compilations share tracks across masters.

### 6. "Already in crate" instead of a duplicate
When a search (or remembered result) gives a video another entry of the crate already has as its clip, the entry becomes unavailable with "already in crate". No further search is run (the result is the result), and it does not trigger the full-album clip. This keeps the count of entries equal to the tracklist and the crate free of repeats. It covers *Mezzanine*'s B2 "Exchange" / D2 "(Exchange)", which share a track key.

### 7. Resend dedupe for tracks to search
Before adding a `Search` entry, `digging.rs` skips it when the crate holds an entry with the same record (release, or master when no release) and position. Same check as clips use, by a different key.

### 8. Vinyl takeover matches tracks to search too
The twin takeover in `digging.rs` currently matches a non-vinyl entry to the vinyl plan by clip id. It also matches by track key: a non-vinyl `Search` entry (waiting, found or not found) whose track key equals a vinyl `Search` track becomes that vinyl entry in place, keeping its found clip and its playback. Without this, a playing found-by-search entry would stay as a leftover and the vinyl twin's same track would end "already in crate".

### 9. Record row count
`format::record_counts` builds the counting line from the record's entries in the crate: total, entries whose clip came from Discogs (origin has a clip and no search key), and entries still "to search". It is shown only when the record has an entry with a search key, as the first detail line ("Record") of the record row's tooltip.

### 10. Test fixture release 1001
Release 1001 had four tracks and three videos; many headless tests use it as "a release whose previews all play". Its clip-less A2 is dropped from the fixture so those tests keep their meaning; the partial case is covered by a new fixture, release 5077187 (*Mezzanine*: 11 tracks, one video listed twice), and by `matching` unit tests.

## Risks / Trade-offs

- [A single mislabelled clip ("Track 3") on a record is treated as a full-album clip and held back] → It is still added after the first failed search; its track is searched meanwhile, so the record is heard either way.
- [Track key collisions: a reprise or a same-titled track on another record shares a result] → Only within the duration tolerance; a different length searches again. Inside one crate the repeat becomes "already in crate" rather than a duplicate.
- [More searches per send: an 11-track record with one clip now queues 10 searches] → Searches stay inside the download-ahead window and run one at a time, so a send never bursts yt-dlp; results are reused across releases.
- [Larger crates from labels whose releases had partial clips] → Intended; the record row count makes the mix visible, and Remove album still removes a record at once.
- [Old `searches.ron` keys never migrate] → They are read as-is for old entries; new entries use track keys. The file only grows by the new keys.

## Migration Plan

No migration step. `Origin.album_clip` and `Remembered::Found.duration` default to `None`. Existing crates are not re-expanded. Rollback: an older build ignores the new fields and keys (serde skips unknown fields only if allowed, so confirm `Origin` and `Remembered` deserialize with `#[serde(default)]` and without `deny_unknown_fields`).

## Open Questions

- None blocking. If "already in crate" shows up too often on compilations, a later change could try the next usable search result instead.
