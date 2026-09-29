## Why

While digging it's easy to fall for a record you already own, and to buy it twice. The app knows the user's Discogs account (through the token), so it can know their collection and say so in the list.

## What Changes

- **Collection sync, with as few requests as possible:**
  - Only with a token. Without one, nothing is fetched and nothing is marked.
  - The first time, the whole collection is fetched, 100 records per request, and cached on disk.
  - After that, the sync is **incremental**: newest additions first, stopping at the first record already known. A normal week costs 1 request. It re-fetches everything only when the collection's total shows records were removed.
  - It syncs only when there is something to mark (a crate with Discogs entries is shown) and the cache is older than 7 days, or on demand from OPT ▸ Discogs…, which also shows the collection's size and age.
  - Other pressings are never searched for: the match uses the master id already known from the release data fetched while digging.
- **Owned mark:** an entry whose record is in the collection is clearly marked **OWNED**, in a distinct colour, in the playlist. That covers the same release, and also another pressing of the same master, which counts as the same vinyl. The tooltip says which: "Owned: this pressing", or "Owned: another pressing (AF001R, 2019)". The mark is worked out when drawn, from the cached collection, so it's never stale in saved crates.
- **Keep asks first:** keeping an owned entry (`Y` or the entry menu), which adds its release to the Discogs wantlist, first opens a dialog: "You already own this record … Keep anyway?", with Keep anyway and Cancel. Undoing a keep never asks.
- **In the browser:** on a release, master or marketplace item page, the extension's button shows "✓ In your collection" (or "✓ Another pressing in your collection") when the record is owned. It asks the player through the bridge, which answers from the cached collection, with no Discogs request for release and master pages. A marketplace item costs one lookup the first time it's seen (to learn its release), and is remembered for good.
- **A crate of the whole collection:** saving a token sends the user's collection into a crate "Collection: ‹username›" (once) and shows it. The user's collection page on Discogs (`/user/‹name›/collection`) gets the extension's button too, so it can be sent again. That's a normal dig: one request per 100 records to list them, then one per record for its clips, nearest the playhead first and cached. It only happens on those two actions.

## Capabilities

### New Capabilities
- `discogs-collection`: fetching and caching the user's collection, and marking owned entries.

### Modified Capabilities
- `dig-verdicts`: Keep asks for confirmation when the record is owned.
- `discogs-intake`: a crate created by a send is shown, with the playlist opened.
- `browser-bridge`: a paired extension can ask whether a page's record is owned.
- `chrome-extension`: the page button shows when the page's record is owned.

## Impact

- `crates/dig`: a collection fetch through the existing client and rate limiter (low priority, behind expansion work); a `collection.ron` cache in the cache folder (release ids, and master id → owned pressings).
- `crates/ui/src/app/digging.rs`: refresh timing, the OPT ▸ Discogs… line and button, the Keep confirmation dialog.
- `crates/ui/src/app.rs` and `format.rs`: the OWNED badge on rows (single line and columns), and the tooltip line.
- No requests before the window is interactive (the launch target stays under 300 ms). No playback-path changes.
