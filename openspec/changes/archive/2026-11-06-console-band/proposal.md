# Proposal

## Why

With SYNC on, the turntable takes 275 px of the band and leaves the waveform about 130 px: too
narrow to dig with. The band also jumps from 58 to 104 px when SYNC toggles, and the deck says
little about what SYNC is doing: a screenshot shows a record at 124 BPM playing at its own
tempo under a target of 102, with only `0.0%` and a lit OWN TEMPO lamp, and the target's lit
digits are hard to tell from the unlit ones. Making the turntable always present, in three equal
panels, fixes the waveform's width and gives SYNC a home of its own.

## What Changes

- **A window header** (15 px) across the top in the skin's title-bar style: DIGGR centred
  between groove lines, and at the right help (?), the Options gear, minimize and close. It
  replaces the button corner, which left three tiny buttons in a mostly empty column, so the band
  gets the full width. Dragging the header moves the window and double-clicking it maximizes or
  restores. It lights while the player has the keyboard.
- **The band is three panels of a third each**: the mini player, the turntable and the waveform,
  at least 227 skin px for the player and the turntable, at most 360, the waveform taking the
  rest. At the default window (720 skin px) that is 240 each.
- **The turntable is always shown**, and the band is always 76 px tall (no jump when SYNC
  toggles). The SYNC button leaves the player and sits on the turntable, lit while on. With SYNC
  off the deck is dimmed: the platter turns at the speed heard and the digits show the record's
  own BPM.
- **A compact turntable**: about 64 px platter, a shorter pitch fader, no title bar and no ×.
- **The band's panels have no title bars.** The window drags from any empty part of the band, and
  the focus light moves to the playlist's title bar alone.
- **SYNC says what it is doing**: an explanation line in place of the OWN TEMPO lamp ("LOCKED",
  "ANALYZING", "OWN · OUT OF ±8%"), a pitch meter showing how hard the record is pushed, a
  one-time flash when it locks, digits that roll from the record's BPM to the target on a new
  track, and unlit digit segments dimmed so the target reads at a glance.
- **The player's LCD gets a second line** for the playing record's label, side, year and
  for-sale summary, which no longer trail the scrolling track line.
- **BREAKING** (layout): the deck's × and the player's SYNC button are gone, the corner column
  is gone, and the header and band are 33 px taller than today's SYNC-off band, so the list shows
  22 rows at 720×435 instead of 25.
- Not in this change: merging the playlist's title bar into the filter bar (it would win back two
  rows and touches many specs), a cover in the player, a beat LED, native window decorations
  (the header is skinned) and window buttons at the left like macOS.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `player-window`: the window header; the band's three panels and their widths; the 76 px band;
  the player's second LCD line and its icons without SYNC; focus shown on the header and the
  playlist's title bar.
- `tempo-preview`: the turntable is always shown and holds the SYNC button; the dimmed state;
  the explanation line, pitch meter, lock flash and rolling digits.
- `waveform-view`: the waveform is the right panel, a third of the band, and 76 px tall.
- `playlist`: the row count under the header and the taller band.

## Impact

- `crates/ui/src/layout.rs`: `BAND_H`, `band_h`, `deck_h` (no longer depend on SYNC), a new
  function giving the three panels' x and widths.
- `crates/ui/src/app.rs`: `band_layout`, a new `header`, `mini_player`, `deck_section`,
  `window_buttons` (replaced by the header), `waveform_section`, the focus dimming, the strobe
  radius, new effect state.
- `crates/ui/src/skin/generate.rs`, `mod.rs` and `assets/skin/default/`: a stretchable, smaller
  deck (`PITCH_TOP`, `PITCH_LEN`, the platter), new `sync_*` button sprites, and dimmer unlit
  digit segments; regenerate with `cargo run -p ui --bin skin-gen`.
- `crates/ui/src/waveform.rs`: `HEIGHT` and the two rows' share of it.
- Tests: the band, SYNC and deck headless tests (`PITCH_UP`, `DECK_RANGE`, `SYNC`), the layout
  tests, and the mini-player fit tests.
- Docs: README (the layout tour and the SYNC section, the test count), CHANGELOG, the SYNC entry
  of the help panel.
- Related: the unstarted `beatmatch-automix` change assumes SYNC beside shuffle and repeat; it
  will need updating to the turntable.
