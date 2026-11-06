# Design

## Context

The record card (`crates/ui/src/app/card.rs`) is a pure drawing function. The app builds a `Card`
view each frame and `card::show` paints it and returns `CardAction`s. The app keeps which record
is open in `OpenCard` (`app.rs`), including `moved: Option<(f64, i8)>`, the egui time and
direction of the last step. The content slide is a `TSTransform` over everything inside the
frame (`SLIDE` = 0.22 s, `ease`). The cover is drawn by `draw_cover` into a fixed square
(`COVER` = 112 pt × scale), fitted from a `CardImage` (large cover, else thumbnail, else
waiting frame, else icon).

Covers are decoded by the cover worker (`dig::cover`, a background thread) into
`Cover { width, height, rgba }` and turned into textures in `app/covers.rs` (`CoverResult::Ready`
/ `LargeReady`). Thumbnails live in an LRU of textures, large covers in a second one.

The `motion` module (`crates/ui/src/motion.rs`) already provides the timing vocabulary
(`FAST` 0.12, `MEDIUM` 0.18, `SLOW` 0.24, `ease_out_cubic`), `motion::reduced(ctx)`, and the
layer pass that fades and grows dialogs, the card included (96% → 100% over 180 ms).

See proposal.md for why. The behaviour is in `specs/record-sleeve` and the `record-card` deltas.

## Goals / Non-Goals

**Goals:**
- Every sleeve effect is computed from state the app already has, plus timestamps. Nothing is
  persisted and no request is made.
- The pure parts (geometry, timing, sticker choice, tint) are plain functions with unit tests
  in `card.rs` / `cover.rs`. Headless tests cover the wiring.
- Zero idle cost: once nothing moves, the card requests no repaint.

**Non-Goals:**
- Real 3D or a shader. egui meshes with a fake perspective are enough at 112–224 pt.
- Animating the record rows, or anything outside the card.
- Changing what the card's controls do, its size or its place.

## Decisions

### 1. The sleeve is drawn inside `card.rs`, from a `Sleeve` view the app fills
`Card` gains a `sleeve: Sleeve` field:

```
Sleeve {
    tint: Option<Color32>,              // label colour, None → skin selection colour
    out: f32,                           // disc's edge, 0 = resting (6 pt) … 1 = played (16 pt)
    price: Option<String>,              // "€9", already formatted
    hype: Option<Hype>,                 // Sold | Owned | Wanted
    slap: Option<f32>,                  // seconds since WANTED appeared
    flip: Option<Flip>,                 // previous image + its tint, seconds, direction
    flight: Option<(Rect, f32)>,        // row thumbnail rect, seconds since opening
}
```

The app works out each field; `card.rs` only draws. This keeps the existing split (format/app
decide, card draws) and lets unit tests drive drawing helpers with plain values.

*Alternative:* let `card.rs` read the app's marks and timers. Rejected: the card module has
never known the app, and its tests would need a whole rig.

### 2. The disc's slide is an egui bool animation, keyed by the record
`out` = `ctx.animate_bool_with_time(id(record), playing_here, MEDIUM)` eased with
`ease_out_cubic`, or `playing_here as u8 as f32` under Reduce motion. egui's animation requests
repaints only while it changes, so the card is idle once it settles. Keying by record means a
step to a playing record shows its disc already out, because a new key starts at its target.
That matches the scenario "Playback moves on".

The disc is drawn *before* the cover, as a filled circle (radius 0.47 × side, centred
`side × 0.5 − 6 pt + out × 10 pt` right of the cover's centre so its edge shows 6 → 16 pt). It
has three faint groove rings, and a label of radius 0.17 × side in the tint with a small hole.
The text column's x-offset grows from `GAP` to 22 pt × scale, a constant, so layout never
depends on the animation.

### 3. The tint is computed once, in the cover worker
`Cover` gains `tint: [u8; 3]`, computed where the pixels are decoded (`dig::cover`), off the UI
thread. The algorithm samples a grid of at most 32 × 32 pixels, weights each by saturation ×
(1 − |lightness − 0.5|) plus a small floor, takes the weighted mean, then clamps the lightness to
0.35–0.75, so the label always shows against the black disc. Sampling a fixed grid makes the
thumbnail and the large cover land on near-identical colours, so swapping them doesn't visibly
retint the label. `covers.rs` keeps the first tint read for each cover (thumbnail or large), so the
large cover arriving never retints the label, and the app looks it up by entry.

*Alternative:* the most frequent colour (k-means or a histogram). It costs more, and on
covers with a white border it picks the border. A weighted mean is cheap and good enough for a
label.

### 4. Stickers are text on rotated shapes, drawn last
- Price: a circle of radius 15 pt × scale, centred 4 pt inside the cover's top-right corner. It
  is filled with `sticker_price` and has a 1 pt darker rim. The text is in a bold font fitted
  to 80% of the diameter (shrunk for "€12.50"), rotated 12° with `TextShape::with_angle`
  about the circle's centre.
- Hype: a rounded rectangle at the bottom-left, filled `sticker_hype` (OWNED: `pl_owned`),
  with the word in the card's background colour, both rotated −8°. egui can't rotate a rect,
  so it's drawn as a 4-point convex polygon from rotated corners.
- Choice (`sticker_price`, `hype`) is a pure function in `format.rs` next to `record_details`,
  taking the cart copy, seller copies and for-sale snapshot it already uses for the strip. The
  price formats without zero cents: `price_short(cents, currency)` → "€9", "€12.50".
- Stickers are painted with the painter only, never `ui.interact`, so they take no input.

### 5. The slap reuses a timestamp, not an egui animation
`OpenCard` gets `wanted_at: Option<f64>`. It is set when the card's record goes from not wanted
to wanted *while the card shows it*, and cleared on step. The scale is
`1.25 − 0.25 × ease_out_cubic(t / 0.15)`. A record that is already wanted when the card arrives
on it has `wanted_at = None` and draws at 1.0. The app detects the change by comparing last
frame's wanted state of the card's record. That catches Y, + WANT and the menu alike.

### 6. The flight is painted in its own layer above the card, and only painted
When a plain click opens the card, the app records the clicked row's cover rect (`record_row`
already computes `cr`) and the open time in `OpenCard::from: Option<(Rect, f64)>`. Only when
that row's cover was `Loaded`. While `t < MEDIUM`, `card::show` paints the image in a
`LayerId::new(Order::Tooltip, …)` with `layer_painter` (no `Area`, so it never takes input). It
goes from `from` to the square, lerped with `ease_out_cubic`, and skips the square's own image
for those frames, so the cover isn't drawn twice. The card's own layer still gets the
`motion.rs` grow-and-fade. The flying cover is fully opaque so it reads as the object being
lifted.

