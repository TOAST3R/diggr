# record-sleeve Specification

## Purpose
Draws the record card's record as a physical sleeve pulled out of a crate: the vinyl inside it,
the shop's stickers on it, and how it moves as the card opens, steps and plays. Everything here
is drawing only, so no action waits for it.

## Requirements

### Requirement: Vinyl in the sleeve
Behind the record card's cover there SHALL be a black disc with grooves and a round label, centred on the cover's middle height. While no track of the card's record plays or is paused, the disc's edge SHALL show 6 points (at 1×) to the right of the cover. A record with no image SHALL show its record icon and no disc.

#### Scenario: Resting
- **WHEN** the card of "Glasshouse EP" is open, its cover is loaded and another record plays
- **THEN** a disc's edge shows 6 points to the right of the cover, behind it

#### Scenario: Double size
- **WHEN** the skin is at 2× and the card is open on a record with a cover
- **THEN** the disc's edge shows 12 points to the right of the cover

#### Scenario: No image
- **WHEN** the card is open on a local album with no cover
- **THEN** the square shows the record icon and no disc is drawn

### Requirement: The vinyl's label takes the cover's colour
The disc's label SHALL be tinted with one colour taken from the record's cover, worked out once when the cover is read, not while drawing. While the cover is loading, or if its colour isn't known, the label SHALL use the skin's selection colour.

#### Scenario: A red cover
- **WHEN** the card shows a record whose cover is mostly red
- **THEN** the disc's label is a red tint

#### Scenario: Thumbnail first
- **WHEN** the card shows a thumbnail while the large cover loads
- **THEN** the label uses the thumbnail's colour, and keeps a similar colour when the large cover arrives

### Requirement: The vinyl comes out while its record plays
While a track of the card's record plays or is paused, the disc SHALL show 16 points (at 1×) to the right of the cover. When that starts, the disc SHALL slide out from 6 to 16 points in 180 ms, eased out, and when playback leaves the record it SHALL slide back in 180 ms. It SHALL NOT spin or move otherwise, and the card SHALL request no repaint once it has settled.

#### Scenario: Play a track
- **WHEN** the card of "Glasshouse EP" is open and the user plays A1 from it
- **THEN** the disc slides from 6 to 16 points out in 180 ms (±1 frame) and then stays still

#### Scenario: Playback moves on
- **WHEN** the last track of the card's record ends and the card steps to the next record, which plays
- **THEN** the new record's disc shows 16 points out

#### Scenario: Stop
- **WHEN** a track of the card's record plays and the user stops playback
- **THEN** the disc slides back to 6 points out in 180 ms (±1 frame)

#### Scenario: Idle
- **WHEN** a track of the card's record is paused and the disc has settled
- **THEN** the app requests no repaint for the card

### Requirement: Price sticker
The cover SHALL carry a round price sticker in its top-right corner, tilted about 12°, reading the price with no cents when they are zero ("€9", "€12.50"). The price SHALL be, first available: the copy in the user's cart for this record; in a seller crate, that seller's cheapest unsold copy; the cheapest price in the record's for-sale snapshot. With no price, there SHALL be no price sticker.

#### Scenario: For sale
- **WHEN** the card shows a record with 6 for sale from €9.00
- **THEN** a price sticker reading "€9" sits on the cover's top-right corner

#### Scenario: In the cart
- **WHEN** a copy of the record at €14.50 is in the user's cart and the cheapest for sale is €9.00
- **THEN** the sticker reads "€14.50"

#### Scenario: Seller crate
- **WHEN** a seller crate's record has unsold copies at €12.00 and €18.00 and a sold one at €6.00
- **THEN** the sticker reads "€12"

#### Scenario: Nothing for sale
- **WHEN** the card shows a local album
- **THEN** there is no price sticker

### Requirement: Hype sticker
The cover SHALL carry at most one hype sticker in its bottom-left corner, tilted about −8°: SOLD when every copy is sold or the record's store says it's sold, else OWNED when the user owns the record or another pressing of it, else WANTED when it is on the wantlist. The status strip's pills SHALL be unchanged.

#### Scenario: Wanted
- **WHEN** the card shows a record on the wantlist that the user doesn't own
- **THEN** a WANTED sticker sits on the cover's bottom-left corner, and the strip still shows the WANTED pill

