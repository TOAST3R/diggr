## 1. Analysis with progress

- [x] 1.1 `analysis::eval::analyze_file_with(files, track, progress)` reporting the share decoded (nothing when the duration is unknown); `analyze_file` uses it; unit test on a fixture: progress rises to 1.0 and the score matches `analyze_file`'s

## 2. The preparation worker

- [x] 2.1 `PrepareHandle::progress()` with the current track and share, behind a `Mutex`; the worker wakes the UI at most every 0.5 s while analyzing; test: progress is seen for a fixture and cleared after its `Prepared`
- [x] 2.2 Test that the latest list's first unprepared track goes next (a track put at the front while another is in progress comes right after it)

## 3. The full list (UI)

- [x] 3.1 `dig_horizon` builds the list: playing track (score not complete), `asked`, near window, the rest of the playing crate's playable clips, then the shown crate's; deduplicated; the playing track no longer left out
- [x] 3.2 The gate also opens at 15 s of playback
- [x] 3.3 `Dig::analyzed` from `Prepared`, `asked` cleared as tracks finish
- [x] 3.4 Headless tests: background-downloaded previews get their BPM without playing; a played-and-skipped track gets its BPM; the playing track is next in the list

## 4. Progress in the BPM column

- [x] 4.1 BPM cell: a bar for the track in progress, an empty bar frame for a queued entry with no BPM; without the column, in a slot before the row's time (a long name hid it)
- [x] 4.2 Tooltip: "analyzing N%" / "BPM queued"
- [x] 4.3 Tests for queued / analyzing / done states

## 5. Analyze BPM

- [x] 5.1 Entry menu item for selections with an unanalyzed playable entry (clip or local file), appending to `asked`
- [x] 5.2 Headless test: local files asked for are analyzed first and get their BPM; the item is absent for an entry waiting for its download

## 6. SYNC waiting

- [x] 6.1 `SyncSettings.pending` (not saved); `toggle_sync` sets it when the playing BPM is unknown; the per-frame take-over and the clear conditions (track change, stop, no tempo found)
- [x] 6.2 `skin-gen`: `digit_q`, regenerate the skin; the deck shows "  ?" and "REC ?" while pending
- [x] 6.3 Fader, scroll, Shift+↑/↓ and RESET leave the target and show "Analyzing the BPM, wait a moment" while pending
- [x] 6.4 Headless tests: turn on before the BPM is known → "?" then the analyzed tempo; fader while pending changes nothing and shows the message; another track starting ends the wait with the old target

## 7. Docs and checks

- [x] 7.1 README (BPM analysis, the column's progress, Analyze BPM, SYNC waiting, test areas and count) and help (Analyze BPM in the entry menu)
- [x] 7.2 `cargo test --workspace`, clippy `-D warnings`, `cargo fmt --all --check`, the wasm check; `diggr --bench` shows 0 underruns
- [x] 7.3 Live: dig a label crate with "Download all tracks" and watch BPMs fill; play a track straight away with SYNC on and see "?" turn into its tempo
- [x] 7.4 Found live: drum & bass read at 2/3 of its tempo (Speedball 174 → 116), because the tempo prior weighed every candidate. The strongest periodicity now picks the tempo family and the prior only its octave; a synth two-step break test; `ALGORITHM_VERSION` 3 so cached scores are redone
- [x] 7.5 Found live: the analysis thread died on a file with a stretch where nothing repeats (no tempo candidate scored, so a 0 BPM lag overflowed), and every BPM after it stayed queued. `rhythm::tempo` gives no tempo there; the worker passes a file whose analysis panics and carries on
- [x] 7.6 Asked live: up to 3 workers on one shared queue (progress per track, so several bars move); Analyze BPM says it's queued and then what each track gave
