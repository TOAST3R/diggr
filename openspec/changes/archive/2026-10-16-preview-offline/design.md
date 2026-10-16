## Context

- **What happened:** on a network behind a TLS-intercepting proxy, yt-dlp fails every request with `[SSL: CERTIFICATE_VERIFY_FAILED] … Unable to download API page`. `fetcher::failure()` turns any error that isn't a limit into `FetchError::Failed`. The scheduler counts it, and after `ATTEMPTS` (2) it gives up: `PreviewEvent::Failed(clip, "clip failed")`. The entry becomes `Unavailable(ClipFailed)`, is saved that way, and never comes back by itself.
- **Searches:** a failed search is remembered in `search_failed`, so the track is retried late or shows "not found".
- **What already exists:** limiting (`FetchError::Limited`) does exactly what an unreachable source needs: no failed try, the request is put back, one pause per source (`limits`, `until`, `backoff`), the `Limited`/`Unlimited` events, the main window message and "waiting for YouTube". Retry failed (`retryable()`, `PreviewCommand::Retry`) already puts dead entries back, but only for label crates.
- **Double-click today:** `play_entry` arms a waiting entry, and for an unavailable one only notifies "‹name›: clip failed".

## Goals / Non-Goals

**Goals:**
- A network problem never marks a track dead.
- The user learns the real cause, including a hint when secure connections are intercepted.
- One gesture brings back an entry that died for any reason that can change.

**Non-Goals:**
- Working around the proxy: no option to skip certificate checks, and no custom CA bundle passed to yt-dlp.
- Reviving entries saved as "clip failed" by older versions automatically. A double-click or Retry failed does it.
- Changing the Discogs client's offline handling.

## Decisions

### D1. `FetchError::Unreachable(String)`, classified like `Limited`
- **What counts:** `unreachable(errors)` beside `limited()`, matching yt-dlp's output case-insensitively for transport failures only:
  - `certificate verify failed`, `ssl:`;
  - `urlopen error`, `connection refused`, `connection reset`, `network is unreachable`;
  - `nodename nor servname`, `name or service not known`, `temporary failure in name resolution`, `failed to resolve`.
- **Order:** `failure()` checks the limit first, then unreachable, then falls back to `Failed`.
- **What doesn't count:** an HTTP status (`HTTP Error 404`), "Video unavailable" and our own 120 s timeout stay failures, since the server answered or the clip itself is suspect. A test pins each pattern and each exclusion.
- **Juno:** ureq's transport errors (`Io`, `Tls`, `ConnectionFailed`, `HostNotFound`, `Timeout`) map to `Unreachable`; HTTP statuses keep their current mapping.
- **Why not a flag on `Failed`?** Every match on `Limited` already sits where the new case goes. A variant makes the compiler list them.

### D2. One pause per source, two kinds
- **Shared machinery:** `Limit` gains `kind: Pause` (`Limited` or `Unreachable(reason)`), and the scheduler uses the same `limited(source)` path with a `kind` argument.
- **Waits:** limited keeps 10 → 20 → 40 → 60 min. Unreachable uses `cfg.unreachable_wait` (1 min) → 2 → 4 → 8 → at most 10 min, because a network comes back sooner than a rate limit lifts.
- **Kind changes:** if a try under one kind fails with the other, the new kind and its base wait take over.
- **Success:** `answered()` clears either kind and resets the wait, as today.
- **Events:** `PreviewEvent::Limited { source, until }` gains `reason: Option<String>`: `None` when limited, `Some(line)` when unreachable. That keeps one event and one UI map (`d.limited`), which already drives "waiting for ‹source›" and pausing Download all tracks.
- **Why share:** two parallel mechanisms would double the per-source state and the UI wiring for the same behaviour, "don't start requests to that source until …".

### D3. Message
- **Text:** `limited_text` gets the reason. Unreachable reads "Can't reach YouTube: trying again in ‹N› min".
- **Certificate hint:** when the reason contains `certificate`, it adds " (a proxy, VPN or security tool may be intercepting secure connections)".
- **Frequency:** once per period, like the limit message. No product names.

### D4. Double-click on a dead entry
- **What it does:** in `play_entry`, `Unavailable(ClipFailed)` and `Unavailable(NotFound)` reuse what Retry failed does for one entry:
  - set it waiting (`Queued` for a clip, `Search` for a search);
  - forget its remembered failure or "not found";
  - send `PreviewCommand::Retry { clips, searches }` with just it;
  - arm it.
  The status line then says it is waiting for that entry, as for any armed entry.
- **What stays the same:** "no clip" and "already in crate" keep the notification.
- **The function:** `retryable()`'s per-entry part becomes a small function used by both Retry failed and the double-click. The label-crate filter stays on Retry failed only.
- **Crates:** works in any crate, playing or shown. An armed clip is already wanted whatever its crate (`horizon`). For a search, `dig_horizon` must also take the armed entry when it is in another crate than `cid`, which today it drops (`armed.filter(|(c, _)| *c == cid)`). This change fixes that too.

### D5. `none` catalogue number
`entry_name` treats a catalogue number equal to `none` (any case, trimmed) as empty. Only the display changes: the stored value stays as Discogs sent it, so matching by catalogue number is unchanged.

## Risks / Trade-offs

- **[A pattern matches a real clip failure]** → The patterns are transport-level wording that a dead video never produces. If one slips, the cost is a track waiting instead of "clip failed", and a later successful request clears the pause.
- **[Permanently blocked network]** → The app retries every 10 minutes forever, one request per try per source, and says so once per period. That's cheap and honest. The entry tooltips say "waiting for YouTube" instead of "clip failed".
- **[A broken clip that also hits network errors]** → It still fails on its own error once the network works. Nothing is lost.
- **[Double-click retries a really dead clip]** → It costs two more yt-dlp runs, then "clip failed" again, which is what the user asked for.

## Open Questions

- Should a successful request after an unreachable period also revive entries marked "clip failed" during the last few minutes before the pause began? Probably not worth tracking. Double-click and Retry failed cover it.
