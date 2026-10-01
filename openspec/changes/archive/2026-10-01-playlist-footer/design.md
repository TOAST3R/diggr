## Context

- **The footer (playlist bottom bar)** is 38 skin px tall and stretches with the playlist's width. At the classic 275 px width it holds:
  - ADD, REM, SEL and MISC (25 × 18 each) at x 11, 40, 69 and 98;
  - the "selected/total" time readout at x 128 (100 px);
  - OPT at x 236 and the resize grip at x 263.
- **What those menus hold (15 items):**
  - ADD: Add files…, Add folder…
  - REM: Remove selected, Clear crate
  - SEL: Select all / none / invert
  - MISC: Import M3U… (the same action as Add files…), Export M3U…
  - OPT: Double size, Spectrogram, Sort ▸, Discogs…, Browser…
- **The BPM filter** is a full row above the list, drawn by `bpm_bar`: ALL, a two-handle slider, and the range text. It shows when the crate has two different known tempos.
- **The crate sidebar** shows at 600 px or maximized, when the `crate_sidebar` setting is on, which the ☰ title-bar button toggles.
- **The main window** has no menu. While the playlist is maximized, the player is a thin strip.
- **Clipboard:** text from the clipboard arrives only as a paste event, so a "Paste link" menu item would need a new dependency. Cmd+V stays the way to paste a Discogs page.

## Goals / Non-Goals

**Goals:**
- Give the list back its rows, and the footer only what belongs to the crate.
- Fit everything at the classic 275 px width.
- Keep every action reachable.

**Non-Goals:**
- Changing what the BPM filter does, how the sidebar works, or any menu item's behaviour.
- A menu bar, keyboard shortcuts for the new menus, or a Paste item.

## Decisions

**1. The sidebar has no switch.** It shows exactly when it fits: 600 px or maximized.
- `Settings::crate_sidebar`, `Action::ToggleSidebar` and the `pl_side` button and sprite are removed.
- An old `settings.ron` holding `crate_sidebar` still loads, because unknown keys are ignored.

**2. The footer, left to right (classic width):**
```
 x: 11  32  56                                      151  155          259 263
    [+] [≡] ◂━━●━━●━━▸ 124-139 ×                         [ 1:02/48:12+ ]  ◢
```
- `+` and `≡` are 18 × 18 buttons at x 11 and 32, with the skin's button face and a glyph drawn by the generator. They are momentary like the old ones.
- The time readout's LCD box moves to the right of the footer's right cap, into the space OPT used, ending just before the grip (W−120 to W−16). The readout is right-aligned in it, moves with it as the playlist widens, and shows only the total when "selected/total" is too long for the box (`0:00/427:46:04+` still fits).
- The BPM control sits between `≡` and that box, and never runs into it:
  - the slider, a fixed 40 px at any width (two handles, the same sprites and the same nearest-handle drag);
  - the range text (`124-139`, without "BPM"), then a × sprite button, shown only while a range is set, which clears it;
  - "BPM" in the skin's font in front of the slider, when there's room for it (not at 275 px, from about 300 px on).
- The slider starts at a fixed place, so it never moves under a dragged handle.
- Double-clicking the slider also clears the range. The tooltip shows the range and "n without a BPM are hidden".
- Under two known tempos, the control isn't drawn and the footer is just the buttons and the readout.
- The ALL button goes. The list still says "NO TRACKS MATCH", and its tooltip says that × clears the range.
- × is a 7 × 7 sprite button (`bpm_clear`), because the skin's font has no × glyph.

**3. The menus.**
- **`+`:** Add files…, Add folder…, ─, Import M3U… (kept under its own name, because people look for it).
- **`≡`:** Select all, Select none, Invert selection, ─, Remove selected, Clear crate, ─, Sort ▸ (every field), Show all tempos (only while a range is set), ─, Export M3U….

**4. The Options menu.**
- Right-clicking (or Control-clicking on macOS) anywhere on the main window or the maximized strip that isn't a control opens: Double size / Classic size, Spectrogram (S), ─, Discogs…, Browser… (the last two only where digging is available).
- It's a context menu on a click area under the section's controls, so the controls keep their own clicks.
- The main window's tooltip on that area says "Right-click for options" (shown after the usual delay).

**5. The filter row's removal.** `pl_bar()` and the extra row in `pl_visible_rows` go, so the list's first row is entry 1 again. The header and rows start at the list top as before the last change.

**6. Skin.** `generate.rs` gains `pl_plus` and `pl_menu` (normal and pressed) and `bpm_clear`. It loses `pl_add`, `pl_rem`, `pl_sel`, `pl_misc`, `pl_opts` and `btn_side`. The layout gets `pl_plus`, `pl_menu` and `pl_bpm` (the control's area), and `pl_info` moves to the right. Regenerated with `skin-gen`; its test keeps the committed atlas in sync.

## Risks / Trade-offs

- [Settings are harder to find in a right-click menu] → a tooltip on the main window says so, the help panel lists it, and the README names it everywhere OPT was.
- [Fewer visible buttons] → every item is still one click into `+` or `≡`, and the common ones keep their keys (Cmd+O, Delete, Cmd+A, S).
- [A tight footer at 275 px] → a fixed 40 px slider, the range and × fit between `≡` and the time's box; "BPM" shows only when there's room. 40 px over a 92–171 span is about 2 BPM per pixel: the tooltip shows the exact range.
- [Muscle memory for ADD/REM/SEL] → the two menus group the same items in the same order.