The flight's target is the square `draw` returns in the same frame: layout is final there
(the card's grow-and-fade is a visual transform applied afterwards), so no rect from an earlier
frame is needed. On the card's very first frame egui sizes the centred area before placing it,
so the target can be off for that one frame, which is invisible under the card's fade-in. A step
lands a flying cover at once (`from` is cleared).

### 7. The flip is a 5 × 5-vertex textured mesh with a projected rotation
`flip_mesh(rect, tex, uv, angle, pivot_bottom, shade)` builds a `Mesh` of a 4 × 4 grid. Each
vertex is rotated about the horizontal axis through the bottom edge by `angle` and projected
with a simple perspective (`f = d / (d − z)`, with `d` = 3 × side). Subdividing keeps egui's
affine-per-triangle texturing from visibly bending the cover. Vertex colours darken with the
angle (`shade`) for a light falloff.

- Next (›): the new cover is drawn first, scaled 0.94 → 1.0 and darkened 40% → 0. The old cover
  is drawn over it, rotating 0 → 80° toward the viewer (its top comes forward and down) and
  fading 1 → 0.
- Previous (‹): the old cover is drawn first, sinking (1.0 → 0.94, darkening). The new one is
  drawn over it, rotating 80° → 0 from forward.
- The disc and stickers belong to the new record and are drawn flat over the flip's result.
  Hiding the old disc mid-flip is unnoticeable at 220 ms.
- The timing is the existing `moved` (so text slide and flip stay in step), `SLIDE` and
  `ease_out_cubic`. The flip replaces the slide's transform for the cover only: the cover is
  drawn outside the `with_visual_transform` that moves the text.

`OpenCard` keeps `shown`, the image drawn last frame, and `left`, the image shown when the last
step happened, copied from `shown` *before* the record changes. A quick second step captures what the
first step's new record showed, so the flip restarts from the cover shown. A `Waiting` or
`Icon` image flips as its frame or icon drawn into the mesh's rect, a plain quad with no texture.

*Alternative:* a horizontal "page turn" around the left edge. Rejected: sleeves in a crate tip
forward about their bottom edge. The vertical tip is what reads as a crate.

### 8. Reduce motion is handled where the timings are made
The app passes `flight: None`, `slap: None`, `flip: None`, no text slide and a snapped `out`
when `motion::reduced(ctx)`. `card.rs` never checks the setting. One place decides, and the
drawing code has no branches for it. (The text slide used to run under Reduce motion too; it
now stops with the rest, as `ui-motion` "Reduce motion" asks.)

### 9. Skin colours
`Colors` gains `sticker_price` (default `[244, 214, 72]`, a shop yellow) and `sticker_hype`
(default `[214, 64, 52]`, a sticker red), both `#[serde(default = …)]` so older skin files
load. `skin-gen` writes them, and the existing skin-gen test catches a stale `skin.ron`.

## Risks / Trade-offs

- [Rotated text in egui looks soft at 1×] → Stickers use a bold font at no less than 9 pt; at 1×
  the tilt is small enough that the rasterised glyphs stay legible. Check at 1× on a non-Retina
  scale factor before ticking the task. If it isn't legible, drop the price sticker's tilt to 0°
  at 1× only (a design detail, not a spec change: the spec says "about").
- [The flight's first frame has no card rect] → falls back to no flight that frame (Decision 6).
  Worst case the cover appears one frame late, invisible at 60 Hz.
- [Narrow windows] → the text column loses 12 pt to the disc lane. The card is already capped at
  420 pt and narrows with the window, and long titles already elide.
- [Thumbnail evicted before a flip] → `left` holds a `TextureId`. If the LRU drops the texture
  mid-flip, egui draws a missing texture for a few frames. The thumbnail LRU holds a screenful
  plus scroll-back and the card's covers were just drawn, so eviction within 220 ms needs a
  sudden flood of new rows. Accepted. If it shows up, pin the card's current and previous keys
  in the LRU.
- [Repaint cost during a flip] → two meshes of 25 vertices and a few shapes; nothing compared to
  the waveform. Repaint requests stop with the timers (the existing `card.slide.is_some()`
  check, extended to flight and slap).

## Migration Plan

No data migration. Skin files without the two sticker colours load with their defaults. To roll
back, revert the change; nothing on disk depends on it.
