# Design

## Context

See proposal.md for why. The code today:

- `band_layout` places the mini player (275 px, `def.main_size`), then the turntable only while
  `settings.sync.on` (`deck_section`, 275 px, 104 tall), then the waveform in what is left up to
  a `CORNER_W` = 39 corner. `layout::band_h` is `BAND_H` (58, from `waveform::HEIGHT`) or
  `deck_h` (104) while SYNC is on, and every playlist row count follows it.
- The deck is one generated sprite, `deck_bg`, 275×104: panel, a "DIGGR TURNTABLE" title bar, an
  83 px platter (rim, grooves, label), a tonearm, an LCD inset, the pitch fader's groove. The app
  draws over it: strobe dots at radius 37.5, the label mark, the digits, three text lines, the
  OWN TEMPO lamp sprite, the 8%/16% and RESET buttons and the fader's thumb.
- The mini player draws on a fixed 275 px grid: the LCD fill spans `w − 8`, but the icons, the
  volume (`mini_volume`, 108 px sprites), shuffle and repeat are placed from constants
  (`lcd_icons_x`, `SHUFFLE_X`) that assume 275.
- Section focus dims the deck's and the playlist's title bars (`dim_title`).

## Goals / Non-Goals

**Goals:**
- Three panels in thirds of the band, from 227 px (the default window) up to 360, with a pure,
  unit-tested function giving their x and width.
- A 76 px band that never changes height.
- The deck readable at a glance: the state, the push and the target.
- Nothing on the playback path changes. These are layout and draw changes only, and the
  animations cost nothing while paused.

**Non-Goals:**
- Merging the playlist's title bar into the filter bar (a separate change: it touches the
  `crates`, `crate-search`, `filter-bar`, `record-filters` and `discogs-write` specs).
- A cover in the player, a beat LED, or SYNC doing anything new. It is still every track at one
  tempo, vinyl-style.
- Title bars on the panels.

## Decisions

**One function for the panels.** `layout::band_panels(win_w, corner) -> Panels { player, deck,
wave }` (each an x and a width, in skin px): `third = (win_w − corner) / 3` rounded down;
`player = deck = clamp(third, 227, 360)`; `wave` is the rest, never negative. `band_layout`,
`window_buttons` and the tests all read it, so the thirds are defined once. *Alternative:*
centring the deck and stretching the player: at 1,500 px the player would be 584 wide, with
nothing useful to put in it.

**The band is a constant height.** `BAND_H` = 76 (`waveform::HEIGHT`), and `band_h`, `deck_h` no
longer look at `settings.sync`. The playlist's rows follow (23 at 720×435). The 76 is the
deck's content (a 64 px platter plus 6 above and below) and gives the waveform 18 px more than
today. *Alternative:* 90 px, keeping the 83 px platter: two rows fewer.

**The deck's content is one 227×76 sprite, centred.** `deck_bg` is redrawn at 227×76 without a
title bar: a 64 px platter (r 32; the rim between 26 and 31, the strobe dots at the rim's middle,
r 28.5, so `STROBE_DOTS` stays 24), a shorter tonearm whose pivot sits at (74, 10), the LCD inset,
and a fader groove 56 px long (`PITCH_LEN`, `PITCH_TOP` 8) with its ticks and no "PITCH" label.
A 1×76 `deck_fill` and the 2 px bevel `deck_edge` (the same gradient `panel()` draws) tile the
rest, so a panel wider than 227 shows the content centred between two fills. Layout entries
(`deck_platter`, `deck_target`, `deck_push`, `deck_record`, `deck_state`, `deck_range`,
`deck_reset`, `deck_sync`, `deck_pitch`) are relative to the content, which the app offsets by
`(panel − 227) / 2`.

Vertical budget in the 76 px: digits 5–18; push line 22; record line 31; the state line, a
12 px inset replacing the OWN TEMPO lamp sprite, 43–55; the buttons (SYNC 30, 8% 30, RESET 42, with
4 px gaps, 110 px) 58–71. Horizontally: platter 8–72, readouts 80–194 (19 characters), fader
200–220. The longest state string, "OWN · OUT OF ±8%", is 16 characters (96 px).

**SYNC moves onto the deck, drawn over the dimming.** `deck_section` runs every frame. With SYNC
off it draws the same content, then a translucent black rect over everything but the SYNC button,
so it reads as unlit; the platter still turns and the digits show `playing_bpm()`. The fader, 8%/16%
and RESET are not wired up while off (`sync.on` gates their actions, as `pending` already gates the
fader). The sprites for the button are the existing `sync_on` / `sync_off`. The deck's ×,
`deck_titlebar` and `own_tempo_*` go.

**The player is parametrized by its width.** `mini_player` takes the panel width in place of
`def.main_size.0`: the LCD fill, `lcd_icons_x(w)` (now three icons: `w − 4 − 3 · 19 + 2`), the
volume rectangle (from x 123 to `SHUFFLE_X(w)`, at least 60 px) and shuffle and repeat (ending 4 px
from the edge) are computed from it. The volume's track becomes three pieces (`volume_track_l`, `_m`, `_r`) so it stretches without
distorting the groove's edges, and the fill sprite is stretched over the whole track and clipped
to the thumb's position, so the gradient keeps its look at any width; `hslider` draws both when it
finds the pieces. At 227 the track line keeps 15 characters.

