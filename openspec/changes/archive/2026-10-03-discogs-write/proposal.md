## Why

The player already knows your Discogs wantlist and collection: it reads the collection to mark OWNED records, and Keep quietly adds releases to the wantlist. But the collection can't be changed from here, "Keep" doesn't say it touches your Discogs account, the Keepers crate and the Discogs wantlist drift apart, and a record you bought can stay on your wantlist forever. Digging ends with buying, and the app should follow a record all the way: wanted, then owned.

## What Changes

- **Keep becomes "Add to wantlist" (Y).** **BREAKING:** the label, the help and the README change, and the Keepers crate becomes the wantlist crate.
  - It acts on the **record**, not the clip: the record's clips go to the wantlist crate, and the release goes on the Discogs wantlist.
  - When the record is already on the wantlist, the item reads **Remove from wantlist (Y)**.
  - It acts on the right-clicked entry, or on the selection when that entry is selected, with one request per release.
- **Add to collection**, a new entry-menu item:
  - adds the release to the Discogs collection, in Uncategorized, so it shows under All;
  - updates the local collection cache at once, so the OWNED badge appears with no sync;
  - takes the release off the Discogs wantlist, and moves its entries from the wantlist crate to the collection crate.
- **You never want what you own:**
  - owning any pressing of a record blocks Add to wantlist. The menu shows "In collection ✓", disabled, and the old "Keep anyway?" dialog goes away;
  - when a collection sync finds a wanted record that you own, the app removes it from the Discogs wantlist and says so.
- **The wantlist crate mirrors the Discogs wantlist:**
  - without a token, it's a crate named "Wantlist" among your own crates;
  - after you connect, it becomes **"Wantlist: ‹user›"**, shown in orange in the DISCOGS group above "Collection: ‹user›". It's filled from the Discogs wantlist, and records added before connecting are pushed up to Discogs;
  - removing a whole record from it removes it from the Discogs wantlist, after asking;
  - existing Keepers crates are migrated.
- **Connect to Discogs modal:** Add to wantlist without a token still adds the record locally. It then explains that you can keep your Discogs wantlist and collection up to date from here by connecting, with the steps and a link to Discogs' token page. It has a "Don't show this again" option, after which a one-line message is shown instead.
- **Refresh from Discogs:** right-clicking the wantlist or collection crate (in the sidebar, or the title-bar crate menu) offers Refresh wantlist / Refresh collection. Each makes the crate match Discogs: new records come in, and records gone from Discogs leave.
- **Retries that end:**
  - while Discogs is offline, wantlist changes keep waiting;
  - after server errors, they back off over 1, 2, 5, 15 and 60 minutes, then stop. The entry shows "⚠ wantlist failed", the main window shows the error, and the menu offers Retry;
  - collection adds are never retried automatically, because each attempt can add a copy. A manual retry checks first whether the add went through.
- Out of scope:
  - the grouped record view (change `record-view`);
  - removing from the collection;
  - choosing a collection folder;
  - bridge and extension endpoints.

## Capabilities

### New Capabilities
- `discogs-write`: changing the user's Discogs account from the player. It covers Add to wantlist and Add to collection (on records and selections), the never-want-what-you-own rule, the wantlist crate as a mirror, the Connect to Discogs modal, and retries and failure reporting.

### Modified Capabilities
- `dig-verdicts`: Keep is replaced by Add to wantlist. The owned dialog is removed. "Undo keep" is replaced by Remove from wantlist (no longer limited to releases this app added). Dig memory no longer records which releases the app added.
- `discogs-collection`: adding locally after a write without breaking incremental sync, and removing owned records from the wantlist after a sync.
- `crates`: the wantlist crate joins the DISCOGS group, above the collection crate, and Keepers is migrated.
- `playlist`: the entry context menu's Discogs items.
- `player-window`: Y is described as add to (or remove from) the wantlist in the keyboard shortcuts.

## Impact

- `crates/dig`:
  - `transport.rs`: `Method::Post`;
  - `intake.rs`: `Command::Want/Unwant/Collect/CheckCollected`, wantlist reads reused for mirroring;
  - `collection.rs`: insert one item locally, a release → instances map;
  - `memory.rs`: the pending queue with attempt counts and next-try times; `added_to_wantlist` dropped;
  - the fake transport and fixtures for POST.
- `crates/ui`:
  - `app/digging.rs`: the actions, the modal, the migration, retries; `confirm_keep` removed;
  - `crates.rs`: the crate kind (wantlist / collection) instead of the `collection` flag, migrated;
  - `app.rs`: the sidebar and the crate menu group, menu labels;
  - `format.rs`: "wantlist failed";
  - `help.rs`.
- README: Y, the menu, the wantlist crate, the modal, and the test count.
- Discogs requests: about one per action per release, plus one wants read per session (100 a page). Every request goes through the existing rate limiter, so no new load on the playback path.
