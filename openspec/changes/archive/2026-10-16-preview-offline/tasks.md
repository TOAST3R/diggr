## 1. Unreachable errors (`dig::preview`)

- [x] 1.1 Add `FetchError::Unreachable(String)` and `unreachable(errors)` beside `limited()` (D1 patterns); `failure()` checks limited, then unreachable, then `Failed`; verify with unit tests: the certificate, DNS, refused and reset lines are unreachable, and "HTTP Error 404", "Video unavailable", a 429 and our "timed out" are not
- [x] 1.2 Map Juno's ureq transport errors (`Io`, `Tls`, `ConnectionFailed`, `HostNotFound`, `Timeout`) to `Unreachable` in `sample.rs`; verify with a test against an address that refuses connections
- [x] 1.3 In the scheduler, give `Limit` a kind and an `unreachable_wait` (1 min → 2 → 4 → 8 → max 10); downloads, searches and Bandcamp reads treat `Unreachable` like `Limited` (requeued, no failed try, no `search_failed`); `PreviewEvent::Limited` carries `reason: Option<String>`; verify with scheduler tests using a fake fetcher: no "clip failed" after many unreachable tries, the waits double up to 10 min, the other sources go on, a success resumes and resets the wait, and a 404 still gives up after two

## 2. In the player (`ui`)

- [x] 2.1 `limited_text` with the reason: "Can't reach ‹source›: trying again in ‹N› min", plus the interception hint for certificate errors; tooltips "waiting for ‹source›"; verify with a headless test that a certificate error shows the message once and leaves the entries waiting
- [x] 2.2 Factor `retryable()`'s per-entry step into one function; in `play_entry`, a "clip failed" or "not found" entry goes back to waiting, is retried with `PreviewCommand::Retry` for itself and armed; "no clip" only notifies; verify with headless tests (clip failed in a non-label crate retries and plays, not found re-searches, no clip sends nothing)
- [x] 2.3 `dig_horizon` takes the armed entry's search whichever crate it is in; verify with a headless test: playing crate A, double-click a search-waiting track in crate B, and its search is sent first
- [x] 2.4 `entry_name` treats a `none` catalogue number (any case, trimmed) as empty; verify with a unit test ("(none) " gone, "NONE" too, "LT-012" kept)
- [x] 2.5 Keep the pause visible: an armed entry waiting on a paused source says why in the status line ("Waiting for ‹entry›. Can't reach YouTube: trying again in 1 min…"), and the status line keeps saying it once the message expires; verify in the headless test of 2.1 (found while testing behind Zscaler: the armed line hid the message)

## 3. Docs and checks

- [x] 3.1 README: the unreachable message and waits next to YouTube limiting, double-click retrying a dead entry, the test count
- [x] 3.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check
- [x] 3.3 On this Mac with Zscaler on: send a label, confirm entries wait with "Can't reach YouTube" and the interception hint and none become "clip failed"; with Zscaler off, confirm they download within the next try, and that double-clicking an old "clip failed" entry plays it