**The LCD's second line.** The LCD inset grows to 38 px (the band is 76): row one is the play
state, the time digits and the scrolling track line; row two, under the digits, is
`now_playing_detail()`: for an entry with an `Origin`, `label · side · year · for-sale summary`
(the pieces the scrolling line used to append, plus `origin.label`), else the album tag. It
scrolls with the same `title_offset`. The verdict flash takes row one only.

**The state is a pure function.** `SyncSettings::fit(bpm, analyzing, playing)` gives a
`Fit` (Off, Idle, Analyzing, Locked(speed), Out(raw speed), NoBpm) and `Fit::line(range)` its text;
the deck draws from that, and both are unit-tested. "SYNC ON" (Idle) is the line while SYNC is on
and nothing plays.

**Effects are small state, kept on the app.** Four fields: `deck_flash: Option<Instant>`,
`deck_count: Option<(from: u16, to: u16, since: Instant)>`, the last `(entry, locked)` pair to
detect "becomes fitted", and `deck_new`, set when a different track starts and cleared when its
fit is known: a track whose tempo is still being read, or whose clock hasn't started, flashes and
counts once it locks, and one that doesn't fit never does. The
flash lights the platter's rim for 0.6 s, easing out; the count interpolates the shown digits
over 0.5 s. Both call `request_repaint_after` only for the time they have left, and the existing
`Activity` policy already repaints at 16–33 ms while playing, so nothing new runs when paused.
`deck_strobe` already drifts by `record/target − 1`; "analyzing" adds a chase (the dots'
brightness rotates) driven by `deck_turn`, and own tempo draws the lamp red.

**The pitch meter is not the fader.** The fader's thumb already moves with the push, but with
an out-of-range record it sits at the centre, which hides the mismatch the screenshot showed.
The meter is a 60×5 px scale under the digits with a needle at `push / range`; out of range it is
pinned to the end and drawn red. *Alternative:* only recolouring the thumb: it doesn't say
which way.

**Digit contrast.** `seven_seg` draws the unlit segments in a new `LCD_GHOST` (26, 17, 4), a tenth
of the lit amber's luminance; the old dim amber was already a quarter, which is why the digits
still read poorly. The sprites are shared with the mini player's time display, which gets the
same cleaner look. A skin test measures the ratio.

**Drag and focus.** The band registers one drag-sensing rect over its whole area, created before
the panels so the controls drawn after it come first, replacing `titlebar_drag` on the deck and the
corner's own rect; the player's right-click Options area keeps its secondary click. A control that
senses drags (the pitch fader, the waveform) keeps them; a click-only button, as in the old corner,
doesn't stop a drag that starts on it from moving the window, which is accepted. With no band
title bars, `dim_title` runs for the playlist only: lit while it has focus.

**The window header replaces the corner.** `layout::HEADER_H` = 15 and `TOP_H` = header + band =
91 (what the playlist and the EQ's floating position sit under); `band_panels(win_w)` takes no
corner, so the thirds use the whole width. `DiggrApp::header` draws, stretched to the window
width, a left cap, a tiled grooved column and a right cap (`hdr_l`, `hdr_fill`, `hdr_r`, drawn by
`skin-gen` with the same grooves as `titlebar()`), then the caption DIGGR in the accent colour on a
plain patch that cuts the grooves, centred as `draw_crate_name` centres the crate's. The four
buttons are 11×11 at the right (`hb_help`, `hb_opts`, `hb_min`, `hb_close`, 2 px apart, 4 px
from the edge) at y 2, each through `corner_button`, which counts a click only inside the button.
The gear's slot stays empty without digging, as before. The gear is redrawn as a 7×7 cog with a
hole; `?` is a new glyph. A drag-and-click area under the buttons starts a window drag and, on a
double-click, sends `ViewportCommand::Maximized(!maximized)`. While the playlist has focus the
header gets the same translucent dimming the playlist's title bar gets when the player has it.
The skin font gains a `±`, so "OWN · OUT OF ±8%" draws whole. *Alternative:* native decorations,
rejected for clashing with the pixel skin and differing per platform.

## Risks / Trade-offs

- [The list loses two rows, 25 → 23 at 720×435] → The deferred title-bar merge wins them back;
  the proposal says so. A user who never uses SYNC still gets the taller band.
- [The platter at 64 px is less detailed] → It is generated, so the radius is a constant; the
  grooves keep their 3 px period and the label shrinks with it.
- [The 227 px minimum doesn't fit a window narrower than 720] → The waveform takes what is left
  and goes to nothing; the two other panels keep their size. Narrow windows were already cramped.
- [A drag area under every control can start a window drag from a missed click] → Controls sit on
  top, and the area only moves the window on a real drag, as the title bars did.
- [The shared digit sprites change the time display too] → Intended; checked in the tests.
- [The header costs a row, 23 → 22 at 720×435] → The deferred crate-name merge wins two back.
- [Many headless tests hard-code the band] → Most use `BAND_H`; the ones that use the deck's
  numbers (`PITCH_UP`, `DECK_RANGE`, `SYNC`, `DECK_CLOSE`) are rewritten from the layout.

## Migration Plan

No data changes: `sync` keeps its fields and settings files load as they are. The skin's deck
sprites and layout entries change, and a skin file that lacks the new required names fails
validation, which already falls back to the bundled skin (data files fail soft). Rolling back
restores the 58/104 band.

## Open Questions

- Whether the platter's tonearm should sweep with the track's progress instead of staying still
  (a nice cue, but it is a new behaviour for another change).
