## Context

- **The wantlist today** is a side effect of Keep, which works per clip. `dig_keep_now` (`ui/src/app/digging.rs`) copies the clip to the Keepers crate and records `memory.kept[clip] = Kept { release, added_to_wantlist }`. It then sends `Command::Keep(release)`, which becomes a `PUT /users/{u}/wants/{release}` in `intake.rs::change_wantlist`. The worker reads the wants set once a session and skips the PUT when the release is already there. Un-keep sends a DELETE only if `added_to_wantlist`.
- **Pending wantlist changes:** `memory.wantlist_pending: Vec<(release, WantOp)>` is saved and re-sent every 60 s while a token is set (`dig_retry_wantlist`), **with no limit**. Offline and `Other` errors re-queue; anything else is dropped with a message.
- **Owned records:** `Collection::owned(release, master)` matches this pressing or another pressing of the same master. Keeping an owned record opens `confirm_keep` ("Keep anyway?").
- **The collection** is read-only. `collection.rs::sync` reads pages newest first and stops at the first known `instance_id`. It reads everything again when `old.count + added != total`, so it catches records removed and records missed.
- **Crates:** `CrateInfo.collection: bool` pins a crate under DISCOGS in the sidebar and the crate menu, in amber (`pl_owned`). The Keepers crate is an ordinary crate, found through `settings.keepers` or by name. Sending a wantlist page creates "Wantlist: ‹user›" (`PageKind::provisional_name`).
- **Transport:** `Method::{Get, Put, Delete}` only. The fake transport answers non-GET calls with 201 or 204.
- **Context menu:** when the clicked entry is selected, Remove and Send to crate act on the selection. Everything else acts on the clicked entry.
- **Discogs API facts relied on:**
  - `PUT` / `DELETE /users/{u}/wants/{release}` are idempotent.
  - `POST /users/{u}/collection/folders/{folder}/releases/{release}` adds a **new instance** on every call, and answers `{instance_id, resource_url}`. Folder 0 ("All") can't be written; folder 1 is Uncategorized.
  - `GET /users/{u}/collection/releases/{release}` lists that release's instances in every folder.

## Goals / Non-Goals

**Goals:**
- Wantlist and collection changes from the player, one release per action, on a single entry or a selection.
- The wantlist crate and the Discogs wantlist say the same thing, and nothing owned stays wanted.
- No duplicate collection copies, ever.
- Every failure ends visibly. Nothing stays "pending" forever.
- Nothing new on the playback path: all requests go through the intake worker and its rate limiter.

**Non-Goals:**
- The grouped record view and row covers (change `record-view`).
- Removing from the collection, folders, ratings, notes or media condition.
- Bridge or extension endpoints for writes.
- Appending records bought outside the app to the collection crate.
- Several Discogs accounts.

## Decisions

**1. The unit is the release, not the clip.**
- What gets wanted is a record. "Wanted" is a per-release state: `memory.wanted: BTreeSet<u64>`, seeded from the Discogs wants set once it's read. It replaces `memory.kept`.
- The ✓ on a row now means that its release is wanted. Pass stays per clip.
- **Add to wantlist** does three things:
  - inserts the release into `wanted`;
  - queues `Want(release)`;
  - sends the release page to the wantlist crate in Crate mode. That reuses intake, gets every clip of the record and skips clips already there, with no request when the release is cached.
- **Alternative considered:** keep copying only the clicked clip. It was rejected because a wantlist crate that mirrors Discogs has to hold records, and change `record-view` groups by record anyway.
- Local files have no release, so the items are hidden for them. A master-only entry carries its main release (album-entries), so it uses that.

**2. Acting on a selection.** When the clicked entry is selected, the items act on the distinct releases of the selection; otherwise they act on the clicked entry's release.
- With a selection, the label counts records: "Add 3 records to wantlist".
- Releases that are owned, already wanted (for Add) or have no release are skipped, and the message says so: "Added 3 records to your wantlist; 2 skipped (already in your collection)".
- `Y` still acts only on the playing entry.

**3. Never want what you own.**
- Add to wantlist is **disabled** when `Collection::owned(release, master)` is `Some`, either this pressing or another. The item reads "In collection ✓", with a tooltip naming the owned pressing. `Y` says "‹record› is already in your collection (AF001R, 2019)".
- `confirm_keep` and its dialog are removed.
- **Alternative considered:** keep the confirm for another pressing. The user chose a hard rule.

