## Context

- **How the visuals plug in:** through one seam. `ui` defines `VisualScene` and `SceneFrame` (`crates/ui/src/fullscreen.rs`) and hosts whatever scene `AppContext.scene` holds. `apps/native` passes `visuals::VisualEngine::new()`, and `render.rs` uses `visuals::render` for `--render-show`.
- **Dependency direction:** `visuals` depends on `ui`, never the other way. Deleting it touches only the seam and the binary.
- **What only exists inside fullscreen:**
  - the analysis debug strip (`timeline.rs`, key T);
  - annotation mode (A), which writes JSON to `<cache>/annotations`;
  - the host keys (`host_action`).
  - `timeline::kind_color` is also used by the waveform to tint sections.
- **What the player reads from `SongScore`:**
  - `tempo_segments` for BPM;
  - `beats` and `downbeats` for BPM, the waveform and navigation;
  - `sections` and `curves.energy` for structure navigation and the waveform.
- **What only the visuals and the debug strip read:**
  - `events` (kick, snare, hat);
  - `curves.bass`, `mid`, `treble`, `brightness`, `flatness` and `tension`;
  - `drop_in()`.
- **Inside the analyzer:** the drum onsets are also used to estimate tempo, and `phrase_origin` is used to build sections.
- **Startup cost:** `VisualEngine::new()` only stores a folder path, so removing it doesn't change launch time.

## Goals / Non-Goals

**Goals:**
- No visuals code, assets, specs, keys, docs or user folder left.
- The analysis computes and keeps only what the player reads.
- Everything for digging works exactly as before: BPM column and filter, waveform, spectrogram, structure keys, lossy-source check, previews, verdicts.

**Non-Goals:**
- Rewriting `beatmatch-automix` or `web-target`, which aren't started.
- Removing the user A/V offset setting from the playback clock (`av_offset_ms`). It shifts every clock reader, including the spectrum bars and the waveform playhead.
- Changing tempo, beat or section quality.

## Decisions

### D1. Delete, don't hide behind a feature flag
- **The decision:** the visuals crate and the fullscreen host are deleted outright. Git history and the archived `visual-engine` change keep the code and its reasoning.
- **Why not a cargo feature:** it would double the check matrix and rot unused. Reviving the visuals later is a revert plus an update to current egui.

### D2. Remove the seam, not just its implementation
- **What goes:** `VisualScene`, `SceneFrame`, `AppContext.scene`, the `fullscreen` state, `toggle_fullscreen`, `host_action`, `DIGGR_AUTO_FULLSCREEN`, and the cursor hiding.
- **Why the trait goes too:** a trait with no implementation is dead weight, and keeping it would invite rebuilding the feature.
- **Keys:** F and Esc-to-leave do nothing now; Esc keeps its other uses (closing the help, the search field). T and A go with the debug strip.
- **The section colours:** `timeline::kind_color` moves into `waveform.rs`, and `timeline.rs` is deleted.

### D3. Trim the score to what is read
- **What goes:** `SongScore` loses `events`, `BeatCurves`' bass, mid, treble, brightness, flatness and tension, and `tension()` / `drop_in()`. `curves` keeps `energy`.
- **What stays:** onset detection, because tempo estimation uses kick periodicity and hi-hat density. It just isn't copied into the score.
- **Dead code:** whatever the compiler then reports as unused (per-band curve computation, assembly of the removed fields) is deleted.
- **The cache:** `ALGORITHM_VERSION` goes from 1 to 2, so old cache entries are ignored and recomputed as tracks play. Saved crate tempos are untouched.
- **Why not keep the fields, since they're cheap:** they would still cost per-beat work on every prepared preview, and cache space, for no reader. The user asked for only what digging uses.

### D4. Analysis evaluation stays, capture goes
- **What stays:** `analysis-eval <dir>` keeps scoring beats and boundaries against annotation JSON files, which is how BPM and section quality are measured.
- **What goes:** in-app annotation (A in fullscreen). Existing annotation files, and any written by hand in the same format, still work.
- **`AppContext.annotations_dir`:** removed.

### D5. The visuals folder is deleted at launch
- **When and how:** after the first frame, on the background spawner, the app checks for `<config>/Diggr/visuals/` (honouring `DIGGR_CONFIG_DIR`) and removes it with `remove_dir_all`. It is one `metadata` call when the folder is absent.
- **Failures:** they are ignored, because there's nothing useful to say. No other path is ever touched.
- **Why at every launch, not once with a marker:** the check costs nothing, and a marker would be another file to manage.

### D6. Docs and project rules
- **AGENTS.md:** "What this is", the workspace table (no `visuals` row; `ui` no longer lists the fullscreen host), the data files line, the commands' test count, hard rule 4 removed, and hard rule 5 without "A/V offset < 1 frame".
- **`openspec/config.yaml`:** the product line and the constraints that mention visuals.
- **README:**
  - the hero and pitch;
  - the visuals, fullscreen, deck, scenes and render sections;
  - the key tables, `DIGGR_AUTO_FULLSCREEN`, `--render-show`;
  - the test areas and count;
  - the layout list.
- **The help panel:** the "Fullscreen visuals" group goes.

## Risks / Trade-offs

- **[Something for digging read a removed field]** → The compiler finds every reader once the fields are gone. Each is either visuals or debug strip (checked above) or must be kept. The headless tests for BPM, waveform and navigation must pass unchanged.
- **[Deleting a user's own scenes]** → The user chose this. Only that one folder is removed.
- **[Tempo quality depends on the onsets]** → They're kept as analyzer internals. The rhythm tests (kick within ±20 ms, tempo within 1 BPM on the synth set) still run against the analyzer.
- **[Recomputing cached scores]** → It happens once per track as it plays, at low priority, as for any new track.
- **[Size and launch time]** → Expected gains are small: about 1–2 MB of binary, and the same launch time. This is not a performance change, and it isn't sold as one.

## Migration Plan

- **Upgrade:** a user who upgrades loses F and fullscreen, their visuals folder is removed, and scores are recomputed as tracks play. Nothing else changes.
- **Rollback:** revert the change's commits.
