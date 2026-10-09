## Context

- **Top Sellers is the template.** A seller crate is pinned in its own sidebar group under DISCOGS, tied to its seller by `CrateInfo.seller` (it survives a rename), grouped by default, and has Refresh, Add and Remove. Its extra state (order, criteria, last dig) lives in `<config>/dig/sellers.ron`.
- **Hand-edit refusals** go through `refuse_discogs_insert` and `refuse_discogs_edit` in `app.rs`, which check `Crates::is_discogs` (the wantlist and collection crates).
- **Sends:** `dig_send` (Play / Enqueue / Crate) and `dig_send_to(page, crate)` (used for seller crates) queue an intake job. The UI gets `Event::Progress`, `Finished` and `Failed` per job. Since `partial-clips`, sending a page into a crate that holds its tracks adds nothing twice, and skip passed leaves passed clips out.
- **Entries remember their page:** `Origin.page` is the address they were sent from, so a crate made from a label send can be recognised.
- **The bridge:** `send` in `crates/dig/src/bridge/mod.rs` already treats seller pages specially, whatever the mode: add or refresh, answer "added", come to the front. `Snapshot.sellers` tells it which sellers are known.
- **The extension:** `content.js` already has a one-item menu for seller pages; `background.js` builds the Play and Enqueue link menus from `WR.LINK_PATTERNS`.
- Pasting in the player goes through `dig_paste`, which stays as it is.

## Goals / Non-Goals

**Goals:**
- Follow a label with one click from the browser, several in a row, without leaving the browser or disturbing playback.
- A label crate that can't lose tracks by accident: only its label fills it, and pass is the only way to set a track aside.
- A manual refresh that brings only what's new.

**Non-Goals:**
- Automatic or scheduled refresh.
- Sub-label handling beyond what the label page itself returns.
- A shared crate-kind type for all special crates (see Decision 1).
- Following artists or other page kinds.
- Changing paste (Cmd+V) for labels.

## Decisions

### 1. `CrateInfo.label: Option<u64>`, next to `seller`

A label crate is a crate with `label = Some(discogs_id)`, saved in the crates index (`serde(default)`, skipped when `None`, so older index files load unchanged). `Crates` gains `label_of(id)`, `set_label(id, label)`, `find_label(label) -> Option<CrateId>` and `labels()`, the label crates in creation order. `apply_grouped` treats label crates like seller crates (grouped by default).

No separate `labels.ron`. Unlike sellers there are no criteria and no last-dig time, and the order is the crates' creation order, so the index holds everything.

*Alternative:* one `enum CrateKind { Plain, Wantlist, Collection, Seller(String), Label(u64) }`. Cleaner, but it touches every place that reads the three existing fields and the saved index format. It's left for when a further kind arrives; this change adds one field the way `seller` was added.

### 2. One rule for "no hand edits"

`Crates::is_locked(id)` is true for the wantlist, collection and label crates. `refuse_discogs_insert` and `refuse_discogs_edit` check it, and pick their message by kind. For a label crate the message is "Label crates fill from their label: Refresh label brings new records, N passes a track". Paste into the shown crate, file drop, entry drop on a sidebar crate, Send to crate, Delete, drag-out and Clear crate already pass through these two checks. The task list includes going through each path with a headless test.

Pass, Undo pass, Y and I don't edit the crate's entries, so they are untouched.

### 3. Following, and the refresh, are a send into the label's crate

`dig_follow_label(page)` (for a `PageKind::Label(id)` page):
1. finds the label's crate with `find_label(id)`;
2. if there is none, creates "Label: ‹provisional name›" (numbered when the name is taken, for example by an old normal crate), and marks it with `set_label`;
3. calls `dig_send_to(page, crate)` with skip passed on.

It never shows the crate or touches playback. Refresh label calls the same function, so a browser send of a followed label *is* a refresh.

The crate's name follows the label's real name once the intake knows it, as the provisional name of a Play send does today.

### 4. Refresh summary from the job's events

