## Why

Bleep (bleep.com, once bleep.co.uk, Warp's shop) is where a lot of electronic, experimental and leftfield music is sold, on vinyl, CD and as files, and much of it isn't on Beatport or Traxsource. A digger browsing its new releases or a label's catalogue still has to listen there one record at a time, and can't bring those records into Diggr the way Juno, Beatport and Traxsource pages already come in. Bleep is also a place the user buys from, which the `store-carts` change builds on.

## What Changes

- **Bleep pages from the extension:** on release, label, artist, search and other list pages of bleep.com, the extension's button reads the records the page already shows and sends them over the bridge, as for Juno; on lists, a ▶ after each record plays that record alone. The app never fetches a Bleep page.
- **Records from lists:** a list shows no tracks, so a list record comes in as one entry; with a Discogs token, when exactly one release fits, that release's tracks replace it.
- **Previews:** found by the YouTube preview search, as for Traxsource: Bleep issues its preview addresses per request, behind its challenge.
- **Like Juno:**
  - **Merging:** entries merge into crates without doubles, and fix Discogs entries that are "clip failed" or "not found";
  - **Discogs links:** a record links to Discogs when exactly one release fits, so Y and I work;
  - **Stock and price:** each format's price is remembered with its date ("Bleep: vinyl £22.99, digital £7.99 (seen 18 Oct)");
  - **Menu:** Open on Bleep; a Bleep track plays from YouTube, "found for Bleep".
- **Pasted Bleep addresses** are refused with a hint to send them from the extension.

## Capabilities

### New Capabilities
- `bleep-intake`: what a Bleep send holds, its records and tracks in a crate, previews (Bleep samples or search), Discogs linking, the stock and price note, and Open on Bleep.

### Modified Capabilities
- `browser-bridge`: a send may also be a Bleep send, checked like a Juno send.
- `chrome-extension`: access to bleep.com, and a button on its pages.
- `bandcamp-intake`: "Source of the sound" adds "found for Bleep".

(`browser-bridge` and `chrome-extension` are also modified by `mix-tracklists` and `artist-follow`; whichever is archived later is rebased on the earlier ones. `store-carts` builds on this change.)

## Impact

- **`extensions/chrome`:** a Bleep reader (`bleep.js`) run only when the menu opens, the button on Bleep pages, `host_permissions` for `*.bleep.com`, saved fixture pages for the manual checklist.
- **`crates/dig`:** `bleep.rs` checking a Bleep send into `stores::StoreRecord`s; `Store::Bleep` and `bleep.ron` in the cache; the bridge's new send kind.
- **`crates/ui`:** list records as one entry and their Discogs tracks once linked, tooltips, Open on Bleep, the refused paste.
- **README:** Bleep in "From the browser", the extension's site access, the cache table, test areas and count.
- **No change** to `audio`, `analysis` or `platform`.