#### Scenario: Owned and wanted
- **WHEN** the record is on the wantlist and the user owns another pressing
- **THEN** the sticker reads OWNED

#### Scenario: Sold out
- **WHEN** every copy of a seller crate's record is sold
- **THEN** the sticker reads SOLD

#### Scenario: Nothing to say
- **WHEN** the record is neither wanted, owned nor sold
- **THEN** there is no hype sticker

### Requirement: Stickers are only drawn
Stickers SHALL take no clicks, hovers or tooltips: the pointer over a sticker SHALL act as over the cover. They SHALL be drawn in the skin's sticker colours, a price colour and a hype colour, with OWNED in the skin's owned colour, and SHALL scale with the skin (1× or 2×). They SHALL be drawn with Reduce motion on as well.

#### Scenario: Click a sticker
- **WHEN** the user clicks the price sticker
- **THEN** nothing happens, as for a click on the cover

#### Scenario: Reduce motion
- **WHEN** Reduce motion is on and the card shows a wanted record for sale
- **THEN** both the price sticker and the WANTED sticker are drawn

### Requirement: The WANTED sticker slaps on
When the open card's record goes on the wantlist while the card shows it, its WANTED sticker SHALL appear at 125% of its size and settle to 100% in 150 ms, eased out. A record already wanted when the card opens or steps to it SHALL show its sticker at once, at full size.

#### Scenario: Press Y
- **WHEN** the card shows the playing record, not on the wantlist, and the user presses Y
- **THEN** the WANTED sticker appears at 125% of its size and is at 100% 150 ms (±1 frame) later

#### Scenario: Already wanted
- **WHEN** the user steps to a record already on the wantlist
- **THEN** its WANTED sticker is drawn at full size from the first frame

### Requirement: Pulling the record out
When a click on a record row opens the card and that row's cover is drawn, the cover SHALL fly from the row's thumbnail to the card's cover square in 180 ms, eased out, growing as it goes and drawn above the card. The card SHALL take clicks and keys from its first frame, and the flight SHALL take none. With no cover on the row, the card SHALL open without a flight.

#### Scenario: Open a card
- **WHEN** the user clicks the record row of "Glasshouse EP", whose thumbnail is loaded
- **THEN** its cover moves from the row's thumbnail to the card's cover square, reaching it 180 ms (±1 frame) later

#### Scenario: Click during the flight
- **WHEN** the card has just opened and the user clicks ⏵ on A1 in its first frame
- **THEN** A1 plays in that frame

#### Scenario: No cover
- **WHEN** the user opens the card of a record whose row shows the record icon
- **THEN** the card opens with no flight

### Requirement: Flipping the sleeve
When the card moves to another record (‹ › ← → or following playback), the sleeve SHALL flip in 220 ms: to the next record, the old cover tips forward about its bottom edge toward the viewer and fades while the new one rises behind it from slightly smaller and darker; to the previous record, the new cover tips up from forward and the old one sinks behind. A new step SHALL restart the flip from the cover shown.

#### Scenario: Next record
- **WHEN** the card shows "Glasshouse EP" and the user presses →
- **THEN** the old cover tips forward and away while the next record's cover rises behind it, and it is still and flat after 220 ms (±1 frame)

#### Scenario: Previous record
- **WHEN** the user clicks ‹
- **THEN** the previous record's cover tips up into place from forward, in 220 ms (±1 frame)

#### Scenario: Quick steps
- **WHEN** the user presses → three times within 220 ms
- **THEN** the card ends on the third record after them, the flip restarting at each step, and no step waits for a flip

#### Scenario: Following playback
- **WHEN** the card's record's last track ends and the card moves to the next record
- **THEN** the sleeve flips as for →

### Requirement: The sleeve with Reduce motion
With Reduce motion on, the sleeve SHALL NOT move: no flight on opening, no flip on stepping (the new cover is drawn in place at once), no slap (stickers at full size at once), and the disc SHALL be at its resting or played position at once. The disc's edge and the stickers SHALL still be drawn.

#### Scenario: Step with Reduce motion
- **WHEN** Reduce motion is on and the user presses → on the card
- **THEN** the next record's cover, disc and stickers are drawn in place in the next frame

#### Scenario: Play with Reduce motion
- **WHEN** Reduce motion is on and the user plays a track from the card
- **THEN** the disc shows 16 points out in the next frame
