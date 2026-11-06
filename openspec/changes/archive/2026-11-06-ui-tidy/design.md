# Design

## Context

See proposal.md for why. The pieces involved today:

- The band (`band_layout`) is the mini player (275 skin px), the turntable while SYNC is on, the
  waveform or seek bar, then a `CORNER_W` = 28 corner (`window_buttons`) with minimize at x 4
  and close at x 15, both 9×9 `sprite_button`s (`btn_min`, `btn_close`).
- The mini player's LCD row ends in three 17×12 `icon_toggle`s (EQ, W, S) at
  `icons_x = w − 4 − 3·ICON_W − 4`, spaced `ICON_W + 2`. The title line runs from x 74 to
  `icons_x − 3`. A unit test checks that the longest verdict flash fits that line.
- The playlist frame is `pl_top_h` 20 + `pl_filter_h` 16 + `pl_bottom_h` 38
  (`layout::playlist_chrome`). The footer draws `pl_bottom` and holds the gear
  (`pl_opts`, 18×18) and the time text (`pl_info`). ▤ is `pl_group` (9×9) in the title bar.
- `filter_bar` adds up what its controls want (`want`), gives the search the rest within
  [60, half the width], then lays out BPM, filters and the CART switch left to right.
- `CrateInfo::grouped: Option<bool>`. `None` means "follow the crate's kind". `pl_header()` is
  `pl_columns() && !is_grouped()`.
- The skin is generated (`skin-gen`), and a test checks that the files on disk match the
  generator.

## Goals / Non-Goals

**Goals:**
- Two button families in the band: 9×9 window buttons in the corner (minimize, gear, close) and
  17×12 icon toggles on the player (▤, EQ, S, shuffle, repeat). SYNC keeps its own sprite.
- The player's two rows line up: the same right edge, one centre line in the button row.
- The footer goes away completely: no footer sprites drawn, and only a thin edge under the rows.
- Launch, idle and the playback path are untouched. These are layout and draw changes only.

**Non-Goals:**
- Merging the title bar into the filter bar (another 20 px). That may come later.
- Making the time readout follow the search and filters. It keeps counting the whole crate, as
  the footer's did.
- Adding a Shift+G shortcut. The spec says there is none; only the stale tooltip text goes.

## Decisions

**▤ as an LCD-row icon.** It joins EQ and spectrum in the same `[(id, icon, on, tip, action)]`
array, with `on = crates.shown().is_grouped()` and `Action::ToggleGrouped`, first in the row.
Where the icons start is one helper, `lcd_icons_x`, which the verdict-flash test uses too
(so the longest flash, "UNWANTED 999", is checked against the real title width). *Alternatives:*
putting ▤ in the transport row; putting it in the filter bar was considered but not chosen.

**The mini player's rows, realigned (from review feedback).** With the waveform always
shown, its button goes and SYNC moves up beside the spectrum button: the LCD row ends in ▤, EQ
and spectrum (17×12, 2 px apart) and SYNC (30×12), the last ending 4 px from the edge
(`lcd_icons_x` = 184, leaving the title 107 px, 17 characters; "UNWANTED 999" is 72). The
button row then has room to line up with it: transport 4–119, volume 123–231, shuffle 235,
repeat 254–271, so both rows end at 271. The volume track and fill are generated
`VOLUME_W` = 108 wide; `hslider` reads the width from the layout, so nothing else changes.
Volume, shuffle and repeat share `BUTTON_ROW_Y` = 37, centred on the transport's line (43)
to half a pixel (the slider is 13 tall). *Alternative:* keeping the volume at 48 and
leaving a 60 px hole after repeat, or spreading the gaps; both look less aligned than ending
the two rows together.

**The waveform is always on.** `Settings::show_waveform`, `Action::ToggleWaveform`, the `W`
key, the band's seek bar (`band_seek`, `seek_track`, `seek_thumb*`) and `tog_wave_*` go. Old
settings files still load: `Settings` is `#[serde(default)]` and ignores the field. The
repaint policy keeps its `waveform` input, now always true.

**An edge under the rows.** A `pl_bottom_edge` sprite (1×12, the left edge's panel turned
on its side, a dark line along the list) is stretched across the playlist's width under the
last row. Its height counts in `playlist_chrome`, read from the sprite, so the skin needs no new
field.

