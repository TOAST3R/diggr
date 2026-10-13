## MODIFIED Requirements

### Requirement: Bandcamp pages
The app SHALL accept Bandcamp addresses of a label or artist (`https://‹name›.bandcamp.com/` or `/music`), an album (`/album/‹slug›`) and a track (`/track/‹slug›`), where ‹name› is 1 to 63 lowercase letters, digits or `-`, and ‹slug› is 1 to 200 of the same. It SHALL also accept a fan's collection and wishlist pages on `bandcamp.com` (see `bandcamp-collection`). A query string or fragment SHALL be ignored. Any other address on `bandcamp.com`, or on any other host, SHALL be refused as Discogs addresses are, and nothing SHALL be added. They SHALL be accepted by paste (Cmd+V, or Ctrl+V on Linux and Windows) and from the browser extension.

#### Scenario: Album with a query
- **WHEN** the address is `https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep?from=search`
- **THEN** it is recognised as that album, and its tracks are read

#### Scenario: Not a Bandcamp page
- **WHEN** the address is `https://analogicalforce.bandcamp.com/merch`
- **THEN** a message lists the supported kinds of page, and no crate changes

#### Scenario: A hostile address
- **WHEN** the address is `https://evil.bandcamp.com.attacker.net/album/x` or `https://a.bandcamp.com/album/x;rm`
- **THEN** it is refused, and yt-dlp is not run

#### Scenario: A fan page
- **WHEN** the address is `https://bandcamp.com/toast3r`
- **THEN** it is recognised as fan "toast3r"'s collection, not refused
