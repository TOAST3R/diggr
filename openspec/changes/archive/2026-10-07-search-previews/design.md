## Context

- **Records and entries today.** `intake::expand_next` turns a record into `Outcome::Clips(entries)`, where `matching::entries` matches each Discogs video to a tracklist track. Without clips it becomes `Outcome::Unavailable("no clip")`. The UI (`dig_record`) replaces the record's "listed" placeholder with those entries. Each entry's `origin.clip` is an 11-character video id, `clip_url` rebuilds its URL, and `preview_path` names its file.
- **The preview worker** (`preview/scheduler.rs`) downloads only the horizon (armed, playing and the next 3 in play order), 2 at a time, through a `Fetcher`. The real one runs the user's yt-dlp with fixed arguments (`--ignore-config`, `--no-playlist`, `--`, the rebuilt URL). The UI computes the horizon in `dig_horizon` from the entries' clips.
- **The tracklist** (`model::Track`) has position, title, the track's own artists (empty when the release's) and a duration. It's already fetched with the record's details, with no extra request.
- **The `preview-fetch` "Safe invocation" requirement** says yt-dlp is only given a validated id. A search breaks that, so the requirement is extended, not bypassed.

## Goals / Non-Goals

**Goals:**
- Records without clips show and play their tracks when a trustworthy video exists.
- No search until a track is about to play, one search at a time, and each result is remembered.
- A wrong video is far less likely than "not found": the rule prefers missing over mismatched.
- The same safety for yt-dlp as for downloads.

**Non-Goals:**
- Other sources; filling gaps in records that have some clips; picking results by hand; the YouTube API.

## Decisions

### 1. `Outcome::Tracks`: entries before any search

`matching` gains `track_entries(record) -> Vec<TrackEntry>`: one per tracklist track, with position, title, artist (the track's own, else the record's), duration and a search key `release/position`. `expand_next` returns `Outcome::Tracks(..)` when a record has no clip that matched or is usable and has a non-empty tracklist. Otherwise it returns `Unavailable("no clip")` as today.

In the UI, `dig_record` turns each `TrackEntry` into an entry with no clip, `origin.position` set, the duration known, and status `WaitKind::Search` ("to search"). A searchable entry is in play order (it's waiting), so it gets into the horizon like a clip waiting to download.

*Why entries first:* the user sees the tracklist at once, and the search happens only for what is about to play.

### 2. Searches ride on the horizon

`dig_horizon` already walks the window in play order. For entries in it with `WaitKind::Search`, it builds a `SearchRequest { key, artist, title, duration }` and sends `PreviewCommand::Search(list)` with the window's requests, in the same priority order. The worker keeps one search running at a time, ahead of downloads. A request that leaves the window is dropped if it hasn't started.

On `PreviewEvent::Found { key, clip, video_title }`, the UI:
- sets `origin.clip` and `origin.found` (the video's title, saved) on every entry with that key, in every loaded crate;
- moves them to `WaitKind::Queued`.

The next horizon then downloads them the usual way. On `NotFound { key }` they become unavailable with "not found by search".

*Alternative:* search at expansion time, for every track of every record. Rejected: a 600-record label would run hundreds of yt-dlp searches up front, against the "nothing ahead of the horizon" rule of `preview-fetch`.

### 3. The search command

```
yt-dlp --ignore-config --no-warnings --flat-playlist --skip-download \
       --print "%(id)s\t%(duration)s\t%(channel)s\t%(title)s" -- "ytsearch5:<query>"
```

- **The query:** artist and title from the tracklist, joined by a space; control characters and newlines removed; trimmed to 120 characters. It's passed as one argument after `--`, so it can't be read as an option.
- **No shell:** it runs as an argument list, with a 30 s timeout and the same program as downloads.
- **Output:** one line per result. Lines whose first field isn't a valid 11-character id are ignored.

### 4. The trust rule

Each candidate is scored after normalising its title, the track's title and the artists (lower-case, accents folded, punctuation as spaces, "original mix" and similar suffixes dropped, using `matching`'s normaliser):

- **title:** the candidate's title contains the track's title (all its words, in order);
- **artist:** the record's or the track's artist appears in the candidate's title or channel ("The 89th Passenger", "The 89th Passenger - Topic", or the label's name as channel);
- **duration**, when both are known: within max(10 s, 5 %).

A candidate passes when it has the title and the artist, and the duration when known. Among those that pass, the closest duration wins (the first result when no duration is known). None passing means not found.

*Why so strict:* a wrong track playing under a record's name misleads a digger more than a missing preview does.

### 5. Remembering results

`<cache>/searches.ron` maps `release/position` (or `master/position`) to `Found { clip, title }` or `NotFound { at }`.
- A cached Found is used without running yt-dlp.
- A NotFound younger than 7 days is too.
- The file is written atomically after each result.
- It's left alone when the preview cache is cleared, since it holds no audio.

### 6. What entries show

- **Single-line rows:** "Artist - Title" from the tracklist.
- **Tooltip:** a "Preview" line: "to search", "found by search: ‹video title›" or "not found by search".
- **Status icon:** a waiting entry reuses the queued clock.
- **Vinyl first:** it treats an entry without a clip as silent, so a digital twin's searchable entries leave when the vinyl release is in the crate. Only the vinyl's tracks are searched.

## Risks / Trade-offs

- [A remix or a live version matches a studio track] → Duration within 10 s/5 % plus title containment filters most of them. The tooltip names the video, so a wrong one is visible. Picking by hand is out of scope.
- [YouTube throttles or blocks searches] → One at a time, only in the window, and cached. A search error counts as "not found" for this session only (not cached), so it's tried again next session.
- [yt-dlp's output format changes] → The parser ignores lines it can't read. Unit tests pin the format, and the real-search test is `#[ignore]` (network).
- [Old crates hold "no clip" rows for records that now have tracks] → They stay until the page is sent again. The record's details are cached, so no request is needed to rebuild it then.

## Migration Plan

No migration. New fields (`WaitKind::Search`, `origin.found`) are optional when saved. Older builds would see these entries as waiting entries without a clip, which they show as queued and never download.

## Open Questions

None blocking.
