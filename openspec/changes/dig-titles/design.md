## Context

- The name shown for an entry comes from `Entry::display_name()` ("Artist - Title").
- The main window's title line is `format::title_line` plus `format::origin_details`. The details are " · side · catno · year · for sale".
- Tempo exists only inside a `SongScore` (`tempo_segments`, each with a `bpm()`). Scores are cached on disk by content hash (`ScoreCache`).
- Previews are prepared (scored) only for the playing entry and the next 3, so most dig entries have no score for a long time.

## Goals / Non-Goals

**Goals:**
- One name format everywhere: `(catno) Artist: Title (N BPM)`.
- BPM appears as soon as any analysis of that audio exists, and survives restarts.
- Nothing new is downloaded or analysed just to learn a BPM.

**Non-Goals:**
- Reading BPM from file tags (ID3 `TBPM` and similar). Analysis is the only source, so the number matches what SYNC/automix will use.
- Sorting or columns (that's `playlist-columns`).
- Musical key.

## Decisions

**1. The format is computed when the entry is drawn, not stored in the title.**
`artist`, `title` and `origin.catno` stay separate fields, and `display_name()` builds the string. Rejected alternative: writing the string into `title`. That would break the "tags don't overwrite an origin" rule, M3U export and later format changes.

**2. The BPM is a field on the entry (`bpm: Option<f32>`), saved with the crate.**
It is set whenever a score for the entry's audio becomes known:
- a preview is prepared (the prepare worker reports `(clip, bpm)`);
- the playing track's score is available (from the cache or finished);
- the metadata worker's cache lookup for local files finds a cached score. That lookup runs in the existing low-priority worker; it costs only the content hash, never an analysis.

Alternative: looking up `ScoreCache` on every frame. Rejected, because it means disk reads in the UI thread.

**3. A DJ-range fold into 88–176.**
The rule: `while bpm < 88 { bpm *= 2 }`, then `while bpm > 176 { bpm /= 2 }`, then round. This range keeps house and techno (118–150) and drum and bass (170–176) as they are, and puts dubstep and half-time hip-hop at their "double" (140 stays 140; 85 becomes 170). The raw analysed value stays in the score; only the display and the stored entry field are folded.

**4. Dominant tempo = the longest tempo segment.**
The helper is `SongScore::dominant_bpm()`. A track with no tempo segments has no BPM.

**5. Separators.**
- `Artist: Title` replaces `Artist - Title` for all entries, local ones included, so the list reads uniformly.
- The main-window line becomes `N. (catno) Artist: Title (N BPM) (m:ss) · side · year · for sale`.

## Risks / Trade-offs

- [Most entries in a big dig crate show no BPM for a long time] → accepted. BPM fills in as previews near the playhead are prepared, and nothing is fetched early (preview-fetch's horizon rule stays).
- [Half and double time are ambiguous near the fold edges, e.g. 87.9 vs 88] → the fold is applied after the dominant tempo is picked, and the edges are rare in electronica.
- [Old crates have no `bpm`] → `#[serde(default)]`, so the field is filled as scores appear.
- [Existing tests assert the "Artist - Title" format] → update them together with the spec scenarios.
