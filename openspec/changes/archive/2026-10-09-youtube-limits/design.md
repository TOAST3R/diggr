## Context

- `YtDlp::fetch` and `YtDlp::search` turn any yt-dlp failure into `FetchError::Failed(last stderr line)`. The scheduler handles them the same way:
  - a failed download counts a try, and the clip is given up after 2 (`Failed(clip, "clip failed")` → "clip failed");
  - a failed search is put aside for 60 s (`search_failed`) and tried again.
- Search results, including "not found", are remembered for 7 days in `searches.ron`. A "not found" track isn't searched again before then.
- Download all tracks (label-crates) keeps a background list in the worker and a window in the app. The app counts progress from the label crate's entry statuses.
- yt-dlp's wording when YouTube limits it varies by version. The lines seen in practice include `HTTP Error 429: Too Many Requests` and `Sign in to confirm you're not a bot` (with a straight or curly apostrophe), and sometimes `rate-limited`.

## Goals / Non-Goals

**Goals:**
- A limited YouTube never turns good tracks into "clip failed" or "not found".
- Waiting it out happens by itself, visibly, and can be cut short (Try now).
- Tracks that did fail can be tried again from the label crate.

**Non-Goals:**
- Pacing Download all tracks ahead of time (a pause between requests). Discussed, and left out for now.
- Cookies or signing in to YouTube (`--cookies-from-browser`).
- Retrying "no clip" or "already in crate" tracks.
- A global "Retry failed" for crates that aren't label crates.

## Decisions

### 1. `FetchError::Limited`, recognised from the error text

`fn limited(stderr: &str) -> bool` looks at all of stderr, not only the last line, case-insensitively, for `http error 429`, `too many requests`, `confirm you're not a bot` / `confirm you’re not a bot`, and `rate-limit`. When it matches, `fetch` and `search` return `FetchError::Limited(line)` instead of `Failed`. Unit tests use real yt-dlp error lines, plus "Video unavailable" and "Private video", which stay `Failed`.

*Alternative:* yt-dlp's exit code. Rejected: it's 1 for everything.

### 2. One limited state in the worker, with doubling waits

The scheduler gains `limited_until: Option<Instant>` and `backoff: Duration`. The backoff starts at 10 min, doubles on each limited try, and is capped at 60 min.
- **On a limited answer:** the clip or search is put back as it was. Nothing is given up, the try count isn't raised, and a search isn't marked failed. Then `limited_until = now + backoff`, and the worker sends `PreviewEvent::Limited { until }`, as Unix seconds, from which the app shows the minutes left.
- **While `limited_until` is in the future:** `fill`, `fill_background` and `search_next` start nothing. Downloads already running finish or fail on their own.
- **When the time comes:** the next start is a probe. One success clears the state, resets the backoff to 10 min and sends `PreviewEvent::Unlimited`. Another limited answer doubles the backoff.
- **`PreviewCommand::TryNow`** sets `limited_until` to now, so the next start is the probe.

The test config can shorten the base wait (`Config.limited_wait`), as `recheck` and `timeout` already are.

*Alternative:* pause only Download all tracks. Rejected: when YouTube refuses, the playing track and the 3 after it fail just the same, and would be marked "clip failed".

### 3. Telling the user

- **The app** keeps `limited_until: Option<u64>` from the events.
- **On `Limited`:**
  - the main window notifies "YouTube is limiting requests: trying again in N min", only when it wasn't limited already;
  - Download all tracks' window, if a download runs, is shown and draws its pause line with **Try now**, which sends `TryNow`.
- **On `Unlimited`:** the state clears.
- **The entry tooltip** for a waiting entry adds "waiting for YouTube" while limited.

### 4. Retry failed

- **Which tracks:** a label crate's entries in `Unavailable(ClipFailed)` or `Unavailable(NotFound)`.
- **Retry:**
  1. puts them back to `Waiting(Queued)` and `Waiting(Search)`;
  2. sends `PreviewCommand::Retry { clips, searches }`. The worker removes the clips from `given_up` and `failures`, and forgets the searches' remembered results (by track and length, and by old key) and `search_failed`. `Searches::forget(req)` does the forgetting, and the file is saved;
  3. starts Download all tracks on that crate if it isn't running, which shows its window.
- **Where:** the window shows **Retry failed (N)** when N > 0. The label menu shows **Retry failed tracks (N)** when that crate isn't downloading and N > 0. One function, `retryable(crate) -> (Vec<EntryId>, clips, searches)`, serves both, along with the count.

## Risks / Trade-offs

- [yt-dlp changes its wording] → The matcher looks at the whole stderr for several phrases. A miss falls back to today's behaviour, a failed try, and Retry failed recovers from it.
- [Pausing the horizon too] → While limited, an un-downloaded next track won't play. That's the honest state: it would fail anyway. Downloaded previews keep playing.
- [A long limit (hours)] → The waits cap at 60 min, so a check is never more than an hour away, and Try now is always there.

## Migration Plan

None.
