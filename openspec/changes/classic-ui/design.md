## Context

Winamp 2.x drew its UI by blitting regions of a few bitmaps (`main.bmp`, `eqmain.bmp`, `pledit.bmp`). We keep that model for speed and for the look (see user screenshot: three stacked sections, green LCD text, yellow EQ sliders). egui/eframe is chosen because it runs on native and web with wgpu, which the visual engine also needs.

## Goals / Non-Goals

**Goals:**
- Pixel-crisp classic look at 1×/2× scale, driven entirely by a skin atlas.
- Cold launch to interactive window < 300 ms on macOS; idle CPU ≈ 0%.
- All controls operate on the Engine without ever blocking it.
- One-key fullscreen entry into the visual engine.

**Non-Goals:**
- Loading third-party `.wsz` skins (future; the atlas format is designed to allow it).
- Separate, dockable OS windows per section (single window with collapsible sections instead).
- Media library, album art browser, tag editor.
- Fader deck / overlay (belong to `visual-engine`).

## Decisions

### D1. Single window, three stacked sections
One eframe window with Main (always), EQ (toggle), Playlist (toggle), in the classic order and 275 px base width. Alternative (three OS windows with snapping like Winamp) rejected for MVP: complex across platforms and irrelevant on web.

### D2. Skin = atlas PNG + RON sprite map
```
assets/skin/default/
  atlas.png          all sprites (buttons in normal/pressed states, digits, sliders, frames)
  skin.ron           sprite rects, widget layout rects, text colors, font glyph map
```
Widgets are custom egui widgets that paint atlas sub-rects via `egui::Image` UVs with nearest filtering. Layout coordinates are in skin pixels, multiplied by an integer scale (1× or 2×; auto 2× on HiDPI). This mirrors Winamp's approach and keeps a door open for a `.wsz` importer that converts to the same format.

### D3. Original art
We draw our own skin inspired by the classic layout and palette (dark panels, green LCD, gold sliders). Winamp's bitmaps are copyrighted and are not bundled.

### D4. Reactive repaint
egui repaints only on input or explicit requests. While playing, the UI requests repaint at 30 Hz for the time display / mini spectrum only when the main section is visible and the window is not occluded/minimized. Fullscreen visual mode drives its own frame loop (vsync).

### D5. Mini spectrum
19 bars + peak caps from the audio tap: realfft (1024 window) on samples aligned to the playback clock's audible position; log-spaced bins; falloff animation. Runs on the UI thread (cheap) at the UI repaint rate. Also an oscilloscope mode (click to toggle), as in the original.

### D6. Playlist model and persistence
`Playlist { entries: Vec<Entry{ path, title, artist, duration: Option<Duration>, status }> , current, selection }`. Durations are filled lazily by a low-priority worker that reads headers only. Persisted as RON in the platform config dir on change (debounced); M3U/M3U8 import/export. The playlist feeds the Engine queue (respecting shuffle/repeat) and informs pre-warm.

### D7. Keyboard map
Classic: Z prev, X play, C pause, V stop, B next, ←/→ seek ±5 s, ↑/↓ volume, Del remove, Ctrl/Cmd+O open, `F` toggle fullscreen visuals, Esc leave fullscreen. In fullscreen, all other keys are routed to the visual engine (mutate/keep/deck, etc.).

### D8. Fullscreen hosting
`F` switches the eframe viewport to borderless fullscreen and swaps the root UI to a `FullscreenHost` that gives the visual engine the wgpu device/queue and the full surface via an egui paint callback, then lets the visual engine draw its egui overlay/deck on top. Leaving restores the previous window size/position. The fullscreen request originates from a key event, which also satisfies the browser user-gesture requirement on web.

## Risks / Trade-offs

- [egui text/layout is not pixel-font native] → Bitmap font glyphs from the atlas for LCD/title text; egui text only in menus/dialogs.
- [Single-window stacking differs from original floating windows] → Accepted for MVP; layout is data-driven so floating can be added.
- [Lazy duration scanning of large folders] → Low-priority worker, header-only reads, capped concurrency.

## Open Questions

- EQ band labels to display (60/170/310 classic vs 70/180/320 from the screenshot) — defaulting to classic.
