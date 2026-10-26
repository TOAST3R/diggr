## Context

The extension already plays a whole page through `/v1/send` with mode Play: the bridge answers `202` at once, the UI expands the page in the background and `play_when_ready` plays the first entry the send adds (`browser-bridge` "Play adds to the crate on screen"). Every entry from Discogs carries `Origin { release, position }`, so "the entry for B2 of release 123" already exists in the data. `/v1/status` answers what plays (artist, title, crate) from the `Snapshot` the UI stores only when it changes; it is `None` while the engine is stopped. The audio lives in the app, so the page can only be a remote.

## Goals / Non-Goals

**Goals:**
- Play one track of a release from its tracklist row, and stop it, without leaving the page.
- The row marks stay true to the app (ends, next, stop in the app) within about a second.
- No new waiting on the bridge thread; every new request answered within 100 ms.

**Non-Goals:**
- A bar, pause, seek, next/previous, volume, want/pass, progress or BPM in the page.
- Master release pages, other sites' tracklists, the ▶ after links on list pages (unchanged).
- Playing audio in the browser.

## Decisions

**A position on the existing Play send, not a new endpoint.** `SendBody` gains an optional `position` (at most 8 characters, no control characters), accepted only with mode Play on a release page (`PageKind::Release`); anywhere else it is refused with 422. It reuses the send's checks, crate choice and background expansion. *Alternative:* `/v1/play {release, position}`: a second path to the same expansion, for no gain.

**The UI targets (release, position).** The UI's `SendMode` gains `PlayTrack(side)`, and `play_when_ready` keeps, beside today's crate and entries before the send, an optional (release, side). A track target plays the first entry of the crate whose origin has that release and side, new or not, so a second click on a release already in the crate plays at once (the dedupe that makes "Nothing new" play nothing doesn't apply). When the send ends and no such entry exists (the side isn't in the API's tracklist), nothing plays. Positions are compared after trimming and ignoring case ("b2" = "B2"). *Alternative:* the extension finds the entry id: it would need the crate's contents, more API for nothing.

**The row knows it plays from the status, polled.** `Playing` gains `release: Option<u64>` and `position: String` (from the playing entry's origin; empty for entries that aren't from Discogs). The content script polls `status` every 1 s, and only while one of its rows is ⧗ or ■; it stops when neither is. A row shows ■ when the status's release is the page's and its position is the row's, ⧗ from the click until then (or 30 s, then ▶), ▶ otherwise. The snapshot changes only when the track changes, so this costs the UI nothing per frame. *Alternatives:* a local toggle (stale as soon as the track ends or the app moves on); a long-poll like the cart's (more code for a 1 s gain).

**Stop is `POST /v1/control {"action":"stop"}`.** Keyed like every request, body ≤ 16 KB, unknown actions or fields refused with 422. It is answered `202` and handed to the UI as `BridgeCommand::Stop`, which does what the Stop button does (`engine.stop()`). Only `stop` exists; the shape leaves room without building anything for it. A paused track shows ■ too (the status names it), and ■ stops it.

**Rows are found by their position text.** Discogs' class names are hashed, so the script looks for the tracklist table's rows and takes each row's position from its first cell's text (A1, B2, 1, 1-3…). Rows with no position (headings, index tracks) get no ▶. On a live page (release 5077187) each track is a `tr[data-track-position="A1"]` inside `section#release-tracklist`, its first cell holding the side; headings carry no such attribute. The script uses that attribute (a data attribute, not a hashed class) and re-applies its buttons when React re-renders the rows. Nothing is read from the rows but that text, and only on a release page.

## Risks / Trade-offs

- [Discogs changes its markup] → the selector depends on the row structure and visible position text only, not class names; if none is found, no ▶ is shown and the page button still works.
- [First click is slow: Discogs fetch, matching, search, preview fetch] → ⧗ says it is coming; the bridge still answers at once. A release already in the crate plays at once.
- [The tracklist's side differs from the API's (rare edits, sub-tracks)] → nothing plays and the row goes back to ▶ after the send ends; the whole release is still in the crate.
- [Two tabs of the same release] → both poll and both show ■ on the playing row; correct, and each stops polling once nothing of it plays.

## Open Questions

None: the row selector was settled on a live page (above).
