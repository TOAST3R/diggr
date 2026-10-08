## Context

- The default skin is drawn by code: `crates/ui/src/skin/generate.rs` paints `atlas.png` and writes `skin.ron` from about 17 palette constants (`PANEL_TOP`, `GOLD`, `LCD_ON`, …) and helpers like `titlebar` and `title_fill`. `cargo run -p ui --bin skin-gen` regenerates both files, and `skin::tests::committed_assets_match_the_generator` fails when they drift.
- The LCD green is in two places: in the atlas (digits, MONO/STEREO, lit button LEDs, visualizer bars) and in `app.rs`, which draws text in a hard-coded `color([0, 236, 0])` six times (title line, footer BPM control, filters, cart switch, footer info).
- `skin::Colors` already holds the playlist and visualizer colours, with serde defaults for fields added later (`pl_title`, `pl_owned`).
- The title line shows `now_playing_line()` or, with nothing loaded, `engine_status()` ("DIGGR - DROP FILES HERE"), scrolled by `format::scroll` with `title_offset`.
- Verdicts go through `DigAction` in `app/digging.rs` (`dig_want`, `dig_unwant`, `dig_pass`, `dig_collect`); their messages go to `notify()`, a tooltip that lasts 4 s.
- The playlist title bar is drawn from end caps plus a tiled middle, all stripes; `draw_crate_name` covers the middle of it with `pl_title_fill` and writes the name in `colors.pl_title`, which becomes amber.
- Main audience: DJs who play vinyl; secondary: DJs who play files.

## Goals / Non-Goals

**Goals:**
- A default skin that no longer reads as the old player: graphite and amber, groove lines, no gold stripes.
- Diggr's own actions visible on the player: an empty crate points at digging, and verdicts flash on the LCD.
- LCD colour owned by the skin, not the code.

**Non-Goals:**
- Logo, wordmark, app icon (`macos-release`).
- Moving or resizing any sprite or layout rectangle; the only additions are the waveform title bar's.
- A label behind the crate name on the playlist title bar: tried (cream card, dark ink) and dropped.
- A second, "classic" skin or a skin picker.
- Fullscreen visuals, waveform and spectrogram colours.

## Decisions

### 1. Palette

| Role | Now | New |
|---|---|---|
| Panel gradient | (62,66,96) → (34,36,56) | (52,46,40) → (28,25,22) warm graphite |
| Highlight / shadow | (120,126,166) / (14,14,24) | (104,94,82) / (10,9,8) |
| Title bar | (26,27,42) | (20,18,16) |
| LCD on / dim | (0,236,0) / (0,44,0) | (255,176,40) / (64,40,8) amber |
| Accent (caption, slider marks) | gold (236,204,90) | amber (255,176,40) |
| Buttons | blue-silver | neutral silver (186,180,170) → (110,104,96) |
| Playlist text / current | green / white | amber (240,164,40) / white |
| Visualizer low → high, peak | green → yellow, grey | dim amber (150,96,20) → amber (255,196,64), off-white |
| OWNED badge | amber (255,176,32) | off-white (240,236,226) |

EQ slider knobs lose the gold and become silver with an amber tick, so amber means "the display" and not "a part you grab". The values are a starting point; the task list has a look-and-tune step on the running app, with a screenshot of each window.

*Alternative:* red-orange "strobe" accent. Rejected for the LCD: a red screen reads as error or recording, and it is tiring over a long set. Amber matches the displays on decks and mixers.

### 2. `Colors::lcd`, defaulting to green

`Colors` gains `lcd: [u8; 3]` with `#[serde(default = "lcd_green")]` returning (0,236,0). The six `color([0, 236, 0])` in `app.rs` read `sk.def.colors.lcd`, and the two derived dims use `lerp_color(colors.lcd, …)`. A skin file written before this change keeps its green text; the default skin writes amber.

*Alternative:* derive it from the atlas (sample a digit pixel). Rejected: implicit, and wrong for skins with gradient digits.

### 3. Groove title bars

`titlebar` draws, either side of the caption, horizontal 1-pixel lines on every other row in two alternating dark tones (title background +18 and +30 in each channel), with the line nearest the caption one step brighter, like light catching a record's edge. No accent colour is used in the grooves, so the amber caption is the only bright thing on the bar. The playlist bar's caps and tiled middle use the same lines, so they still line up when stretched.

*Alternative:* concentric arcs. Rejected: arcs don't tile, and the playlist title bar stretches.

### 4. Empty crate hint

An empty crate (not the wantlist or collection crate, which a paste can't fill) shows "PASTE A DISCOGS LINK · CMD+V" over "OR DROP FILES", then after a gap "PRESS H FOR HELP" (H opens the help panel), centred in its list, in the skin font and `pl_text`, like the existing "NO TRACKS MATCH" line. `empty_crate_hint(modifier)` holds the wording; `PASTE_MODIFIER` is CMD on macOS, CTRL elsewhere. It is drawn with the list, so it costs nothing when idle and shows in the maximized layout too.

*Tried first:* the hint on the main window's title LCD whenever no track was loaded. It rarely showed (another crate's track usually stays loaded, and the maximized layout has no title LCD), and scrolling its 44 characters on a 25-character LCD would have broken "Low idle cost".

### 5. Waveform title bar

A `wave_title` sprite (275 × 14, `titlebar(…, "DIGGR WAVEFORM")`) and `wave_titlebar` / `wave_close` layout entries, matching the EQ's. `layout::wave_title_h(def)` is the sprite's height (0 for a skin without it), and `player_height` adds it when the waveform shows, so the window, the playlist's minimum rows and the drawing all agree. `wave_title` draws it, dims it out of focus, drags the window and closes the waveform (`Action::ToggleWaveform`). The maximized band has no title bar: it spans the playlist area, and its height stays the same.

### 6. Verdict flash

The app keeps `flash: Option<(String, Instant)>`. `dig_want`, `dig_unwant` and `dig_pass` set it after they have changed local state (records added, removed or passed), with the count when it is above 1; the paths that only notify (owned, already wanted, not from Discogs) don't set it. A collection add has no local change until Discogs answers, so OWNED flashes in `dig_collected` when the add succeeds, one record per answer. The title-line drawing checks the flash first: while it is younger than 1.5 s, the text is centred in `title_text` without scrolling, and a repaint is asked for when it ends so a paused player doesn't keep it; then it is cleared. A pure `flash_text(verdict, count) -> String` holds the wording and is unit-tested; headless dig tests check that the flash is set or not set per action.

The flash and the 4-second tooltip are separate: the tooltip still explains ("added to your wantlist", errors), and the flash is the at-a-glance confirmation on the LCD. Wantlist writes are asynchronous; the flash confirms the local change, and a failed write still reports through the tooltip as today.

*Alternative:* blink the flash. Rejected for now: a steady 1.5 s is enough, and blinking adds timing state for little gain.

## Risks / Trade-offs

- [Amber playlist text is less readable than green on black at small sizes] → The playlist amber is slightly darker than the LCD amber to cut glare; check legibility at 1× and 2× in the tuning task, and raise the luminance if needed.
- [OWNED off-white vs the white current-track text] → The badge is a filled box with dark text, the current track is plain text; the shape tells them apart. Checked in the tuning task.
- [Atlas diff is a large binary change] → It is generated; reviewers compare screenshots, and the sync test guarantees the atlas matches the code.
- [Users who dropped their own skin files in] → A missing `lcd` falls back to green, so their skins look as before.

## Migration Plan

None: the default skin is compiled in. Rollback is reverting the commit.

## Open Questions

- Exact amber values after seeing them on a real screen (decided in the tuning task, not before).
