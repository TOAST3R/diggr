## Why

Records bought or wishlisted on Bandcamp are invisible to Diggr: the fan pages (`bandcamp.com/‹user›`, `/wishlist`) are refused, yt-dlp doesn't read them ("Unsupported URL"), and the extension shows no button there. Bandcamp has no public API for them, but the endpoint its own site pages with answers without a login (`POST bandcamp.com/api/fancollection/1/collection_items` and `…/wishlist_items`, given the fan id the page embeds: 99 collection and 103 wishlist items read for the current user). That makes both lists importable into the crates the user already digs from, and onto their Discogs account where Discogs has the record.

## What Changes

- **Fan pages are accepted**: `https://bandcamp.com/‹user›` (the collection) and `https://bandcamp.com/‹user›/wishlist`, by paste and from the extension. The extension sends only the address.
- **Reading**: the app reads the fan page for the fan id and the first items, then pages the rest from Bandcamp's collection endpoint; each item's album or track page is then read through the existing Bandcamp album path, at least 1 s apart.
- **Merged into the Discogs crates**: the collection goes into "Collection: ‹you›", the wishlist into "Wantlist: ‹you›", with no doubles (the existing artist-title and catalogue-number merge). Without a Discogs token, a Bandcamp import creates these two crates.
- **Written to Discogs when Discogs has the record**: with a token, each item is looked up on Discogs (artist, title, catalogue number, and format: File for a digital purchase, the physical format for a package). Exactly one match that the collection (or wantlist) doesn't already hold is added to the Discogs collection (or wantlist). No match, or several, writes nothing. Nothing is ever removed from Discogs.
- **Two wantlists, OR**: a wantlist crate entry stays while the Discogs wantlist or the Bandcamp wishlist holds it, and leaves on Refresh only when neither does.
- **Import and refresh**: the first send imports; Refresh collection and Refresh wantlist also read the Bandcamp lists of an imported fan. Both refreshes show the progress window with a bar and Stop (Refresh wantlist gains one).
- **Import report**: when an import or refresh with Bandcamp ends, or is stopped, a report window lists what came into each crate, what was added to Discogs or was already there, and the items not added to Discogs (with links to their Bandcamp page and a Discogs search), and the items that couldn't be read. It can be copied and reopened.
- **Digital is not another pressing**: a File (digital) release in the Discogs collection marks only itself OWNED. It no longer marks the physical pressings of its master as "another pressing", and no longer removes them from the wantlist. **BREAKING** for Discogs-only users who relied on a File release marking its vinyl owned.

## Capabilities

### New Capabilities
- `bandcamp-collection`: fan collection and wishlist pages, reading them through the fan id and the paging endpoint, merging into the collection and wantlist crates, the two-source wantlist, refresh with progress and Stop, the Discogs lookup and write, and the import report.

### Modified Capabilities
- `bandcamp-intake`: the accepted Bandcamp addresses include fan collection and wishlist pages.
- `discogs-collection`: a File (digital) release is not "another pressing" of its master's physical releases, for the OWNED mark and for leaving the wantlist.
- `discogs-write`: Refresh collection and Refresh wantlist include the imported Bandcamp lists; Refresh wantlist shows a progress window with Stop; refresh items are offered without a token when a Bandcamp list was imported.
- `chrome-extension`: a button on Bandcamp fan collection and wishlist pages; the Bandcamp page button no longer covers "other Bandcamp pages" only by exclusion.

## Impact

- `crates/dig`: `bandcamp.rs` gains the fan page kinds and a parser for the page's data blob and the endpoint's JSON; a small fetcher over the existing `ureq` transport (yt-dlp `--write-pages` as the fallback if Bandcamp's bot check refuses it); a Discogs database search in `discogs/client.rs` (rate-limited like every request); `collection.rs` keeps each owned release's format so a digital release can be told apart; the imported fan and its last items saved under the config folder.
- `crates/ui`: `app/bandcamp.rs` and `app/digging.rs` for the import, the merge into the two crates, the per-entry source (Discogs wantlist, Bandcamp wishlist) and the progress and report windows.
- `extensions/chrome`: recognise the two fan pages.
- No change to playback, analysis or the visuals; reads and writes run off the UI thread.
- The Bandcamp endpoint is undocumented and may change; a failure keeps the crates as they are and says why.
- `README.md`: Bandcamp collection and wishlist, the report, and the test count.
