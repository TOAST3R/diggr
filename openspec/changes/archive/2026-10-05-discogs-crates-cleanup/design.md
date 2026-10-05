## Context

The wantlist and collection crates (`Crates::is_wantlist` / `is_collection`) mirror the user's Discogs account, but they share every UI path with ordinary crates:

- `App::entry_menu` (`app.rs`) offers Play, Remove, Remove album, Select album and Send to crate to every crate, then `dig_entry_menu` (`app/digging.rs`) adds the Discogs items and Pass.
- `remove_entries` goes through `dig_before_remove`, which asks "Remove N records from your Discogs wantlist?" only when a removal in the wantlist crate would take away a whole wanted record.
- The only filter is the BPM range: `Playlist::bpm_filter` / `shows(e)` decides what is shown, and `Rows` (record view), play order, previews and the title bar count all go through `shows`.
- Each entry carries `styles: String` ("Deep House, Minimal"), taken from the release (its genres when it has no style) when the record's details were fetched. Entries only exist once those details are known, so every Discogs entry has its styles without a new request.
- The cached `Collection` (`crates/dig/src/collection.rs`) holds `count` (copies), `instances: BTreeSet<u64>` and `releases: BTreeMap<u64, Pressing>`. It doesn't know which instance belongs to which release, or in which folder.
- Writes (`Command::Want`, `Unwant`, `Collect`) run on the background Discogs worker, within the shared rate limit.

## Goals / Non-Goals

**Goals:**
- A style filter in the two Discogs crates that slots into the existing "shown entries" path, so play, previews, record view and counts follow it with no new code paths.
- Menus in those crates offer only actions that make sense for a record you want or own.
- Remove one copy of one record from the Discogs collection, safely: confirmed, never in bulk, safe to retry, and leaving the local cache consistent so syncs stay incremental.
- Those crates only change through Discogs-aware actions.

**Non-Goals:**
- A style filter in other crates, AND matching, or genre and label filters.
- Removing several records, or choosing which copy to remove.
- Changing the N key, or the wantlist and collection crates' sync rules.

## Decisions

### 1. The style filter is a second predicate inside `Playlist::shows`

`Playlist` gains `styles_on: BTreeSet<String>` (saved with the crate, like `bpm_range`, `#[serde(default)]`). `shows(e)` becomes `bpm_ok(e) && style_ok(e)`, where `style_ok` is true when the set is empty or any of the entry's styles (split on ", ") is in it. `style_filter()` returns the selection intersected with the styles the crate still holds, so a style that disappears stops filtering, which matches how `bpm_filter()` clamps a range that drifted.

*Why:* every consumer (the `Rows` cache key, next/prev, shuffle, previews ahead, the "2/4" title count, "k of N tracks") already reads `shows` and the filter revision. Adding a second predicate there gives all of them for free. *Alternative:* a separate style view layered on top. Rejected, because it would duplicate the play order logic that playlist-filters already got right.

The style list for the footer (`Playlist::styles() -> Vec<(String, usize)>`, counted per record (distinct album key), sorted by count then name) is cached in the app by crate and revision, so it is counted again only when the crate changes. It is computed over the whole crate, not over the BPM-shown entries, so chips don't vanish as the BPM range moves. The filter is offered only in a Discogs crate (`Crates::is_discogs`: the wantlist or collection crate of a connected account) whose records have at least two styles.

### 2. Footer layout: chips or one button, never a mix

The control goes after the BPM control, up to the time box. It measures the chips (skin font, LCD green, lit when on, dim when off, 5 px apart) against the room left. When all of them fit and there are at most 20, it draws them. Otherwise it draws "STYLES" plus the number selected (the skin font has no ▾). Clicking that opens an egui popup with a search field, a scrolling list of checkboxes with record counts, and Clear.

*Why not a "+N ▾" overflow chip:* it gives two places to find a style, and which ones end up in the overflow changes as the playlist is resized. One rule ("all fit, or the list") is easier to understand and to test. A click on a chip toggles it, a double-click on the control clears it (as on the BPM slider), and ≡ ▸ "Show all styles" clears it too.

### 3. Menus and keys by crate kind

