## Why

Many entries never get a BPM, so SYNC can't fit them. Whole-file analysis ("preparing") runs only for the 3 downloaded previews after the playhead. Everything else waits until it is played to within 2 minutes of its end without a seek:
- the playing track itself;
- a track started by double-click outside that window;
- tracks downloaded by "Download all tracks";
- entries behind the playhead.

That happened with a 170 BPM track the user played straight away: it had no BPM, so SYNC kept its old 112 target. Nothing shows that analysis is running or waiting either, and there's no way to ask for it.

## What Changes

- **Every downloaded clip is analyzed:** each downloaded preview in the playing and shown crates gets one whole-file analysis in the background. The order is:
  1. the playing track;
  2. entries the user asked for;
  3. the next tracks to play;
  4. the rest, in crate order.

  Up to 3 files at a time, at low priority, behind the existing gate that keeps it off a track start. Nothing is downloaded only to learn a tempo.
- **The playing track is analyzed in full too**, so seeking or skipping no longer leaves it without a BPM. The BPM column shows only the final value, never a provisional one.
- **Progress in the BPM column:** a bar fills while a track is analyzed; a queued track shows the bar empty. The tooltip says "analyzing 40%" or "BPM queued".
- **Right-click › Analyze BPM** on entries, local files included, moves them to the front of the queue.
- **SYNC without a BPM:** turning SYNC on before the playing track's BPM is known shows "?" for the tempo and record BPM. SYNC takes the track's tempo once the analysis gives it. Moving the pitch fader, scrolling or Shift+↑/↓ meanwhile changes nothing and says "Analyzing the BPM, wait a moment".

## Capabilities

### New Capabilities
- `bpm-analysis`: the background queue that analyzes downloaded and requested audio for its tempo, its order, its progress in the BPM column, and Analyze BPM.

### Modified Capabilities
- `playlist`: Entry tempo is now known from the background analysis of every downloaded clip, not only from previews near the playhead.
- `tempo-preview`: SYNC control waits for an unknown BPM ("?"), takes it once known, and refuses fader moves meanwhile with a message.

## Impact

- `crates/dig/src/prepare.rs`:
  - the worker takes an ordered list of every track to analyze;
  - it reports progress (current track and share done);
  - the gate also opens after 15 s of playback, so a beatless track can't hold it shut.
- `crates/analysis/src/eval.rs`: `analyze_file` gains a variant that reports progress.
- `crates/ui/src/app/digging.rs`: builds the full list (playing track first, then asked-for entries, then the near window, then the rest) and keeps the asked-for set.
- `crates/ui/src/app.rs`: the BPM column draws progress and the queue; the entry menu gains Analyze BPM; `toggle_sync`, `deck_section` and the fader and nudge handlers handle a pending BPM.
- `crates/ui/src/sync.rs`: a pending state.
- Skin: a "?" digit sprite from `skin-gen`.
- README and help.
- Playback is unaffected: analysis stays on low-priority threads, up to 3 files at a time, gated.