**4. Add to collection.**
- `Command::Collect(release)` sends `POST /users/{u}/collection/folders/1/releases/{release}` and answers with `Event::Collected { release, instance }`.
- **Folder 1 only.** It's the default, the record shows under All on Discogs, and there's no folder UI and no extra request.
- **On success, in the UI thread:**
  1. `Collection::insert(instance, release, pressing)` adds the instance and the release, takes `master`, `catno` and `year` from the cached release JSON (no request), sets `count += 1`, re-indexes and saves. The OWNED badge then shows everywhere on the next frame.
  2. If the release is wanted, queue `Unwant(release)`. That happens whether or not this app added it.
  3. Remove the release's entries from the wantlist crate, and send the release to the collection crate in Crate mode.
- **Sync stays correct:** the new instance is the newest, so the next incremental sync stops at once and `count + 0 == total`. If a record was also added on discogs.com before ours, `total` is one more than expected and the existing rule reads everything again. That's correct, just one full read.
- **The item reads:**
  - "Add to collection";
  - "In collection ✓" (disabled) when this pressing is owned;
  - "Add to collection (own AF001R, 2019)" when another pressing is owned, without a confirm, because two pressings is a legitimate collection.
- **Without a token:** the Connect modal (decision 7) opens every time, since nothing happens locally.

**5. No automatic retry for collection adds.**
- A POST that timed out may have gone through, so re-sending it can add a second copy.
- On `Offline` or `Other`, the entry shows "⚠ collection add failed" and the main window shows the error once.
- The menu then offers **Retry add to collection**. A retry sends `GET /users/{u}/collection/releases/{release}` first: if the release is there, the newest instance is taken as ours and handled as a success; otherwise it POSTs.
- **Alternative considered:** always GET before the POST. It was rejected because it doubles the requests of the normal case to protect only the rare one.

**6. Wantlist retries with an end.** `wantlist_pending` becomes `Vec<Pending { release, op, attempts, next_at, failed: Option<String> }>`.
- `Offline`: no attempt is counted. It's retried when Discogs answers again (the existing offline/online tracking) or on the 60 s check.
- `Other` (5xx, an unexpected status): `attempts += 1`, `next_at = now + [1, 2, 5, 15, 60][attempts-1]` minutes. After the 5th failure, `failed = Some(error)`: the row shows "⚠ wantlist failed", the main window shows the error once, and the menu offers **Retry wantlist**, which resets `attempts`.
- 429 is already waited out by the limiter and isn't counted.
- `TokenRejected` and `NotFound` drop the change with a message, as today.
- An add and a remove of the same release cancel out, as today.

**7. The Connect to Discogs modal.**
- It opens on Add to wantlist without a token, after the record has gone into the "Wantlist" crate. It stays closed once `settings.connect_hint_dismissed` is set; after that, the one-line message is shown instead.
- For Add to collection without a token it opens every time, without the checkbox.
- **Text, in substance:** "If you have a collection on Discogs, you can keep it up to date from here: add records to your wantlist or collection while you dig, and see what you already own. It only takes connecting your account."
- **Steps:**
  1. open your Discogs developer settings, with a link to `https://www.discogs.com/settings/developers`, opened through `setup.browser`;
  2. Generate new token, then copy it;
  3. paste it in Options ▸ Discogs….
- **Buttons:** Later, and Connect…, which opens Options ▸ Discogs… with the token field focused.
- It's modal like `confirm_keep` was: no other shortcut acts while it's open.

