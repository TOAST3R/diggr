## Why

Juno Records stocks most of the new electronic vinyl being dug in Diggr, and every track on its pages has a 2-minute sample. Sending Juno pages into crates adds a third source beside Discogs and Bandcamp, one whose records can actually be bought, new, today. A spike showed that Diggr can't read juno.co.uk itself: every page is behind a Cloudflare challenge, and Diggr must not get around it. The samples, though, are served without a challenge. So the browser extension reads the page the user has open, and the app only downloads the samples it is about to play.

## What Changes

- On juno.co.uk product, label, artist, genre, chart and search pages, the extension shows its button: Play, Enqueue, Send to crate, skip passed.
- The extension reads the page in view (its schema.org microdata and sample links) and sends the records and tracks it shows. It sends only that page and opens no other one; to send more, the user raises Juno's items per page.
- The bridge accepts a strictly checked Juno send: records with their product id, artist, title, label, catalogue number, stock, price and tracks, each track with its sample path.
- Juno tracks are merged into crates like Bandcamp tracks: a match that plays is skipped, a match with no preview gets the Juno sample, and the rest are added.
- Previews can come from Juno: a 120 s, 128 kbps MP3 fetched with one plain HTTPS request from `mp3.juno.co.uk`, without yt-dlp. It is fetched only for the download-ahead window, like any preview.
- Each Juno record is linked to its Discogs release with one Discogs database search by catalogue number and label (with a token). A linked entry gets the Discogs verbs: Y, I, ★, owned. An unlinked one has them disabled, as a Bandcamp-only entry has.
- Entries show a **JN** badge and "Source: Juno" in the tooltip. They offer **Open on Juno** and show the stock and price seen on the page ("Juno: in stock, €14.78, seen 14 Oct"). A Discogs entry of a linked release shows the same note.
- The app never fetches a Juno page and never searches Juno.

## Capabilities

### New Capabilities
- `juno-intake`:
  - what a Juno send carries and how it is checked;
  - how records become entries and are merged into crates;
  - linking to Discogs;
  - the stock note;
  - Open on Juno;
  - downloading Juno samples and what happens when Juno limits or refuses.

### Modified Capabilities
- `browser-bridge`: a send may carry a checked Juno payload instead of an address, with its own body size limit.
- `chrome-extension`:
  - Minimal access adds `www.juno.co.uk`, and allows reading Juno pages' microdata and sample links;
  - a new requirement covers the button on Juno pages.
- `preview-fetch`: Safe invocation adds a rule for direct HTTPS downloads: only a Juno sample path rebuilt from its parts, only from Juno's hosts.
- `bandcamp-intake`: Source of the sound adds the **JN** badge, and switching between sources covers Juno.

## Impact

- `crates/dig`:
  - new `juno` module: payload types, checks, sample keys (`jn.‹product›-‹rel›-‹side›-‹track›`), catalogue number normalisation;
  - `preview/clip.rs`: `Source::Juno`, `Clip::Juno`;
  - `preview/fetcher.rs`: a plain HTTPS download path next to yt-dlp, through the existing `ureq`;
  - `bridge/mod.rs`: the Juno send;
  - `discogs/search.rs`: search by catalogue number and label;
  - a small Juno memory (product → release link, stock snapshot).
- `crates/ui`: `Origin` gains the Juno product and the stock snapshot. The row badge, tooltip and menu change, and merging reuses the Bandcamp path.
- `extensions/chrome/`: host permission, `pages.js` patterns, a Juno page reader, and the button on Juno pages.
- No new Rust dependency. yt-dlp is not needed for Juno.
- README: Juno section, the JN badge, extension permissions, test count.
- Overlaps with `seller-pages` (not applied), which also touches the extension. This change adds new extension requirements and modifies only Minimal access, so the two can land in either order.