`entry_menu` asks once whether the shown crate is a Discogs crate (`Crates::is_discogs`). When it is:
- Remove, Remove album and Pass are not drawn (Select album stays, because it's harmless and useful in flat view);
- in the collection crate, "Add to wantlist" / "In collection" and "Add to collection" are not drawn either, since they are always disabled there.

Delete and Backspace removal in a Discogs crate removes nothing (≡ hides Remove selected and Clear crate there), and the main window says once a session "Records leave this crate with Remove from wantlist / Remove from collection". The `confirm_unwant` modal is deleted, because nothing can remove a whole wanted record from that crate by hand any more. Without a token, the "Wantlist" crate isn't a Discogs crate: it is the user's own, and `dig_before_remove` keeps un-wanting a record whose last entry leaves it, without asking. `dig_forget_wanted` stays, since Unwant and sync use it.

*Alternative:* make Delete act on whole records with a confirmation. Rejected by the user: in these crates, removing is an account change and should be a deliberate menu choice.

### 4. Blocking hand-made inserts with one gate

`Crates::is_discogs(id)` (the wantlist or collection crate of a connected account) is the one predicate. Every user-initiated insert path checks it through `refuse_discogs_insert`: Send to crate (the crates aren't listed), drops on a sidebar crate (no drop highlight, a one-line hint on release), added and dropped files, a pasted address, and a browser send into the shown crate or naming the crate (refused with the same hint in the main window; the extension's crate list is unchanged). The app's own writers (wantlist add, collection add, sync, refresh, the token merge) insert through the existing internal methods, which bypass the gate.

*Why one gate:* a single predicate is easy to test path by path, and adding a new insert path later won't silently skip the rule.

### 5. Remove from collection: a worker command that finds its own copy

New `Command::Discard { release: u64 }`:
1. `GET /users/{u}/collection/releases/{release}` returns the instances, with `instance_id`, `folder_id` and `date_added`;
2. picks the most recent `date_added` (ties broken by the highest `instance_id`);
3. `DELETE /users/{u}/collection/folders/{folder}/releases/{release}/instances/{instance}`.

The answer event is `Discarded { release, instance, remaining }`, or a failure. If the GET finds no instance, the outcome is `Discarded` with `remaining = 0` and no DELETE is sent, because the copy is already gone. That's what makes a retry safe. A 404 on the DELETE is handled the same way.

*Why look it up each time rather than cache release → (instance, folder):* the cache would need a format change and would still be stale whenever the user moved a copy between folders on discogs.com. The lookup costs one extra request for a rare, user-initiated action, and it is always right.

On `Discarded`, the UI updates `Collection`: it removes `instance` from `instances` and lowers `count` by 1. When `remaining == 0`, it removes `releases[release]` (and its `by_master` entry), and the release's entries leave the collection crate. The cache is saved atomically. OWNED marks recompute from the cache, so another owned pressing of the same master keeps its entries OWNED elsewhere.

The menu item appears only in the collection crate, with a token, for a single record. When the selection spans more than one record, it isn't shown. It opens a modal naming the record and pressing, saying that one copy is removed (and how many remain, when the cache knows of more), and that its notes and rating on Discogs are lost. Remove/Enter and Cancel/Esc work as in the other dig modals, and `dig_asking()` includes it so shortcuts wait.

Failures follow the wantlist pattern, not the collection-add one: a removal is idempotent. While offline it waits, after server errors it is retried 1, 2, 5, 15 and 60 min later, and then the entry shows ⚑ "collection removal failed" with "Retry remove from collection". The pending removal is kept in dig memory across restarts.

## Risks / Trade-offs

- [The user removes the wrong record, and the notes, rating and date added are lost on Discogs] → The confirmation names the record and pressing, removal is one record at a time, and only one copy goes.
- [The user expects Delete to work and thinks the app is broken] → A one-time hint in the main window names the menu items. The help panel and README say so.
- [The local count goes out of step if the user also removed a copy on discogs.com] → The next sync's existing total check reads the whole collection again, as it does for adds.
- [Blocking inserts breaks a habit, such as sending a label page into the wantlist crate] → The hint says to use Add to wantlist (Y). Moving entries out still works.
- [Many styles (150+ in a big collection) make the popup long] → Search box, record counts, sorted by count. Measured on a 5,000-entry crate: style counting is cached by revision, and changing the selection updates the list within one frame (16 ms).
- [Existing saved crates have no style selection] → `#[serde(default)]` loads them unfiltered.

## Migration Plan

No data migration. Crate files gain an optional `styles_on` field, and dig memory gains a list of waiting removals. Both default to empty, and older builds ignore the field. Rolling back leaves the Discogs collection as it is. A removal still waiting is simply never sent.

## Open Questions

None blocking. Whether "Show all styles" belongs in the ≡ menu, next to "Show all tempos", can be settled during implementation.