**8. The wantlist crate is a crate kind.**
- `CrateInfo` gains `wantlist: bool` beside `collection: bool`, and `CrateInfo::discogs()` is either. Only one crate is the wantlist. Implementation chose a second flag over an `Option<Wantlist | Collection>` enum: old indexes load unchanged, and an older build still reads the collection flag.
- Sidebar and crate menu: the DISCOGS group lists Wantlist first, then Collection, both in `pl_owned` amber, with the record icon.
- **Naming and lifecycle:**
  - **Without a token:** the crate named "Wantlist" (from `settings.wantlist`, formerly `settings.keepers`) is an ordinary crate among the user's own.
  - **On a token saved for ‹user›:**
    - if "Wantlist: ‹user›" exists (it was sent before), it becomes the wantlist crate. The local "Wantlist" crate's entries are sent into it, and the local crate is deleted once that is saved;
    - otherwise the local crate is renamed "Wantlist: ‹user›".
    - In either case the crate is marked `Wantlist` and the wantlist page is sent into it (Crate mode, skip-duplicates).
    - Every release in it that isn't on Discogs and isn't owned is queued as `Want`.
  - **Refreshing the mirror:** once a session, when the account is first known, the wantlist is read (`Command::ReadWants`, the worker's once-a-session read). The first time for an account (`memory.synced_user` isn't it), local wants that aren't on Discogs and aren't owned are pushed. After that, Discogs is followed: local wants missing there, with no change waiting, are dropped, and their entries leave the crate. Records wanted but missing from the crate are fetched into it, one release send each, or the wantlist page when there are more than 20. These sends are quiet (no "Digging…" message).
  - **Knowing the account:** a dig learns it as a side effect. When there is something to follow (wanted records, waiting changes, or the wantlist crate on screen) and no dig has asked, `Command::Identify` asks once.
- **Removing:**
  - when Remove, Remove album or the Delete key would leave **no** entry of a wanted release in the wantlist crate, a dialog asks "Remove N records from your Discogs wantlist?" [Remove] [Cancel]. Remove queues `Unwant` for each;
  - removing only some clips of a record is local;
  - without a token, no dialog;
  - deleting the whole crate never touches Discogs.

**9. Sync cleans the wantlist.**
- After a successful collection sync, the worker reads the wants set if needed and computes the wants that `owned()` matches (either pressing).
- For each, it queues `Unwant`, takes it out of `wanted` and removes its entries from the wantlist crate.
- The main window says "Removed 2 records you now own from your wantlist". The cost is one DELETE per record, through the limiter.

**10. Requests per action, with a token.** One identity check and one wants read per session, already the case today, plus:
- Add to wantlist: 1 PUT;
- Add to collection: 1 POST, plus 1 DELETE when wanted;
- a retry of a collection add: 1 GET, plus maybe 1 POST.

None of this is sent before the window is interactive.

## Risks / Trade-offs

- [Mirror removal deletes local entries the user may want to keep] → Only entries whose release left the Discogs wantlist are removed, only after a successful wants read, and never while that release has a pending change.
- [A collection sync that started before an add returns a collection without it] → Adds made while a sync runs are kept and inserted again into its result.
- [Treating the newest instance as ours on a retry could pick a copy added on discogs.com in the meantime] → Harmless: the effect is the same (owned, unwanted), and no copy is added.
- [Migration renames or merges a user's crate] → It happens once, only on the first token save after upgrading. The merge deletes the local crate only after the target is saved, and a crate that can't be read is never merged.
- [Removing the per-clip ✓ changes what users see on old Keepers entries] → `memory.kept` is migrated: every kept release becomes wanted, so their ✓ stays. Kept local files stay in the crate without a ✓.
- [Hard "owned blocks" may annoy someone hunting a specific pressing] → It's the user's explicit rule. They can still add it on discogs.com.

## Migration Plan

1. `settings.keepers` → `settings.wantlist`. On load, the Keepers crate is renamed "Wantlist" unless another crate already has that name, in which case it keeps its name and is still the wantlist crate.
2. `memory.kept` → `memory.wanted` (its releases). `added_to_wantlist` is dropped. Old `wantlist_pending` pairs load as `Pending` with `attempts = 0`.
3. `CrateInfo.collection: true` reads as `discogs: Some(Collection)`, and only the new field is written.
4. With a token already saved, step (8)'s "token saved" path runs once at the first launch after upgrading, behind the window being interactive.

**Rollback:** an older build reads `settings.ron` and `memory.ron` with `#[serde(default)]`, so it ignores the new fields and loses only the kept marks. The index file's `discogs` field is unknown to it, so the DISCOGS group shows only Collection again.

## Open Questions

- Should records found by a sync (bought on discogs.com) also be appended to the collection crate? That's left out here.
- When a wantlist crate entry's record becomes owned, it leaves the crate. Should it leave a note in the main window per record, or is one summary line enough (decision 9)?
