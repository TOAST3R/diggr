# record-card Specification

## Purpose
The record card of a grouped crate: opened by clicking a record row, it shows the record's
cover, pressing, tracklist and buying state in the skin, plays its tracks, and steps from
record to record, so records can be dug through one at a time.

## Requirements

### Requirement: Opening the record card
In a grouped crate, a plain click on a record row (no Shift, Cmd or Ctrl) SHALL select the record and open its card, including for a record of one entry. A Shift-, Cmd- or Ctrl-click SHALL only change the selection, and a double-click SHALL still play the record. Hovering a record row SHALL show no card and no tooltip. Track, copy and flat-crate rows SHALL NOT open the card.

#### Scenario: Click opens
- **WHEN** a grouped crate is shown and the user clicks the record row of "Glasshouse EP"
- **THEN** the record is selected and the card for "Glasshouse EP" opens

#### Scenario: Hover shows nothing
- **WHEN** the pointer rests on a record row whose card isn't open
- **THEN** no card and no tooltip is shown

#### Scenario: Extending the selection
- **WHEN** the user Shift-clicks a second record row
- **THEN** both records are selected and no card opens

#### Scenario: Flat crate
- **WHEN** the crate is shown flat and the user clicks an entry
- **THEN** no card opens

### Requirement: Record card placement and closing
The card SHALL open centred in the window, whichever row opened it, and stay there while it steps. While it is open, the rest of the window SHALL be dimmed and SHALL take no clicks, drags, hovers or scrolling: only the card SHALL respond to the pointer. It SHALL close with the × at its top right or Esc, and also when another crate is shown, the crate is ungrouped, or its record is no longer shown. Building it SHALL NOT read files or make requests.

#### Scenario: Centred
- **WHEN** the user opens the card of the first record row, then of the last record row in view
- **THEN** both cards are centred in the window

#### Scenario: Close button
- **WHEN** the card is open and the user clicks its ×
- **THEN** the card closes and the playlist keeps its selection

#### Scenario: Esc
- **WHEN** the card is open and the user presses Esc
- **THEN** the card closes and the playlist keeps its selection

#### Scenario: Nothing else takes the pointer
- **WHEN** the card is open and the user clicks a record row, the player's buttons or the sidebar, or scrolls over the list
- **THEN** nothing happens and the card stays open

#### Scenario: Dimmed
- **WHEN** the card is open
- **THEN** the window behind it is drawn under a dark, half-transparent layer

#### Scenario: Filtered away
- **WHEN** the card of a record at 124 BPM is open and a BPM filter of 128–130 is set
- **THEN** the card closes

### Requirement: Record card contents
The card SHALL show the record's artist (its credited artist when known) and album on two large lines; then label · catalog number · year and its formats on one dimmed line; its styles (its genres when it has no style) as pills; and its tracklist. Information that isn't known SHALL be left out. A local album SHALL show its artist, album and tracklist only.

#### Scenario: A Discogs record
- **WHEN** the card is opened for "Glasshouse EP" by Nightcraft, Lowtide Tapes, LT-012, 1994, Vinyl, styles Deep House and Minimal
- **THEN** the card reads "Nightcraft" and "Glasshouse EP", then "Lowtide Tapes · LT-012 · 1994 · Vinyl", then the pills Deep House and Minimal

#### Scenario: Compilation
- **WHEN** the record is credited to "Various" and its first track is by Lumen
- **THEN** the card's artist line reads "Various"

#### Scenario: Local album
- **WHEN** the card is opened for local album "Geogaddi" by Boards of Canada
- **THEN** the card shows "Boards of Canada", "Geogaddi" and its tracks, with no label line, no pills and no status strip

### Requirement: Record card tracklist
The card SHALL list the record's shown entries in crate order: a play button, side, title (with the track artist when not the record's) and tempo when known. A track waiting for its audio SHALL show how it's coming instead ("queued", "40%", "search"), or "next" once armed. The playing or paused track SHALL be in the highlight colour, an unavailable one dimmed. The list SHALL always take 8 lines' height, whatever the record holds, and scroll inside the card past 8.

#### Scenario: Short record
- **WHEN** the record has entries A1 "Tidepool" at 124 BPM, A2 "Lowlight" and B1 "Undertow" at 122 BPM
- **THEN** the card lists "A1 Tidepool 124", "A2 Lowlight" and "B1 Undertow 122"

