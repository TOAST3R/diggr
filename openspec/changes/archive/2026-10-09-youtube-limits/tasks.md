## 1. Recognising a limit

- [x] 1.1 `FetchError::Limited(String)`; `limited(stderr)` over the whole error text (429, too many requests, confirm you're/you’re not a bot, rate-limit); `fetch` and `search` use it; unit tests with real yt-dlp lines, and "Video unavailable" / "Private video" staying `Failed`

## 2. The worker waits it out

- [x] 2.1 Scheduler: `limited_until` and the backoff (10 → 20 → 40 → 60 min, `Config.limited_wait` for tests); a limited download or search changes nothing about the track; nothing starts while limited; the probe after the wait; `PreviewEvent::Limited { until }` / `Unlimited`; `PreviewCommand::TryNow`; tests in `crates/dig/tests/previews.rs` with a fake fetcher that answers limited (no "clip failed", nothing starts, waits double, success resets, Try now)
- [x] 2.2 `PreviewCommand::Retry { clips, searches }`: forgets given-up clips, failures, remembered results and failed searches for those keys; `Searches::forget`; tests

## 3. The app

- [x] 3.1 `limited_until` from the events; the main window message once per limited period; "waiting for YouTube" in waiting entries' tooltips; Download all tracks' window shows the pause line with Try now (and shows itself if hidden); headless tests (a limited event pauses and shows the window, Try now sends the command, Unlimited clears)
- [x] 3.2 Retry failed: `retryable(crate)`; Retry failed (N) in the window; Retry failed tracks (N) in the label menu when not downloading; entries back to waiting, `Retry` sent, Download all tracks started; headless tests (clip failed and not found retried, no clip and already in crate left alone, the counts)

## 4. Docs and checks

- [x] 4.1 README (YouTube limits, Retry failed, test count) and help panel; run `cargo test --workspace`, clippy, fmt and the wasm check
