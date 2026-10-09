## Why

Some tracks never get a preview: YouTube has no upload of them ("not found"), or the upload is dead ("clip failed"). Many of these releases, digital-only ones above all, are on the label's or artist's Bandcamp, with full-length streams. Sending a Bandcamp page into Diggr, the way Discogs pages are sent now, fills those gaps and brings in releases that Discogs doesn't list. Each track also shows where its sound comes from.

## What Changes

- Bandcamp album, track and label pages (`<name>.bandcamp.com/`, `/music`, `/album/…`, `/track/…`) can be pasted (Cmd+V) or sent from the browser extension.
- Album and track pages behave like Discogs releases: Enqueue adds to the shown crate, Play makes a crate, and Crate adds to the named crate. If a label crate matches the page's label, they merge into that crate instead.
- A Bandcamp label page behaves like a Discogs label page: it follows the label under LABELS, or refreshes it when it is already followed. The page's label is matched by name to a followed Discogs label. An exact match merges into that crate, and a close match asks Merge or Separate once and remembers the answer. With no match, a Bandcamp-only label crate is made.
- Each Bandcamp track is matched against the crate's entries by artist and title, narrowed by the catalogue number in the album title ("[AF070]"):
  - a match that already plays is skipped;
  - a match with no preview ("not found", "clip failed", "no clip") gets the Bandcamp audio;
  - a track that matches nothing becomes a new entry.
- Each entry shows its source: a **YT** or **BC** badge on the row, and "Source: …" in the tooltip. An entry that has both sources can be switched with Play from Bandcamp or Play from YouTube.
- Bandcamp entries get **Open on Bandcamp** in their menu. Entries without a Discogs release have Y, I and the other Discogs items disabled.
- Previews can come from Bandcamp (128 kbps MP3). They use the same cache, horizon, download slots, rate-limit pause and Download all tracks as YouTube previews.
- The extension asks for access to `*.bandcamp.com`, and shows its button on Bandcamp album, track and label pages and its menu items on Bandcamp links.
- The bridge accepts strictly checked Bandcamp addresses besides Discogs ones.

## Capabilities

### New Capabilities
- `bandcamp-intake`:
  - which Bandcamp pages are accepted and how they are read;
  - how their tracks become entries or are merged with what a crate holds;
  - the source badge and switching sources;
  - Open on Bandcamp.

### Modified Capabilities
- `label-crates`:
  - a label crate follows a Discogs label, a Bandcamp label, or both;
  - Bandcamp label pages follow or refresh a label;
  - the name match decides when to merge;
  - Refresh label reads every source the crate follows.
- `preview-fetch`: Safe invocation also allows a checked Bandcamp address, both for reading a page and for downloading a track.
- `browser-bridge`: a send may carry a checked Bandcamp address. A Bandcamp label page follows the label without bringing the app to the front.
- `chrome-extension`:
  - Minimal access adds `*.bandcamp.com`;
  - new requirements cover the button on Bandcamp pages and the menu on Bandcamp links.

## Impact

- `crates/dig`:
  - new `bandcamp` module: address check, reading through yt-dlp, track matching;
  - `preview/fetcher.rs`: a typed clip, YouTube or Bandcamp, with its own argument list and file name;
  - the scheduler and store: preview keys for Bandcamp tracks;
  - `bridge/mod.rs`: Bandcamp sends;
  - `intake.rs`: Bandcamp jobs on the same queue as Discogs sends.
- `crates/ui`:
  - `playlist.rs`: `Origin` gets the Bandcamp track address and the source in use;
  - `crates.rs`: a label crate's key covers a Discogs id and/or a Bandcamp name;
  - `app/digging.rs`: merging, refresh across sources, the Merge or Separate prompt;
  - `app.rs`: badge, tooltip and menu items.
- `extensions/chrome/`: manifest host permissions, `pages.js` patterns, `content.js` button on Bandcamp pages, `background.js` link menu.
- Saved crates gain fields with defaults, so older saves load unchanged.
- No new Rust dependency. yt-dlp is already required for previews.
- README: Bandcamp section, the badge, extension permissions, test count.
- Overlaps with `seller-pages` (not applied): both touch the extension. This change adds new extension requirements instead of modifying Page button and Links anywhere, so the two can land in either order.
