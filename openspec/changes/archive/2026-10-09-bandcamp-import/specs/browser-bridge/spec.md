## MODIFIED Requirements

### Requirement: Discogs references only
A send request SHALL contain exactly:
- a Discogs or Bandcamp page address;
- a mode (Play, Enqueue or Crate);
- a crate name of 1 to 40 characters when the mode is Crate;
- the skip passed switch;
- for a Bandcamp page only, optionally, the page's title, at most 200 characters, used only to name the label.

A `vinyl_only` field, which older extensions send, SHALL be accepted and ignored. Its body SHALL be at most 16 KB. The address SHALL be checked as a supported Discogs page, or as a Bandcamp page passing the check in `bandcamp-intake`, before anything happens. A request with anything else (a file path, another kind of address, an unknown field) SHALL be refused, and nothing SHALL be added.

Seller pages:
- A request whose address is a seller's page SHALL add or refresh that seller (see `seller-crates`), whatever its mode.
- Its answer SHALL name the seller and say whether it was added or refreshed.
- The app SHALL then ask the system to bring its window to the front, or, where the system refuses, to draw the user's attention to it.

Label pages, Discogs or Bandcamp:
- A request whose address is a label's page SHALL follow or refresh that label (see `label-crates`), whatever its mode.
- Its answer SHALL name the label and say whether it was added, merged into a followed label, or refreshed.
- The app SHALL NOT come to the front.

A Bandcamp album or track SHALL be handled like a Discogs release (see `bandcamp-intake`).

#### Scenario: Not a Discogs page
- **WHEN** a paired extension sends https://example.com/track.mp3
- **THEN** the request is refused as unsupported, and no crate changes

#### Scenario: Label sent
- **WHEN** a paired extension sends a label page with mode Enqueue, and the label isn't followed
- **THEN** the label is followed and its crate fills under LABELS, no tracks are added to the shown crate, the answer says "Added label Siesta Records", and the app stays behind the browser

#### Scenario: Older extension
- **WHEN** a paired extension sends a label page with `vinyl_only: true`
- **THEN** the send is accepted, and the label's crate gets its records in every format

#### Scenario: Seller sent
- **WHEN** a paired extension sends `https://www.discogs.com/seller/decks.de/profile` with mode Enqueue, and decks.de isn't in the list
- **THEN** decks.de is added to TOP SELLERS, no tracks are added to the shown crate, the answer says "Added seller decks.de", and the app asks to come to the front

#### Scenario: Bandcamp album sent
- **WHEN** a paired extension sends `https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep` with mode Enqueue
- **THEN** its tracks are merged into the shown crate (or into the label crate following that Bandcamp), and the answer names the album

#### Scenario: Bandcamp label merged
- **WHEN** a paired extension sends `https://analogicalforce.bandcamp.com/music`, and "Label: Analogical Force" follows the Discogs label
- **THEN** the answer says "Merged into Label: Analogical Force", and the app stays behind the browser

#### Scenario: Look-alike host
- **WHEN** a paired extension sends `https://bandcamp.com.evil.net/album/x`
- **THEN** the request is refused as unsupported, and yt-dlp is not run
