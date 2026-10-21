## Why

The Buy chooser's search links (`cart-everywhere`) land diggers on store search pages, and there, as on most store listings, each record has no Diggr button: the extension shows a ▶ per record only on Discogs marketplace listings and Bleep lists. Discogs and Bandcamp search pages have no button at all, and Juno, Beatport and Traxsource listings can only be sent whole. Early users expect to play the one record they found.

## What Changes

- **A ▶ after every record (or track) on store listings**, sending only that one with mode Play (added to the crate on screen and played), as the marketplace ▶ does:
  - **Discogs**: after each release or master title link on any Discogs page that lists records (search results, labels, artists, lists, wantlists, collections), not only marketplace items; it sends the link's address.
  - **Bandcamp**: after each album or track title link on bandcamp.com search results and on a Bandcamp account's music page; it sends the link's address.
  - **Juno**: after each product's title on Juno listings (search, label, artist, genre, chart), sending that record as the page's reader reads it.
  - **Beatport and Traxsource**: after each track title on their listings (search, chart, label, artist, genre, release), sending that track.
  - **Bleep**: the existing per-record ▶, now also on Bleep search results.
- The page's own Diggr button is unchanged where it exists.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities
- `chrome-extension`: "Play each record of a for-sale list" becomes "Play each record of a list", covering every store's listings and search results.

## Impact

- **`extensions/chrome`:** `content.js` (one ▶ placer per store, sharing the existing ▶ style and send), small "read one row" helpers in `juno.js`, `beatport.js`, `traxsource.js` beside their list readers; `bleep.js` checked on search results.
- **Player:** none. It already takes a single Discogs or Bandcamp address and single-record store sends.
- **README:** the extension section and its manual checklist (the extension has no automated tests).