#### Scenario: Same height
- **WHEN** the user steps from a record of 1 track to one of 3 and one of 14
- **THEN** the card keeps the same height, and the 14-track list scrolls

#### Scenario: Downloading
- **WHEN** one entry's preview is 40% downloaded
- **THEN** its line ends with "40%"

### Requirement: Playing from the record card
Each line's button SHALL show ⏵, or ⏸ on the playing track. ⏵ SHALL do what double-clicking the row does: play ready audio; fetch audio not downloaded yet first and play it on arrival; retry a failed clip or a track not found. ⏵ on the paused track SHALL resume, ⏸ SHALL pause. Only a track with nothing to try (no clip, already in the crate) SHALL have no button. The card SHALL stay open, and ready audio SHALL start in under 30 ms.

#### Scenario: Play a track
- **WHEN** the card of "Glasshouse EP" is open and the user clicks ⏵ on A2
- **THEN** A2 plays, its line is highlighted with ⏸, and the card stays open

#### Scenario: Pause and resume
- **WHEN** A2 plays and the user clicks its ⏸, then its ⏵
- **THEN** playback pauses, then resumes from where it paused

#### Scenario: Another track
- **WHEN** A2 plays and the user clicks ⏵ on B1
- **THEN** B1 plays from its start

#### Scenario: Not downloaded yet
- **WHEN** the user clicks ⏵ on a track whose preview is only queued
- **THEN** its preview is fetched before any other, its line says "next", and it plays as soon as it arrives

#### Scenario: Not found
- **WHEN** the user clicks ⏵ on a track whose search found nothing
- **THEN** it is searched for again, and plays if a preview is found

#### Scenario: Nothing to play
- **WHEN** a track has no clip
- **THEN** its line has no play button

### Requirement: Seeking from the record card
While a track of the card's record plays or is paused, the card SHALL show a thin progress bar under its tracklist with the elapsed and total time, and a click or drag on it SHALL seek that track, as the player's position bar does. Seeking SHALL meet the player's seek target (under 50 ms). With no track of the record playing, the bar SHALL be drawn empty and take no clicks.

#### Scenario: Seek
- **WHEN** A2 of the card's record plays, 6:00 long, and the user clicks the middle of the card's bar
- **THEN** A2 jumps to 3:00 and plays on

#### Scenario: Another record playing
- **WHEN** a track of another record plays
- **THEN** the card's bar is empty and a click on it does nothing

### Requirement: The card follows playback
When a track of the card's record finishes and playback moves on to a track of another record (the next record in a grouped crate's play order), the card SHALL move to that record, as stepping to it does, with the stepping transition. It SHALL NOT move when the playing track changes and the card's record wasn't the one playing.

#### Scenario: End of the record
- **WHEN** the card shows "Glasshouse EP" and its last track ends, and "Lowlight" starts
- **THEN** the card slides to "Lowlight", which is selected

#### Scenario: Browsing elsewhere
- **WHEN** the card shows a record that isn't playing and the playing track moves to the next one
- **THEN** the card stays on its record

### Requirement: Stepping transition
Whenever the card moves to another record (‹ ›, ← →, or following playback), its content SHALL slide in from the side it moves towards (from the right for the next record, from the left for the previous one) while fading in, in about 220 ms, and its size and place SHALL NOT change. The animation SHALL NOT delay stepping again, and SHALL NOT affect playback.

#### Scenario: Next record
- **WHEN** the user clicks ›
- **THEN** the next record's content slides in from the right and fades in, and is in place after about 220 ms

#### Scenario: Quick steps
- **WHEN** the user presses → three times within 220 ms
- **THEN** the card ends on the third record after them, its slide starting again at each step

### Requirement: The record card fetches its record
While a card is open, the previews of its record's tracks SHALL be downloaded, and its tracks with no clip searched for, right after the armed and playing tracks' and before the rest of the playing crate's next ones, two at a time, so a track is usually ready when the user clicks it. Stepping to another record SHALL move this to the new record; tracks of the record left SHALL no longer be fetched unless they are near the playhead. Closing the card SHALL stop it.

#### Scenario: Opening a card
- **WHEN** the user opens the card of a record of 5 tracks, none downloaded, while entry 3 of another record plays
- **THEN** its 5 previews download two at a time, after the playing track's and before entries 4 to 6

