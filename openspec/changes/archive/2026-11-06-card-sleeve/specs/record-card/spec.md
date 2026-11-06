# Spec Delta

## MODIFIED Requirements

### Requirement: Stepping transition
Whenever the card moves to another record (‹ ›, ← →, or following playback), its text and tracklist SHALL slide in from the side it moves towards (from the right for the next record, from the left for the previous one) while fading in, in about 220 ms, and its cover SHALL flip as `record-sleeve` "Flipping the sleeve" says, in the same 220 ms. The card's size and place SHALL NOT change. The animation SHALL NOT delay stepping again, and SHALL NOT affect playback.

#### Scenario: Next record
- **WHEN** the user clicks ›
- **THEN** the next record's text slides in from the right and fades in while its cover flips into place, and both are in place after about 220 ms

#### Scenario: Quick steps
- **WHEN** the user presses → three times within 220 ms
- **THEN** the card ends on the third record after them, its slide and flip starting again at each step

### Requirement: Record card look
The card SHALL be drawn in the skin's playlist colours (its background, text, dimmed text and highlight colours) with a one-pixel border in its dimmed text colour, not in the default tooltip style. It SHALL scale with the skin size (1× or 2×). Its cover SHALL be a square of 112 points at 1× on the left, drawn as a sleeve with its vinyl (see `record-sleeve`), with the text to its right starting 22 points at 1× after the cover, clear of the disc at its furthest out, and the card SHALL be no wider than 420 points at 1×, nor wider than the window.

#### Scenario: Skin colours
- **WHEN** the card is shown
- **THEN** its background is the playlist background colour and its title is drawn in the playlist text colour

#### Scenario: Double size
- **WHEN** the skin is at 2× and the card is shown
- **THEN** the cover square is 224 points

#### Scenario: Clear of the disc
- **WHEN** a track of the card's record plays and its disc is 16 points out
- **THEN** no text or button of the card overlaps the disc

### Requirement: Record card cover
The card SHALL show the record's large cover when it is loaded. Until then, or when the record has no large cover, it SHALL show the record's thumbnail scaled to the cover square, and an empty frame while neither is loaded. A record with no image SHALL show a record icon in the square. The large cover SHALL replace the thumbnail as soon as it arrives, without the card moving or changing size. The image SHALL be the sleeve's front: the disc behind it and the stickers on it (see `record-sleeve`) SHALL be drawn with whichever image is shown, and with the empty frame.

#### Scenario: Thumbnail first
- **WHEN** the card opens for a record whose thumbnail is loaded and whose large cover isn't cached
- **THEN** the card shows the thumbnail in the square at once, and the large cover in its place when it arrives

#### Scenario: No large address
- **WHEN** the record came from a seller listing, which gives no large image
- **THEN** the card shows the thumbnail in the square, and no large cover is requested

#### Scenario: No image
- **WHEN** the record has no image
- **THEN** the card shows the record icon in the square

#### Scenario: Stickers while loading
- **WHEN** the card opens for a wanted record for sale whose covers are still loading
- **THEN** the empty frame carries the price and WANTED stickers and the disc's edge
