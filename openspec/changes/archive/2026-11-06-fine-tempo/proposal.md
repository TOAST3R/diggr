# Proposal

## Why

SYNC's tempo is a whole number of BPM, so a set can't sit at 126.35, and the pitch fader, 56
skin pixels for ±8%, moves about 0.3 BPM per pixel: matching two records by ear needs finer
steps than either gives. DJs read and set tempo to two decimals.

## What Changes

- **The target is kept to 0.01 BPM.** The big digits show its whole BPM and two small
  decimals beside them (`126` `.35`); messages show up to two decimals, trailing zeros dropped
  ("126.35 BPM", "124 BPM").
- **Fine steps.** Shift+drag on the pitch fader moves the target by 0.01 BPM per skin pixel from
  where it was grabbed; Shift+scroll over it steps 0.01 BPM; Alt+↑ / Alt+↓ (Option on macOS),
  with the player focused, step 0.01 BPM; while SYNC is on, plain ↑ / ↓ step 0.1 BPM instead of
  the volume (which keeps its slider). A plain drag, plain scroll and Shift+↑ / Shift+↓ keep their
  coarse steps.
- **Typing the tempo.** Clicking the big digits opens them for typing: digits and a point, Enter
  applies (kept within the playing record's range), Esc or a click elsewhere cancels, and no
  other shortcut acts meanwhile.
- **The plain fader** sets the target to the nearest 0.01 BPM instead of a whole BPM, so pulling
  a 123 BPM record to the top of ±8% gives 132.84 instead of 132.
- **A spike** measures how accurate the analysed BPM of a record is, to decide in a later change
  whether records may show decimals too. Records keep whole BPM here.
- Unrelated, carried here: a 500-track Beatport send's time budget goes from 50 ms to 200 ms. The
  send takes about 11 ms alone, but its test ran in a parallel suite where 50 ms failed now and
  then; 200 ms is still well under anything a user would notice.
- Settings saved by an older version keep their whole target; an older version reading new
  settings gets the whole part.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `tempo-preview`: "SYNC control" (the target's resolution, the fader to 0.01, the digits'
  decimals, persistence), plus new requirements for the fine steps and for typing the tempo.
- `player-window`: "Keyboard shortcuts", where ↑ / ↓ step SYNC's tempo while it is on.
- `beatport-intake`: "Beatport tracks into a crate", the 500-track budget (200 ms).

## Impact

- `crates/ui/src/sync.rs`: `SyncSettings` keeps the target in hundredths beside the legacy whole
  field; `speed`, `fit`, `nudge` and the clamps work in hundredths.
- `crates/ui/src/app.rs`: `deck_section` (fader modes, digits, typing), `Action::SyncTarget`,
  `SyncNudge`, the Alt+arrow keys and the keyboard guard while typing, the SYNC messages.
- `crates/ui/src/skin/generate.rs`: the deck's "BPM" label moves right to make room for the
  decimals; a `deck_fine` layout entry.
- `crates/ui/src/help.rs`, README, CHANGELOG.
- Tests: `sync.rs` unit tests (rounding, clamps, old settings), headless tests for the fader, scroll,
  keys and typing.
