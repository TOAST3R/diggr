## Context

An entry gets its tempo three ways today:
1. `dig::prepare` analyzes the whole file of the 3 downloaded previews after the playhead (`digging.rs`, `scheduler::AHEAD`). It does so behind a gate that opens once the playing track's own analysis has 32 bars.
2. `show_playing_bpm` takes the playing track's tempo once its playhead analysis is *complete*. That analysis keeps 120 s ahead of the playhead, so it completes only when the track plays to within 2 minutes of its end without a seek leaving a gap.
3. `MetaWorker` reads a tempo from the score cache.

The playing track is left out of (1) on purpose ("the analyzer is on it already"). As a result, a track started directly, skipped or seeked never gets a BPM, and neither does anything downloaded by "Download all tracks" or sitting behind the playhead. With SYNC, `toggle_sync` keeps the old target when the playing BPM is unknown, so a 170 BPM track met a 112 target and played at its own tempo.

## Goals / Non-Goals

**Goals:**
- Every downloaded preview gets a BPM without being played, and the playing track always does.
- Show that analysis is running or waiting, and let the user ask for it.
- SYNC waits for an unknown BPM instead of keeping a stale target.

**Non-Goals:**
- Analyzing local files automatically. A folder of 500 files would keep a core busy for an hour, so files are analyzed only when asked or played.
- A provisional BPM from part of a file. The user chose the final value only, since a wrong BPM would make SYNC jump.
- Changing the playhead analyzer, its horizon or `AnalysisService`.

## Decisions

### D1: Grow `dig::prepare`, not `AnalysisService`
Preparation already analyzes whole files one at a time at low priority, caches the score and overview, skips cached files, waits on the gate and reports the tempo. Making it cover every downloaded clip changes only the list it is given and how it reports. (Found live: it then had far more to do, so it now runs up to 3 workers on one shared queue, each claiming the first file nobody has started.)
- Rejected: teaching `AnalysisService` a whole-file mode. It is built around a playhead and an idle stop, and it is on the playback-adjacent path the latency rules protect.
- Preparation is started whenever there is a cache folder, which the native app always has.

### D2: One ordered list, rebuilt by the UI
The UI keeps sending the full list. The worker already treats the latest list as the queue and takes its first unprepared track. `dig_horizon` builds the list in this order:
1. the playing track (when its score isn't complete);
2. `asked` (Analyze BPM), in the order asked;
3. the existing near window;
4. every other playable clip in the playing crate, then the shown crate.

It is deduplicated and sent only when it changes, as now. Including the playing track duplicates the playhead analyzer's decoding, but only on the low-priority thread and only after the gate. If the playhead score completes first, the cache check skips the file.

*Shortcut:* the list is rebuilt every frame from the crates, which is O(entries). That is fine for crates of a few thousand. Upgrade to a dirty flag if profiling shows it.

### D3: Progress
- `analysis::eval::analyze_file_with(files, track, progress: impl FnMut(f32))` reports the share done after each chunk, as decoded seconds over `Source::duration()`, and reports nothing when the duration is unknown. `analyze_file` calls it with a no-op.
- `PrepareHandle` holds the current `(TrackRef, f32)` behind a `Mutex`. That is UI and worker only, never the audio thread. `PrepareHandle::progress()` reads it. The worker wakes the UI at most every 0.5 s while analyzing, so the bar moves without a repaint per chunk.

### D4: Queued and done, on the UI side
`Dig` keeps:
- `analyzed: HashSet<TrackRef>`, filled from `Prepared` events;
- `prepared_list` (already there).

An entry is *queued* when its track is in `prepared_list`, isn't in `analyzed`, isn't the one in progress, and the entry has no BPM. A store BPM counts as known, so Beatport rows don't show an empty bar. The BPM cell, and without the column a slot before the row's time, draw from that, along with the tooltip wording.

### D5: Analyze BPM
- The entry menu item is shown when the selection (or the entry) has a playable entry that hasn't been analyzed: a downloaded clip, or a local file.
- It appends those tracks to `Dig::asked`, which keeps them in order without repeats; a track leaves `asked` when its `Prepared` arrives.
- Local files go through the same worker. It takes any `TrackRef`, and `FileSource` reads files and clips alike.

### D6: The gate opens after 15 s of playback too
The gate is `!playing || complete || downbeats ≥ 32 || position ≥ 15 s`. A beatless track never reaches 32 bars, so before this change it held preparation shut until it completed. Fifteen seconds still keeps preparation well clear of a track start, which is the gate's purpose.

### D7: SYNC waiting
- `SyncSettings` gains `pending: bool`, marked `#[serde(skip)]` so it is never saved.
- `toggle_sync`: if a track plays and `playing_bpm()` is None, it sets `pending` and puts that track at the front of `asked` (after D2's playing slot, so in practice it's already first).
- Each frame in `update`:
  - if `pending` and `playing_bpm()` is Some, take it as the target and clear `pending`;
  - clear `pending`, keeping the target, when the playing track changes, playback stops, or a `Prepared` for the playing track arrives with no tempo.
- `deck_section` draws "  ?" in the big digits and "REC ?" while `pending`.
- The fader, scroll, the Shift+↑/↓ nudge and RESET check `pending` first. If set, they leave the target alone and show "Analyzing the BPM, wait a moment" through `notify`.
- `apply_sync` needs no change: an unknown BPM already plays at speed 1.

### D8: "?" digit
`skin-gen` draws a `digit_q` sprite in the same 7-segment style. It is added to the sprite list, and the generated atlas and RON are regenerated. The skin test checks them.

## Risks / Trade-offs

- **CPU while digging:** a 300-track label crate means about 300 whole-file analyses, at roughly 1–2 s of one low-priority core per minute of audio. It runs only while the gate is open, and it is cached, so it happens once per file.
- **Duplicate work on the playing track:** the playhead analyzer and preparation both decode it. This is accepted for correctness; it is low priority and happens once.
- **Showing a BPM late:** the final-value-only rule means a 7-minute track shows an empty bar, then the bar filling, for a few seconds before its BPM. This is the user's choice.

## Migration Plan

None. New fields are not saved, or default empty. Cached scores stay valid.

## Open Questions

None.