Before a label send starts, the app records the set of release ids in the crate in `DigState.label_refresh: HashMap<CrateId, (JobRef, HashSet<u64>)>`. On `Finished(job)` it counts the crate's release ids not in that set, and notifies "‹crate name›: N new records" or "‹crate name›: up to date". On `Failed(job, e)` it notifies "‹crate name›: refresh failed: ‹reason›". The menu item reads "Refreshing…" (disabled) while the crate has an entry in `label_refresh`.

*Alternative:* count entries instead of records. Rejected: a record's tracks arrive one by one, so a count of records reads better ("4 new records") and matches the seller summary's wording.

### 5. Create a crate from label, Remove label, Move to Labels

- **Create a crate from label:** `crates.create(name)`, where name is the label crate's name without "Label: ", numbered if taken. Then `crates.send(label_crate, all_ids, new)`, the existing copy used by Send to crate, which keeps order and origins. Passes live in dig memory, keyed by clip, so the copy shows the same passes. The new crate is shown.
- **Remove label…:** the existing delete confirmation, worded for labels, then `crates.delete(id)`. Dig memory is untouched, so passes stay.
- **Move to Labels:** offered on a normal crate (not locked, not a seller crate, not empty) when every entry has an `Origin` whose `page` parses to the same `PageKind::Label(id)`, and `find_label(id)` is `None`. It calls `set_label(id, label)`. The crate keeps its name.

### 6. Sidebar group

LABELS sits between the collection crate and TOP SELLERS, built from `crates.labels()`, with the same row drawing as seller crates (name and count, record icon, the OWNED colour). Its fold state is `Settings.labels_folded` (serde default: unfolded). TOP SELLERS keeps its fold in `sellers.ron`, but labels have no file of their own, so theirs goes in the UI settings. The heading shows ⏵/⏷ and the number of labels. It is hidden when no label is followed.

### 7. Bridge and extension

- `Snapshot` gains `labels: Vec<u64>`. It is serialised in the `/v1/crates` answer as `"labels"`; older extensions ignore an unknown field.
- In `send`, a `PageKind::Label(id)` page, whatever its mode, answers `{"page", "crate", "label": id, "added": bool, "message": "Added label ‹name›" | "Refreshed label ‹name›"}`. It queues `BridgeCommand::Send` with the page, and the UI routes label pages from the bridge to `dig_follow_label`. Unlike the seller branch, there is no bring-to-front.
- `content.js`: on a label page, the menu holds one item, "‹App›: Add to Labels", or "‹App›: In Labels · Refresh" when the page's label id is in the crates answer's `labels`. Its click sends mode `enqueue`.
- `background.js`: Play and Enqueue link items get every link pattern except labels. A new "‹App›: Add to Labels" item gets the label-link pattern. `pages.js` exports the label pattern separately for this.

*Alternative:* a new bridge mode `label`. Rejected: the page kind already decides, as for sellers, and an older extension's Enqueue on a label page then does the right thing too.

## Risks / Trade-offs

- [The extension's Play/Enqueue on label pages disappear (BREAKING)] → It's a deliberate product choice; the README and the extension's options page say how to get a label as a normal crate (Cmd+V into a crate, or Create a crate from label).
- [Big labels (thousands of releases) take a while to fill] → Same as a label send today. Progress shows in the main window, and several labels share the rate-limited queue.
- [An old normal crate already named "Label: X"] → The new label crate gets a numbered name, and the old crate can be moved into Labels only while the label isn't followed yet.
- [Refresh summary counts releases, not tracks] → Matches the user's mental model ("new records").
- [A crate with local files or other pages mixed in] → Move to Labels isn't offered: it needs every entry to come from that label's page, so a moved crate never holds tracks its label wouldn't bring.

## Migration Plan

None. `label` is a new optional field in the crates index. Existing "Label: …" crates stay normal until the user moves them. Rollback: older builds ignore the field, and the crates load as normal ones.

## Open Questions

None blocking. Sub-labels behave as the Discogs label page returns them.
