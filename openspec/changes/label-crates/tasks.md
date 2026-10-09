## 1. Label crates in the model

- [x] 1.1 `CrateInfo.label: Option<u64>` (serde default, skipped when `None`); `Crates::label_of`, `set_label`, `find_label`, `labels()` (creation order); grouped by default like seller crates; unit tests (survives a reopen, survives a rename, an index without the field loads, grouped by default)
- [x] 1.2 `Crates::is_locked` (wantlist, collection, label); `refuse_discogs_insert` / `refuse_discogs_edit` use it, with the label wording; headless tests that paste, file drop, entry drop on the sidebar crate, Send to crate, Delete, drag-out and Clear crate all leave a label crate unchanged and say why, and that N dims a track there and the next one starts

## 2. Following and refreshing

- [x] 2.1 `dig_follow_label(page)`: find or create "Label: ‹name›" (numbered when taken), `set_label`, `dig_send_to` with skip passed; never shows the crate or touches playback; the crate takes the label's real name when the intake learns it; headless test (follow fills under LABELS, shown crate and playback unchanged; following again makes no second crate)
- [x] 2.2 Refresh label in the label crate's menus (sidebar, title-bar crate menu), reading "Refreshing…" (disabled) while running; `label_refresh` tracks the job and the release ids before; on `Finished` "‹crate›: N new records" / "up to date", on `Failed` "refresh failed: ‹reason›"; headless tests (new records counted, nothing doubled, passed tracks stay dimmed, offline changes nothing)

## 3. The label crate's menu

- [x] 3.1 The label crate menu, in the sidebar and the title-bar crate menu: Delete label…, Export to crate ▸, Refresh label, Download all tracks (no Rename crate…, no Delete crate…); Export to crate lists normal crates and New crate… (name dialog suggesting the label's name), copies every entry, skips what the target holds, shows a new crate; headless tests (menu items; export to new and existing crate; label crate unchanged)
- [x] 3.2 Delete label…: confirmation worded for labels, then delete; passes kept; headless test (crate gone, a later follow leaves passed tracks out)
- [x] 3.3 Move to Labels on a normal, non-empty crate whose every entry has an `Origin.page` of the same label, while that label isn't followed; headless tests (moved crate keeps name and entries and becomes locked; mixed crate and crate with a local file aren't offered it)

## 3b. Download all tracks

- [ ] 3.4 Preview worker: `PreviewCommand::Background { clips }`, downloaded after the horizon in the same 2 slots, only while the cache is under its limit, never evicting for them; pauses with `PreviewEvent::CacheFull`; `SetLimit` resumes; an empty list stops; unit tests in `scheduler.rs` (horizon first, background after, pause instead of evicting, resume on a larger limit, stop)
- [ ] 3.5 App: Download all tracks / "Stop downloading (N of M)"; entries waiting for a search are searched after the window's; progress from `Done` / `Failed`; the cache-full modal (Raise to 2× / Stop / Options ▸ Discogs…); the end message with skipped ones; headless tests (all downloaded with another crate playing, cache full asks and Raise resumes, Stop)

## 4. Sidebar

- [x] 4.1 LABELS group between the collection crate and TOP SELLERS, hidden when empty, rows like seller crates; heading folds with ⏵/⏷ and the count, saved in `Settings.labels_folded`; right-click a label crate opens its menu (3.1); headless tests (order of the groups, fold survives a restart, menu items)

## 5. Bridge and extension

- [x] 5.1 `Snapshot.labels` (followed label ids, not serialised); `send` answers label pages, whatever the mode, with `label`, `added` and "Added label ‹name›" / "Refreshed label ‹name›", no bring-to-front; the UI routes bridge label sends to `dig_follow_label`; tests in `crates/dig/tests/bridge.rs` and a headless test (three labels sent in a row, three crates, playback and shown crate unchanged)
- [x] 5.2 Extension: `content.js` label pages show one item, "‹App›: Send label"; `background.js` link menus split (Play/Enqueue for non-label links, Send label for label links), with the label pattern from `pages.js`; run the extension's manual checklist in the README for label pages and links

## 6. Docs and checks

- [ ] 6.1 README (Labels: following from the browser, refresh, the menu, Move to Labels; the extension's label button; paste unchanged; test count), the extension's manual checklist, and the help panel; run `cargo test --workspace`, clippy, fmt and the wasm check
