# Proposal

## Why

The record card is where a digger looks at one record at a time, the closest Diggr gets to
flipping through a real crate. But its cover is a flat picture in a square, and stepping to the
next record slides text sideways like a slideshow. The card has the room (112 pt, 224 at 2×, a
large cover up to 600 px) to look and move like a record pulled out of a bin: a sleeve with the
disc inside, the shop's price sticker on it, tipping forward to show the next one. This makes
digging in Diggr feel like digging, without changing what any control does or slowing anything
down.

## What Changes

- **The cover becomes a sleeve with its vinyl.** A black disc sits behind the cover, its edge
  showing on the right. Its label is tinted with a colour taken once from the cover. When a
  track of the record plays, the disc slides further out (about 180 ms, once), and it slides back
  in when playback leaves the record. It never spins. The text column moves right just enough
  to make room for the disc. The card keeps its width.
- **Stickers on the sleeve.** A round price sticker, tilted, in the cover's top-right corner:
  the price of the copy in the cart when one is, else the seller's cheapest unsold copy in a
  seller crate, else the cheapest price for sale. One hype sticker, tilted, in the bottom-left
  corner: SOLD, OWNED or WANTED, in that priority. The pills in the status strip are unchanged.
  Stickers are only drawn: they take no clicks and show no tooltip.
- **The WANTED sticker slaps on.** When the open card's record goes on the wantlist (Y,
  + WANT or the menu), its WANTED sticker lands from 125% to 100% of its size over 150 ms.
- **Pulling the record out.** When a click on a record row opens the card, the cover flies from
  the row's thumbnail to the card's cover square over 180 ms, while the card does its usual
  fade-and-grow. This is drawing only: the card takes clicks and keys from its first frame.
- **Flipping to the next record.** Whenever the card moves to another record (‹ › ← → and
  following playback), the cover flips instead of sliding. On ›, the old sleeve tips forward and
  down toward the viewer while the new one rises behind it. On ‹ it's the reverse. The text keeps
  its slide and both take 220 ms. A quick step restarts the flip.
- **Reduce motion** keeps the stickers and the disc's edge but drops every movement: no flight,
  no flip, no slap, and the disc is at its place at once.
- **Skin.** Two new skin colours for the stickers (`sticker_price`, `sticker_hype`), written by
  `skin-gen`. Older skins default to them.

Not in this change: sleeve wear from a copy's grades, a bin view of the crate, divider cards in
the list, any sound.

## Capabilities

### New Capabilities
- `record-sleeve`: how the record card draws the record as a physical sleeve: the vinyl in it,
  the stickers on it, and how the sleeve moves as the card opens, steps and plays.

### Modified Capabilities
- `record-card`: "Record card look" (the text column starts after the disc's edge, not right
  after the cover), "Record card cover" (the image is drawn as the sleeve's front, see
  `record-sleeve`), and "Stepping transition" (the cover flips instead of sliding, and the text
  keeps the slide).

## Impact

- `crates/ui/src/app/card.rs`: the cover is drawn as a sleeve (disc, stickers, flip mesh), and
  the text column is offset. `Card` gains the sleeve's state: the previous image for the flip,
  stickers, the disc's position, and where the cover flies from.
- `crates/ui/src/app.rs`: `OpenCard` keeps the previous record's image, where the card was
  opened from, and when WANTED appeared. The app works out the stickers from the record's marks
  and prices.
- `crates/dig/src/cover.rs` / `crates/ui/src/app/covers.rs`: the cover worker computes a cover's
  tint once as it decodes, and the UI keeps it next to the texture.
- `crates/ui/src/skin/`: `Colors` gains `sticker_price` and `sticker_hype` (serde defaults), and
  `skin-gen` writes them to `assets/skin/default/skin.ron`.
- Specs: `record-card`, plus a new `record-sleeve`. README (record card section) and CHANGELOG.
- No change to audio, analysis, playback timing, fetching or any action. Nothing new is
  persisted.