#### Scenario: Stepping on
- **WHEN** the card steps to the next record while the first record's previews are downloading
- **THEN** the downloads not near the playhead are cancelled and the new record's start

#### Scenario: Tracks to search
- **WHEN** the card's record has tracks with no clip
- **THEN** they are searched for, and their previews downloaded as they are found

### Requirement: Stepping through records
The card SHALL have ‹ and › buttons beside its title that show the previous and next record row, skipping rows that aren't records; while it is open, ← and → SHALL do the same, in place of opening and closing records. Stepping SHALL select the new record, move the cursor to it, scroll the list by the least amount that shows its row, and keep the card beside it. ‹ SHALL be disabled on the first record and › on the last. Stepping SHALL NOT change what plays.

#### Scenario: Next record
- **WHEN** the card of the first record is open and the user clicks ›
- **THEN** the card shows the second record, which is selected

#### Scenario: Arrow keys
- **WHEN** the card is open on the third record and the user presses ←
- **THEN** the card shows the second record

#### Scenario: Out of view
- **WHEN** the card is open on the last record row in view and the user presses →
- **THEN** the list scrolls by one record and the card shows the next record beside its row

#### Scenario: Ends
- **WHEN** the card is open on the last record row
- **THEN** › is disabled, and → does nothing

#### Scenario: Playing on
- **WHEN** a track of the first record plays and the user steps to the next record
- **THEN** the track plays on

### Requirement: Record card status strip
Along its bottom, the card SHALL show the record's buttons (see Record card actions) and its for-sale snapshot with how long ago it was fetched. In a seller crate, it SHALL show that seller's unsold copies and their price range in place of the snapshot. With nothing to show, the strip SHALL be left out.

#### Scenario: For sale
- **WHEN** the record has 6 for sale from €9.00 fetched 3 hours ago
- **THEN** the strip shows "6 for sale from €9.00 · fetched 3 h ago"

#### Scenario: Seller crate
- **WHEN** a seller crate holds the record with copies at €9.00, €12.00 and €18.00, none sold
- **THEN** the strip shows "3 copies €9.00–€18.00"

#### Scenario: Nothing to show
- **WHEN** the record is a local album
- **THEN** the card has no status strip

