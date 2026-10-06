## MODIFIED Requirements

### Requirement: Discogs references only
A send request SHALL contain exactly: a Discogs page address, a mode (Play, Enqueue or Crate), a crate name of 1 to 40 characters when the mode is Crate, and the skip passed switch. A `vinyl_only` field, which older extensions send, SHALL be accepted and ignored. Its body SHALL be at most 16 KB. The address SHALL be checked as a supported Discogs page before anything happens. A request with anything else (a file path, another kind of address, an unknown field) SHALL be refused, and nothing SHALL be added.

#### Scenario: Not a Discogs page
- **WHEN** a paired extension sends https://example.com/track.mp3
- **THEN** the request is refused as unsupported, and no crate changes

#### Scenario: Label sent
- **WHEN** a paired extension sends a label page with mode Enqueue
- **THEN** the label is sent exactly as if it had been pasted, and the answer names the label and the target crate

#### Scenario: Older extension
- **WHEN** a paired extension sends a label page with `vinyl_only: true`
- **THEN** the send is accepted, and the crate gets the label's records in every format
