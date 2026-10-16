## Why

When YouTube can't be reached (no network, or a proxy or VPN intercepting secure connections), every yt-dlp run fails with a connection or certificate error. The app counts that as a broken clip, so after two tries each entry becomes "clip failed", and it stays dead after the network comes back. Double-clicking it only repeats "clip failed". On an intercepted network a whole crate turns into dead entries, and nothing says the real cause.

## What Changes

- **Unreachable is not a failure:** a yt-dlp or Juno error that means the source can't be reached (connection refused, DNS failure, no network, certificate verify failed) no longer counts as a failed try. The track keeps waiting, no "clip failed" or "not found" is recorded, and the source pauses like a limited one, with a shorter wait: 1, 2, 4, 8, then at most 10 minutes. Once a request goes through, it carries on.
- **Saying why:** the main window says once per period "Can't reach YouTube: trying again in ‹N› min". When the cause is a certificate error, it adds that a proxy, VPN or security tool may be intercepting secure connections. Waiting entries' tooltips say "waiting for YouTube", as when limited.
- **Double-click retries a dead entry:** double-clicking a "clip failed" or "not found" entry, in any crate, puts it back to waiting (download again, or search again), retries it at once and arms it. A "no clip" entry still only says why.
- **No "(none)" catalogue number:** Discogs writes `none` for a release with no catalogue number. Entries no longer show it as "(none) ".

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `preview-fetch`: a new requirement for an unreachable source (not a failure, paused with its own wait and message).
- `playlist`: the row's "(catno) " leaves out `none`, and double-clicking a "clip failed" or "not found" entry retries and arms it.

## Impact

- **`crates/dig/src/preview/fetcher.rs`:** `FetchError::Unreachable`, and a classifier for yt-dlp's error output beside `limited()`.
- **`crates/dig/src/preview/sample.rs`:** Juno's transport errors (no connection, DNS, TLS, timeout) become `Unreachable`.
- **`crates/dig/src/preview/scheduler.rs`:** downloads, searches and Bandcamp reads treat `Unreachable` like `Limited`, with their own wait, and a new event with the reason.
- **`crates/ui/src/app/digging.rs`, `app.rs`:** the main window message, and `play_entry` retrying a dead entry.
- **`crates/ui/src/format.rs`:** `entry_name` drops a `none` catalogue number.
- **README:** the unreachable message and the double-click retry; test count.
- **No change** to `audio`, `analysis`, `visuals`, or the Discogs client.
