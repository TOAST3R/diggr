## MODIFIED Requirements

### Requirement: Discogs references only
A send request SHALL contain exactly: a Discogs page address, a mode (Play, Enqueue or Crate), a crate name of 1 to 40 characters when the mode is Crate, and the skip passed switch. A `vinyl_only` field, which older extensions send, SHALL be accepted and ignored. Its body SHALL be at most 16 KB. The address SHALL be checked as a supported Discogs page before anything happens. A request with anything else (a file path, another kind of address, an unknown field) SHALL be refused, and nothing SHALL be added. A request whose address is a seller's page SHALL add or refresh that seller (see `seller-crates`), whatever its mode. Its answer SHALL name the seller and say whether it was added or refreshed. The app SHALL then ask the system to bring its window to the front, or, where the system refuses, to draw the user's attention to it. A request whose address is a label's page SHALL follow or refresh that label (see `label-crates`), whatever its mode; its answer SHALL name the label and say whether it was added or refreshed, and the app SHALL NOT come to the front. The crates answer SHALL say which labels are followed, so the extension can show In Labels.

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
