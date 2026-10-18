## Context

The main section (275×116) is drawn from the generated skin: `crates/ui/src/skin/generate.rs`
paints every sprite in code and places it with `layout()`, and `skin-gen` writes
`assets/skin/default/{atlas.png,skin.ron}`, which a test keeps in step with the code. The lower
half today:

```
 y=38  kbps   khz            [MONO][STEREO]
 y=57  [====volume====] [ WAVE ][EQ][PL]                     177..266
 y=88  |◀ ▶ ❚❚ ■ ▶|  [⏏] [SHUFFLE][REP][SYNC]                 136..268
```

## Goals / Non-Goals

**Goals:** fewer, clearer controls; icons for the toggles, shuffle and repeat; SYNC with the
other toggles.

**Non-Goals:** the Options menu (right-click and gear stay as they are); a native macOS menu;
using the space the indicators free at y=38; any change to the mini player or the turntable.

## Decisions

**Layout.**

```
 y=38  kbps   khz
 y=57  [====volume====]  [∿][≡][☰][SYNC]                     toggles: 177..268
 y=88  |◀ ▶ ❚❚ ■ ▶|                         [⤮][⟲]           right-aligned to 268
```

The three icon toggles are about 17×12 each, SYNC keeps its 26 px label, all at the toggles'
12 px height. Shuffle and repeat are about 22×15, the transport row's button height, right-aligned
under the seek bar's end. Exact pixels are settled when drawing; the sizes above fit the room.

**Icons are pixel art in `generate.rs`, on the existing `button_face`.** Same as every other
sprite: no image files, no fonts. Lit states reuse the amber the text toggles use, so on/off reads
the same as today. Alternative: Unicode glyphs from the UI font, rejected because the skin draws
at integer scale with its own pixel font, and a glyph would blur and break the look.

**SYNC keeps its text sprite**, re-placed into the toggle row. Its 12 px height means a new
`sync_*` sprite at 12 px (from the same `text_button`) rather than the transport's 15 px one.

**Eject goes with its open mode.** `Action::Eject` and `Open::Replace` are removed; the command
line keeps replacing the Playlist crate through `replace_playlist` in the constructor, which
doesn't use `Open`. Alternative: keep Replace behind a shortcut, rejected (the user chose to
drop it).

**Tooltips** name each icon ("Waveform (W)", "Equalizer", "Playlist", "Shuffle", "Repeat: off /
all / one"), since a picture is less obvious than a word.

## Risks / Trade-offs

- [Icons at 12–15 px are hard to read at 1×] → keep shapes bold (2 px strokes where they fit),
  check at 1× and 2×, and rely on tooltips.
- [Users who used Eject to start over lose that] → Clear crate plus Cmd+O does the
  same in two steps; the README says so.
- [Settings or saved layouts naming removed sprites] → sprites are looked up by name from the
  generated `skin.ron` only; nothing persisted names them.