### Requirement: Record card actions
The card SHALL offer the record row's actions as pills: + WANT / WANTED (a click toggles the wantlist) for a Discogs record; the row's cart pill (+ CART, IN CART, a store's or the buy pill) with » to check out when in a cart; N COPIES for a seller record with several copies, opening it in the list; an OWNED badge naming the pressing; SOLD when every copy is sold. Each SHALL act as on the row, and the card SHALL stay open.

#### Scenario: Add to the cart
- **WHEN** a seller crate's record has one unsold copy at €9.00, not in the cart, and the user clicks + CART on its card
- **THEN** the copy is added to the Discogs cart and the card's pill reads IN CART

#### Scenario: Take it out
- **WHEN** that copy is in the cart and the user clicks IN CART on the card
- **THEN** it is taken out of the cart

#### Scenario: Wantlist
- **WHEN** the user clicks + WANT on a Discogs record's card
- **THEN** the record is added to the wantlist and the pill reads WANTED

#### Scenario: Owned
- **WHEN** the user owns another pressing of the record
- **THEN** the card shows OWNED, and its tooltip says "another pressing (…)"

#### Scenario: Several copies
- **WHEN** a seller crate's record has 3 unsold copies and the user clicks 3 COPIES on its card
- **THEN** the record opens in the list, showing its copies

### Requirement: Record card counts
For a Discogs record with tracks waiting to be searched or found by search, the status strip SHALL count the record's entries, its entries with a Discogs clip and its entries still to search ("11 tracks · 1 clip · 10 to search"), leaving out counts of zero.

#### Scenario: Counting tracks and clips
- **WHEN** the card is opened for "Mezzanine", with 11 entries, one with its Discogs clip and 10 to search
- **THEN** the strip shows "11 tracks · 1 clip · 10 to search"

#### Scenario: All clips
- **WHEN** every entry of the record has its Discogs clip
- **THEN** the strip shows no counts

### Requirement: Record card look
The card SHALL be drawn in the skin's playlist colours (its background, text, dimmed text and highlight colours) with a one-pixel border in its dimmed text colour, not in the default tooltip style. It SHALL scale with the skin size (1× or 2×). Its cover SHALL be a square of 112 points at 1× on the left, with the text to its right, and the card SHALL be no wider than 420 points at 1×, nor wider than the window.

#### Scenario: Skin colours
- **WHEN** the card is shown
- **THEN** its background is the playlist background colour and its title is drawn in the playlist text colour

#### Scenario: Double size
- **WHEN** the skin is at 2× and the card is shown
- **THEN** the cover square is 224 points

### Requirement: Record card cover
The card SHALL show the record's large cover when it is loaded. Until then, or when the record has no large cover, it SHALL show the record's thumbnail scaled to the cover square, and an empty frame while neither is loaded. A record with no image SHALL show a record icon in the square. The large cover SHALL replace the thumbnail as soon as it arrives, without the card moving or changing size.

#### Scenario: Thumbnail first
- **WHEN** the card opens for a record whose thumbnail is loaded and whose large cover isn't cached
- **THEN** the card shows the thumbnail in the square at once, and the large cover in its place when it arrives

#### Scenario: No large address
- **WHEN** the record came from a seller listing, which gives no large image
- **THEN** the card shows the thumbnail in the square, and no large cover is requested

#### Scenario: No image
- **WHEN** the record has no image
- **THEN** the card shows the record icon in the square

### Requirement: Large covers
A large cover SHALL be at most 600 pixels wide and high, fetched by the background cover worker only for the record whose card is open, once the card has shown that record for 250 ms (so stepping quickly fetches only where it stops), never for rows in view. It SHALL be kept on disk in the cache folder apart from thumbnails, so it is fetched only once. A large cover that couldn't be fetched SHALL NOT be tried again before the next launch, and the card SHALL keep the thumbnail.

#### Scenario: Only for the card
- **WHEN** in a grouped crate, 12 record rows are in view and no card is open
- **THEN** their thumbnails are fetched and no large cover is

#### Scenario: Card opened
- **WHEN** the user opens a record's card and leaves it on that record for 250 ms
- **THEN** its large cover is fetched next, ahead of the thumbnails of the rows in view

#### Scenario: Stepping quickly
- **WHEN** the user presses → five times quickly from the first record
- **THEN** only the sixth record's large cover is fetched

#### Scenario: Cached
- **WHEN** the card opens for a record whose large cover was fetched in an earlier session
- **THEN** it is shown with no request to the network

#### Scenario: Failed fetch
- **WHEN** a large cover's address answers 404
- **THEN** the card keeps the thumbnail, and that address isn't asked again this session

### Requirement: Large cover fetching pace
Large covers SHALL be fetched from the image host, never the Discogs API host, and SHALL NOT use its request budget. They SHALL share the thumbnails' pace: one at a time, at most 4 per second across both sizes, with the open card's covers first. Fetching them SHALL NOT affect playback.

#### Scenario: Shared pace
- **WHEN** thumbnails of rows in view and the open card's large cover are waiting
- **THEN** no more than 4 images are fetched per second in all

#### Scenario: No API budget
- **WHEN** a large cover is fetched
- **THEN** no request goes to the Discogs API host

#### Scenario: Playback unaffected
- **WHEN** large covers are fetched while a track plays
- **THEN** playback has zero underruns

### Requirement: Large cover addresses
Each Discogs entry SHALL carry its record's large image address when the data intake reads gives one (collection and wantlist results, and release or master data), saved with the crate. A Bandcamp entry's large address SHALL be derived from its art address. Large covers SHALL come only from the hosts thumbnails are allowed from (Discogs' and Bandcamp's image hosts). A crate saved without large addresses SHALL load unchanged, and no request SHALL be made only to fill them.

#### Scenario: Collection
- **WHEN** the collection is read and a record's data has a cover image
- **THEN** its entries carry that address as their large cover, and it survives a restart

#### Scenario: Older crate
- **WHEN** a crate saved before large covers existed is shown and a record's card is opened
- **THEN** the card shows the scaled-up thumbnail, and no request goes to the Discogs API

#### Scenario: Bandcamp
- **WHEN** a Bandcamp entry's art is `https://f4.bcbits.com/img/a0161524395_7.jpg`
- **THEN** its large cover is fetched from the same art id at a size of at most 700 pixels
