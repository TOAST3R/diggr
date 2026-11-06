# Design

## Context

The mini player (`mini_player`, `crates/ui/src/app.rs`) draws its LCD at y 6–40 and the
transport row at `TRANSPORT_Y` 48–66 in a 76 px band (`layout::BAND_H`), leaving 66–76 empty.
Its width is the band's left panel (227–360 skin px). Seeking today is the waveform overview
row's job (`waveform::draw` returns `Action::Seek`), and the app applies `Action::Seek(secs)`,
which calls `Engine::seek` (under 50 ms). The old band seek bar (removed with `ui-tidy`) sought
on release through a `seek_drag: Option<f32>` fraction; its sprites are gone.

## Goals / Non-Goals

**Goals:**
- A bar that reads as progress at a glance and seeks with one click.
- No cost while paused: it draws from the clock the player already reads.
- Seeks that neither stop nor start with a click.

**Non-Goals:**
- Audio scrubbing while dragging (repeated seeks stutter), hold-to-wind on ⏮ / ⏭, keyboard
  changes (← / → already seek 5 s).

## Decisions

**Place and size.** The bar sits at `SEEK_Y` = 68, 7 px tall (68–75), from x 4 to `w − 4`.
Its interaction rect is 2 px taller on each side (66–77 clipped to the band) so it is easy to
hit without overlapping the transport buttons' rects (which end at 66). *Alternative:* a bar in
the LCD under the second line: it would take the LCD's text room.

**Sprites.** `seek_track_l` / `_m` / `_r` (2, 1, 2 × 7: an inset groove like the volume's, so it
stretches with the panel) and `seek_thumb` / `seek_thumb_p` (9 × 7 button faces with an accent
tick). The groove's played part is tinted by `seek_fill` (a 1-pixel column of the accent's
dim tone, tiled), so progress reads even without the thumb.

**Interaction.** Pointer down or drag sets `seek_drag = Some(fraction)` (the thumb follows it);
release with a fraction sends `Action::Seek(fraction × duration)` once; a plain click is a
press and release in one frame and seeks the same way. The tooltip names
`format::clock(fraction_under_pointer × duration)` on hover and while dragging. With no duration
or a stopped player, the interact rect isn't registered and no thumb is drawn. This is the same
behaviour as the old band seek bar, so the logic comes back nearly as it was.

**No jump back after a seek.** A seek takes up to 50 ms to be heard, and until then the clock
still reports the old position, so the thumb would jump back and then forward. After a release
the app keeps `seek_pending = (fraction, when)` and draws the thumb there until the clock is
within 2% of it or 0.5 s has passed.

**Thumb position.** `x = 4 + fraction × (w − 8 − thumb_w)`, `fraction = position / duration`,
clamped to 0..1; `seek_drag` wins while set.

**De-clicked seeks, in the renderer.** A seek, next or previous bumps the generation and the
renderer used to drop the old audio at once, so the sound stopped mid-wave and the new audio
started mid-wave: two clicks around a silence of up to 50 ms. Now, when a new generation replaces
audio that was heard in the last callback (`audible`), the renderer captures the old audio still
queued (`capture_fade`, up to `DECLICK_FRAMES` = 240, 5 ms) and fades it out from the next block's
first frame (`mix_declick`), over the silence if the new audio isn't there yet; the new audio's
first 240 frames are faded in (`fade_in`). Play from stopped, a seek while paused, gapless
boundaries and the soft flush of a speed change (its own 2 ms crossfade) are untouched. All buffers
are allocated in `Renderer::new`. *Alternatives:* a crossfaded seek that keeps the old audio playing
until the new is ready (no silence at all), declined in favour of keeping a seek's "nothing of the
old audio after it"; `Engine::seek_at`, which can only jump beyond the audio already decoded into
the ring (up to seconds ahead).

## Risks / Trade-offs

- [A 1 px margin at the band's bottom] → The bar stops at 75 of 76; the playlist's title bar
  below has its own top bevel, so the two don't merge.
- [Two places to seek] → Both seek the same way; the waveform keeps its precision (zoom), the bar
  its reach.

## Migration Plan

None: new sprites only, and no settings.
