## Why

The mockup adds the controls a digger reaches for: a DJ filter to sweep a track, a way to narrow a big crate to the tempo being dug for, and crates visible at a glance instead of behind a menu.

## What Changes

- **Resonant low-pass filter:** an LP knob in the equalizer's top row sweeps a low-pass filter over everything that plays, with a slight resonant peak. Fully right means off (no effect at all); double-click returns it there.
- **BPM filter**, per crate, in a bar above the list:
  - **BPM range:** one slider with two handles, spanning the crate's slowest to fastest known tempo, with the range as text ("124–140 BPM").
  - **ALL** clears it.

  The filter decides what's shown **and what plays**: next, previous, shuffle and the previews downloaded ahead stay within the matching tracks, and the playing track finishes even if the filter hides it. It's remembered per crate. While it's on, the title bar shows how many entries match ("LOWTIDE TAPES · 42/301").
- **Crate sidebar:** a list of crates on the left of the playlist when it's wide (at least 600 px, or maximized), with counts and the same playing (⏵) and shown (•) marks as the crate menu.
  - Click a crate to show it; right-click to rename or delete it; **+ New crate** at the bottom.
  - **Drag entries onto a crate** to send them there (as Send to crate).
  - A ☰ button in the playlist title bar shows or hides the sidebar, remembered. The crate menu still works.
- Dropped from the earlier version of this change: styles on entries and style chips, and the footer sort icon (OPT ▸ Sort and the column headers already sort). It no longer depends on `crateamp-look`: everything is drawn in the current skin.

## Capabilities

### New Capabilities
- `playlist-filters`: the BPM range, what is shown and what plays under it.

### Modified Capabilities
- `equalizer`: the resonant low-pass filter and its knob.
- `crates`: the crate sidebar and dropping entries on a crate.

## Impact

- `crates/audio`: a resonant low-pass biquad in the renderer after the EQ and before the tap, with a smoothed cutoff, real-time safe (no allocation; `tests/rt_alloc.rs` stays green); `Engine::set_filter`.
- `crates/ui`:
  - `playlist.rs`: the per-crate BPM range, the filtered play order and the shown rows;
  - `app.rs`: the filter bar, the sidebar and dropping on it, the LP knob, the title bar's count and ☰ button;
  - `app/digging.rs`: the preview horizon follows the filtered play order;
  - `crates.rs` / settings: the range saved with its crate, the sidebar on or off;
  - `skin/generate.rs`: sprites for the knob, the ☰ button and the slider handles (the skin is regenerated with `skin-gen`).
- Help panel, README.
