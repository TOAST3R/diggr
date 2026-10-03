## 1. Discogs worker (`crates/dig`)

- [x] 1.1 `Method::Post` in `transport.rs` (ureq `send_empty`), and the fake transport answering POST from fixtures (`{instance_id}`); tests
- [x] 1.2 `Command::Want/Unwant` (renamed from Keep/Unkeep, no "added by app" check) and `Command::Collect(release)`: POST to `/users/{u}/collection/folders/1/releases/{r}` and `Event::Collected { release, instance }`; tests with the fake transport (one request; token needed; offline)
- [x] 1.3 `Command::CheckCollected(release)`: GET `/users/{u}/collection/releases/{r}`; when present, answer `Collected` with the newest instance and no POST, otherwise POST; tests (timed-out-but-stored gives no second POST)
- [x] 1.4 `Collection::insert(instance, release, pressing)` (count + 1, index, atomic save) and pressing data from the cached release JSON; tests (restart keeps OWNED; the next sync takes 1 request; a web-added record forces a full read)
- [x] 1.5 After a successful sync, the owned wants: reuse the session's wants set (read if missing) and report `Event::OwnedWants(Vec<release>)`; tests (this pressing, another pressing, nothing)

## 2. Memory and migration

- [x] 2.1 `memory.rs`: `wanted: BTreeSet<u64>` and `Pending { release, op, attempts, next_at, failed }`; load old `kept` (releases become wanted) and old pending pairs; drop `added_to_wantlist`; tests
- [x] 2.2 Backoff schedule (1, 2, 5, 15, 60 min; offline isn't counted; failed after the 5th; add and remove cancel out); pure-function tests with fake time
- [x] 2.3 `CrateInfo.discogs: Option<DiscogsCrate>` (Wantlist, Collection), reading the old `collection: true`; `settings.keepers` → `settings.wantlist`; on launch, Keepers is renamed "Wantlist" unless the name is taken; tests with `TestDir`

## 3. Actions in the UI (`crates/ui`)

- [x] 3.1 Add to wantlist / Remove from wantlist per record: mark wanted, send the release to the wantlist crate (Crate mode), queue Want/Unwant; `Y` follows it; ✓ means wanted; remove `confirm_keep`; headless tests (three clips arrive; remove whoever added it; local file shows no items)
- [x] 3.2 Owned blocks: "In collection ✓" disabled with a tooltip naming the pressing; `Y` message; headless tests (this pressing, another pressing)
- [x] 3.3 Add to collection: menu states (add / own another pressing / in collection), and on `Collected`: cache insert, Unwant when wanted, leave the wantlist crate, send to the collection crate; headless tests (bought it; OWNED within one frame)
- [x] 3.4 Selection: distinct releases, counted labels, skip reasons in one message; headless tests (8 entries across 4 releases; 3 entries of 1 release make one POST)
- [x] 3.5 Failures: "⚠ wantlist failed" and "⚠ collection add failed" in the row and tooltip (`format.rs`), the error shown once, Retry wantlist (restarts the schedule), Retry add to collection (through CheckCollected); headless tests
- [x] 3.6 Pass on a wanted record says so and passes nothing; test

## 4. Wantlist crate as a mirror

- [x] 4.1 Sidebar and crate menu: the DISCOGS group with Wantlist above Collection, both in amber with the record icon; "Wantlist" among the user's crates without a token; headless tests
- [x] 4.2 On a saved token: rename to "Wantlist: ‹user›" or merge into an existing one (delete the local crate after the save), mark it Wantlist, send the wantlist page into it, push local releases that are neither wanted on Discogs nor owned; headless tests (connect with 5 local records, 1 of them owned, and 20 remote)
- [x] 4.3 Once-a-session refresh when first shown: take in new wants, drop entries whose release left the wantlist and has nothing waiting; headless test (removed on discogs.com)
- [x] 4.4 The "Remove N records from your Discogs wantlist?" dialog when Remove, Remove album or Delete empties a record from the wantlist crate; one clip and crate delete stay local; headless tests
- [x] 4.5 `OwnedWants` after a sync: Unwant each, unmark, leave the crate, one summary message; headless test

## 5. Connect to Discogs dialog

- [x] 5.1 The modal: text, steps, a link through `setup.browser` to the developer settings, Connect… (opens Options ▸ Discogs… with the token field focused), Later, "Don't show this again" (`settings.connect_hint_dismissed`); shortcuts blocked while it's open; headless tests (first time, not again with the one-line message, Add to collection always shows it)

## 6. Docs and checks

- [x] 6.1 README (Y, the entry menu, the wantlist crate and DISCOGS group, the dialog, retries, the test count), help panel (`help.rs`), remove the Keep and Keepers wording
- [x] 6.2 Run the app against a real account to try add to wantlist, add to collection, a selection, connect and offline; fmt, clippy, the workspace tests, the wasm check, `--click-test` and `--startup-time`

## 7. Refresh from Discogs

- [x] 7.1 `Command::ReadWants { fresh }` (a fresh read drops the session's wantlist); test
- [x] 7.2 Refresh collection: sync, then bring the collection crate in line (fetch new releases, remove gone ones), with a "Collection: N new, M gone" message, "Refreshing…" while it runs, and the error on failure; headless tests
- [x] 7.3 Refresh wantlist: a fresh read through the existing mirror, with a "Wantlist: …" message; headless test
- [x] 7.4 The items in the sidebar's crate right-click and in the title-bar crate menu, only with a token; headless tests; README