**The gear in the corner, at window-button size.** The corner becomes `CORNER_W` = 39, with
buttons at x 4, 15 and 26, each 9×9. egui hands a widget clicks a few points outside it
(`interact_radius`), so a near-miss in a 2 px gap would still reach a button: the corner's
buttons (`corner_button`) count a click only when the pointer is inside their own rectangle,
which makes the gaps dead. The new sprites are `btn_opts` and
`btn_opts_p`, drawn by `skin-gen` in the same bevel as `btn_min` and `btn_close`, with a gear
glyph. The popup is `egui::Popup::menu(&resp)` with `options_items`, as the footer had. Without
`has_options()`, the gear's slot stays empty and minimize and close keep x 4 and x 26, so their
place never depends on the build. Dragging the corner outside the three buttons still moves the
window. *Alternative:* `[⚙][_][×]`, so that a near-miss on the gear lands on minimize. The user
chose `[_][⚙][×]`, and the dead gaps cover the risk.

**The time readout as a filter-bar control.** It is drawn right after the search field, as a
sunken box built the same way as the search field's (same height, inset and padding), with the
existing `selected/total` string logic moved from the footer. In the width budget it is one
more claimant in `want`, so the search shrinks first. When the whole string doesn't fit the room
left after the search's 60 px minimum, it drops to the total only, then to nothing. The BPM
control, the filters and CART then start after it. The filters' own fallbacks (chips → buttons →
panel) run after the time readout has taken its share, so at classic width the readout gives way
before the BPM control does: the time is information, while the filters are controls. *Alternative:*
put it at the bar's right end next to ×. That was rejected because the user asked for it next to
the search.

**The footer is removed, not set to zero height.** `SkinDef` loses its `pl_bottom_h` field, and the
`pl_bottom_l/fill/r`, `pl_opts`, `pl_opts_p`, `pl_group`, `btn_group*` sprites and the
`pl_group`, `pl_opts`, `pl_info` layout entries are dropped. `playlist_chrome` becomes title +
filter. `playlist_def` stops moving `pl_group` and `pl_info`. `pl_bottom_fill` is still used to
fill the band and the corner, so that sprite stays. A skin file that still carries the old
entries or a `pl_bottom_h` loads and draws no footer: sprites and layout are open maps that only
`REQUIRED_SPRITES` / `REQUIRED_LAYOUT` constrain (the footer's names leave them), and serde
ignores the field it no longer has. A unit test loads such a file.

**Grouped by default: `unwrap_or(true)`.** `is_grouped()` becomes
`self.grouped.unwrap_or(true)`. Saved crates keep `grouped: None` unless toggled, so every
untoggled crate (the default Playlist included) opens grouped on the next launch, and a toggled
one keeps its `Some(_)`. No migration is written. *Alternative:* a migration that writes
`Some(false)` into existing crates so only new ones change. That was rejected: the user prefers
one rule, and ▤ is one click.

**Header for crates without albums.** `pl_header()` becomes
`pl_columns() && (!is_grouped() || !has_albums())`, where `Playlist::has_albums` asks whether
any entry of the *whole crate* belongs to an album. It deliberately ignores the filtered rows,
so typing a search never makes the header appear or disappear and the rows don't jump. It is
not cached: `Entry::has_album` mirrors `album_key().is_some()` without building the key, and
the scan stops at the first album, so a crate of records answers at once and only an all-loose
crate (where every row is drawn anyway) walks its entries.

**New crates' lists open grouped.** A crate created during a session gets an empty playlist;
`Crates::fresh` groups it as the crate is, so the view and `is_grouped()` agree from the start
(otherwise ▤ would read the flat view and toggle the crate the wrong way).

## Risks / Trade-offs

- [The title line loses 19 px, so long titles scroll more] → The scroll already handles it,
  and verdict flashes still fit (tested).
- [Users with flat crates see them grouped after updating] → One click on ▤ (or the list
  menu) restores a crate, and the choice is remembered. The CHANGELOG says so.
- [The gear sits next to close, which quits] → Exact 9×9 hit areas with dead 2 px gaps. The
  spec has a scenario for a click in the gap.
- [The filter bar is crowded at 275 px] → The readout is the first thing to give way, and a
  headless test sweeps the widths for overlaps, as the chips/buttons/filters test does.
- [The waveform is 11 px narrower] → Negligible at full-screen widths.

## Migration Plan

Nothing to migrate in data. Settings and crate files keep their formats, and `grouped: None`
simply means "grouped" now. Rolling back restores the old default for untoggled crates.
