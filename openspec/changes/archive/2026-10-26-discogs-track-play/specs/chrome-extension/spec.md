## ADDED Requirements

### Requirement: Play each track of a release page
On a Discogs release page, the extension SHALL show a small button before each track of the tracklist that has a position, and none on headings or tracks without one. Each button SHALL show:
- ▶ while that track isn't playing in the app: clicking it sends the page with mode Play and that track's position, and shows ⧗;
- ⧗ from that click until the app's status names that release and position as playing, and at most 30 s, after which it shows ▶ again;
- ■ while the app's status names that release and position as playing (paused included): clicking it stops the app's playback.

While any of its buttons shows ⧗ or ■, the page SHALL read the app's status once a second, and not at all otherwise, so a track ending, another track playing or the app stopping shows on the page within 1 s. Nothing SHALL be read from the tracklist but each row's position text, and nothing from any other page. Clicking a button SHALL not navigate. When the app isn't running or the browser isn't paired, a click SHALL give the same feedback as the page button's Play in ‹App›. The page's own button and owned note SHALL be unchanged.

#### Scenario: Play one track
- **WHEN** the user clicks ▶ before B2 on `https://www.discogs.com/release/5077187`
- **THEN** the button shows ⧗, the release is added to the crate on screen, B2 plays, and within 1 s of it playing its button shows ■

#### Scenario: Stop
- **WHEN** the user clicks ■ on the playing track
- **THEN** the app stops, and within 1 s the button shows ▶

#### Scenario: The app moves on
- **WHEN** B2 plays from the page and the app goes on to B3 by itself
- **THEN** within 1 s B2's button shows ▶ and B3's shows ■

#### Scenario: Not found
- **WHEN** the user clicks ▶ on a track for which no preview is found
- **THEN** its button shows ⧗, then ▶ again after at most 30 s, and the page stops reading the status

#### Scenario: Idle page
- **WHEN** a release page is open and none of its tracks is playing or waiting to play
- **THEN** the extension makes no request to the app from it

#### Scenario: Headings
- **WHEN** a release's tracklist has a heading row "Side A" with no position
- **THEN** that row gets no button
