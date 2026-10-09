## Why

Download all tracks can run hundreds of YouTube searches and downloads in a row. YouTube has no quota for yt-dlp, but it defends itself: after heavy use it answers "Too Many Requests" (HTTP 429) or "Sign in to confirm you're not a bot" for minutes to hours. Today Diggr treats those answers like a broken video. Each track is tried twice and then marked "clip failed", or a search counts as failed, so a big label quietly turns into a column of tracks without a preview that are perfectly fine. Once that happens, nothing short of waiting 7 days or re-sending the label brings them back.

## What Changes

- **Recognise when YouTube is limiting.** A yt-dlp error that says 429, "Too Many Requests" or "confirm you're not a bot" (for a download or a search) counts as **limited**, not as a failure of that track. The track isn't marked failed and its retry count doesn't go up.
- **Wait it out.** While limited, the preview worker starts no download or search. It tries again after 10 minutes, then 20, 40 and at most 60, and the wait resets once something succeeds. The playing track and the 3 after it wait too, since they would fail the same way.
- **Say so.**
  - The main window says once "YouTube is limiting requests: trying again in 10 min".
  - Download all tracks' window shows that it is paused for YouTube, with how long until it tries again, and offers **Try now**.
  - Tracks waiting meanwhile keep waiting; their tooltip says "waiting for YouTube".
- **Retry failed tracks.** A label crate can try its "clip failed" and "not found" tracks again:
  - from **Retry failed (N)** in Download all tracks' window, and **Retry failed tracks (N)** in the label crate's menu when it isn't downloading;
  - "Clip failed" tracks are downloaded again, and "not found" tracks are searched again, forgetting the remembered "not found";
  - "No clip" and "already in crate" aren't retried, since nothing on YouTube can change them;
  - a retry runs as part of Download all tracks, starting it if it isn't running.

Depends on `label-crates` (Download all tracks), archived on branch `label-downloads` (PR #38).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `preview-fetch`: a new requirement for YouTube limiting: recognised, waited out, told, and never counted as a track's failure.
- `label-crates`: a new requirement for Retry failed tracks, and Download all tracks' window shows a pause for YouTube.

## Impact

- **`crates/dig/src/preview/fetcher.rs`:** `FetchError::Limited(String)` from yt-dlp's error text, for fetch and search; unit tests on real error lines.
- **`crates/dig/src/preview/scheduler.rs`:**
  - the limited state with its backoff (10 → 20 → 40 → 60 min, reset on success);
  - `PreviewEvent::Limited { until }` and `PreviewEvent::Unlimited`;
  - `PreviewCommand::TryNow`;
  - `PreviewCommand::Retry { clips, searches }`, which forgets given-up clips and remembered "not found" results.
- **`crates/dig/src/preview/search.rs`:** forget a remembered result.
- **`crates/ui/src/app/digging.rs`, `app.rs`:** the message, the window's pause line and Try now, the "waiting for YouTube" tooltip, and Retry failed in the window and in the label menu.
- **Docs:** README (YouTube limits, Retry failed, test count) and help panel.
- No new dependency; playback, analysis and visuals are untouched.
