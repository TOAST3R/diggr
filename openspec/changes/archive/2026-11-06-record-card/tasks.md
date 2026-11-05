# Tasks

## 1. Details builders

- [x] 1.1 In `format.rs`, rename `entry_details` to `track_details(e, marks, now, with_record)`, leaving out Album, Style, Label, Cat#, Year and For sale when `with_record` is false. Verify with unit tests: the "Discogs entry" scenario (with_record = true, all lines, "fetched 3 h ago") and "Under an open record" (Side and Tempo, none of the record lines). The local-file case still lists the album and path.
- [x] 1.2 Add `RecordCard` and `record_details(members, marks, copies, now)` to `format.rs`, folding in `record_counts`. It holds: artist (credited), album, pressing line, styles (else genres), up to 8 tracklist lines with state (playing, paused, dim, normal) and "… N more", marks, a sale or copies line, and the counts. Verify with unit tests for each `record-card` contents, tracklist and status-strip scenario (Glasshouse EP, Various, Geogaddi, 14 entries → "… 6 more", seller copies "3 copies €9.00–€18.00", Mezzanine counts, no strip for a local album).

## 2. Hover routing and the track tooltip

- [x] 2.1 Strip the cover branch out of `entry_tooltip`, so it draws only the name and the grid. On the entry-row hover, call `track_details` with `with_record = !grouped`, and stop asking `covers.slot` there. Verify with headless tests (zero tooltip delay, as in `headless_tests.rs:862`): a flat-crate hover shows "Album:" and draws no cover image; a hover on a track under an open record shows "Side:" and no "Album:".
- [x] 2.2 Remove the `CoverSlot` path from track rows. A flat crate then asks the cover worker for nothing on hover. Verify with a cover-cache unit test: hovering 41 flat rows sends no request (the "Flat crate" scenario).

## 3. Record card drawing

- [x] 3.1 Add `record_card(ui, card: &RecordCard, covers: CardCovers, colors, scale)` in `app.rs` (or a new `app/card.rs`), painting in skin colours inside an overridden tooltip frame (`pl_bg` fill, a 1 px dimmed-text stroke). Layout: a 240 × scale cover square on the left; title lines, pressing line, style pills, tracklist and status strip on the right; at most 560 × scale wide. Verify with a headless test: hovering a record row shows the artist, album and tracklist text, and nothing else.
- [x] 3.2 Route the record-row hover to `record_card` (single records included) and drop its `entry_details` + "Record" line. Verify with headless tests: a single-entry record row shows the card, not "Side:"; a drag over record rows shows no card.
- [x] 3.3 Draw the cover square: the large cover if loaded, else the thumbnail scaled to fit (linear filter), else an empty frame while either is waiting, else the record icon. Verify with a unit test of the slot-choosing function covering all four cases, and check that the square's size doesn't change when the large cover arrives.

## 4. Large covers in the worker

- [x] 4.1 In `dig::cover`, add `CoverSize { Thumb, Large }` to requests, results and the failed/asked sets. Large covers are stored at most 600 px under `covers/large/` and fetch up to 8 MB; only the hover queue takes Large. Verify with the worker's fake-image tests: a Large hover writes `covers/large/release-1.png` at ≤ 600 px, `want()` never fetches Large, a 404 on Large leaves the thumbnail untouched and isn't retried, and the pace is still ≤ 4/s across both sizes.
- [x] 4.2 Make the hover queue hold the newest record with both sizes (thumb first). Verify with a test: resting on record A and then B fetches B's thumb and then B's large, and nothing more of A's after B is asked for.
- [x] 4.3 Add `cover::bandcamp_large(url)` (`_<n>.jpg` → `_16.jpg`, same host, still `allowed`). Verify with unit tests: `a0161524395_7.jpg` gives `a0161524395_16.jpg`, and a non-bcbits URL gives `None`.

## 5. Large cover addresses

- [x] 5.1 Add `cover_large: String` (`#[serde(default)]`) to `Origin`, and the same field to `RecordInfo` and `Listed`. Verify with a test that a crate RON saved without the field loads and keeps every other field.
- [x] 5.2 Fill it in `discogs/model.rs`: `basic_information.cover_image` (collection, wantlist), `cover_image` (search), and the primary image's `uri` (release, master), all through `thumb()`. Seller listings leave it empty. Verify with model unit tests on the JSON fixtures, including a spacer image giving empty.
- [x] 5.3 When a stale thumbnail's `fresh_cover` runs, refill `cover_large` from the same data. Verify with an intake test using the fake transport: one request refreshes both addresses.
- [x] 5.4 In `app/covers.rs`, add `card_slot(ctx, e) -> CardCovers`, asking for both sizes after the 250 ms rest, with a large-texture cap of 16 next to `KEEP`. Verify with cover-cache unit tests: no large request before 250 ms; an entry with no large address asks only for the thumb; the 17th large texture evicts the least recently drawn one.

## 6. Docs

- [x] 6.1 In README, update the tooltip and cover description (record card, track tooltip, `covers/large/`) and the test count in Tests. Verify with `grep -n "96\|tooltip" README.md`, which should show no stale description.
- [x] 6.2 Add the entries to `## Unreleased` in CHANGELOG.md with the `changelog` skill (a big record card on hover in grouped crates, sharper covers, lighter track tooltips).

## 7. Checks

- [x] 7.1 Run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, the wasm check for `audio`/`platform` and `cargo xwin check`, all clean.
- [x] 7.2 Run the app on a grouped collection crate: clicking a record opens the card under its row with a sharp cover within a second (a soft one at once); ⏵/⏸ play, pause and resume tracks with the card open; ‹ › and ← → step records and scroll the list; Esc and a click outside close it; it reads right at 1× and 2×; flat-crate hovers show the text tooltip only; playback has no underruns while covers load.

## 8. Click-open card: state and opening

These replace the hover card of 3.1–3.2 and the rested-hover requests of 4.2 and 5.4.

- [x] 8.1 Add `OpenCard { crate_id, key, since }` to the app and `Action::OpenCard(AlbumKey)` (open, switch, or close when already on that record). On a record row, a plain click pushes `SelectRecord` and then `OpenCard`; modifier clicks push only `SelectRecord`; the record row's hover shows nothing. Verify with headless tests: a click opens the card ("Lowtide Tapes · LT-012 · 1994" shows) and selects the record; a second click closes it; a Shift-click opens none; hovering a record row for 10 frames shows no card and no tooltip.
- [x] 8.2 Close the card each frame when its crate isn't shown, the crate isn't grouped, or no shown row has its key. Verify with headless tests: Shift+G closes it; a BPM filter that hides the record closes it; removing the record closes it; showing another crate closes it.
- [x] 8.3 Esc closes the card, and a click outside both the card and the record rows closes it. Verify with headless tests: Esc closes it and keeps the selection; a click on the player band closes it; a click inside the card (on its pressing line) keeps it open.

## 9. Click-open card: drawing and playing

- [x] 9.1 Make `card::show` an `egui::Area` (foreground, constrained) anchored under the record row's rect (above it when there's no room, the list's edge when the row is out of view), with the skin frame, a 128 × scale cover and a 420 × scale maximum width. Verify with a unit test of the anchor function (below, flipped above, clamped to the list edge) and the cover-size test updated to 128 / 256.
- [x] 9.2 Make the tracklist a scroll area capped at 8 lines, listing every shown entry, and drop "… N more" from `RecordCard`. Verify by updating the format test (14 entries → 14 tracks) and a headless test: with 14 tracks the card's height equals the 8-track card's.
- [x] 9.3 Give each track line a ⏵/⏸ button (none when unavailable) that pushes `PlayEntry`, `Play` or `Pause` as the spec says. Verify with headless tests: ⏵ on A2 plays A2 and the card stays open with A2's line showing ⏸; ⏸ pauses; ⏵ then resumes from the same position; ⏵ on a downloading track arms it; an unavailable track has no button.

## 10. Click-open card: stepping

- [x] 10.1 Add `Action::CardStep(i8)`: the previous or next record row, then select it, move the cursor to it, scroll the list by the least amount that shows it, and switch the card with `since` reset; no-op at the ends. Draw ‹ › beside the artist line, dimmed when disabled. Verify with headless tests: › from the first record shows the second and selects it; ‹ on the first record does nothing; stepping past the last row in view scrolls by one record; a playing track plays on.
- [x] 10.2 While the card is open, ← and → push `CardStep` instead of closing and opening records. Verify with headless tests: → steps to the next record and doesn't open the current one; after Esc, → opens the record under the cursor again.

## 11. Click-open card: covers

- [x] 11.1 Call `card_slot(ctx, e, since)` only for the open card's record, asking once the card has shown it for 250 ms. Verify by updating the large-cover headless test: no large cover while no card is open (rows in view fetch thumbnails only); opening the first record's card fetches its large cover; pressing → five times quickly fetches only the last record's large cover.

## 12. Docs and checks again

- [x] 12.1 Update README (record card on click, play buttons, ‹ › and ← →, Esc, 128 pt cover) and the test count, and rewrite the CHANGELOG entry for the click-open card that plays tracks and steps through records. Verify with `grep -n "Hover a record row" README.md`, which finds nothing.
- [x] 12.2 Run `cargo test --workspace`, clippy, fmt, the wasm check and `cargo xwin check`, all clean.

## 13. Feedback: centred card, ×, actions, play anything

- [x] 13.1 Centre the card in the window (drop the row anchor and `place`), and add an × at its top right that pushes `CloseCard`. Verify with headless tests: the cards of the first and last record rows have the same centre, the window's; clicking × closes the card and keeps the selection.
- [x] 13.2 Give waiting tracks a full ⏵ and show their state where the tempo goes ("queued", "40%", "search", "next" once armed); give failed and not-found tracks a ⏵ that retries; only no-clip and already-in-crate tracks get none. Verify with format tests for the states and notes, and a headless test: ⏵ on a queued track arms it and its line says "next".
- [x] 13.3 Build the record's pills in the app (+ WANT / WANTED, cart pill, » checkout, N COPIES, OWNED, SOLD) and draw them in the card's strip; a click maps to the row's action. Verify with headless tests: in a seller crate with one copy, + CART on the card sends `CartAdd` and the card stays open; + WANT sends `Want`; an owned record shows OWNED; 3 COPIES opens the record.
- [x] 13.4 Update README, the help panel and the CHANGELOG entry (centred card, ×, actions), and run the four checks.

## 14. Feedback: a modal card that fetches its record

- [x] 14.1 Make the cover 112 pt (224 at 2×). Verify with the cover-size unit test updated to 112 / 224.
- [x] 14.2 Draw a dark, half-transparent layer over the whole window under the card that takes every click, drag and hover, stop the list's and sidebar's wheel scrolling while a card is open, and drop the click-outside closing (only × and Esc close it). Verify with headless tests: with the card open, a click on another record row, on the player band and a wheel over the list change nothing and the card stays open; × and Esc still close it.
- [x] 14.3 Put the open card's record's clips in the download horizon right after the armed and playing ones (and its tracks to search in the searches), and drop them when the card steps or closes. Verify with headless tests on the dig rig: opening a card of 5 undownloaded tracks while another record plays sends a `Want` with the playing clip, then the record's 5, then the next ones; stepping sends the next record's; closing sends the plain horizon again.
- [x] 14.4 Update README, the help panel and the CHANGELOG entry, and run the four checks.

## 15. Feedback: fixed height, seek bar, follow playback, slide

- [x] 15.1 Give the tracklist a fixed 8-line height whatever the record holds. Verify by updating the height test: the cards of 3, 8 and 14 tracks are the same height.
- [x] 15.2 Draw a thin progress bar with elapsed / total time under the tracklist while a track of the card's record plays or is paused; a click or drag seeks it with `Action::Seek`. Verify with headless tests: clicking the bar's middle while A1 (2 s) plays seeks to about 1 s; with no track of the record playing, a click does nothing.
- [x] 15.3 When the playing track leaves the card's record for another record, step the card to that record (selected, scrolled to). Verify with a headless test: playing the last track of a record to its end moves the card to the next record; with the card on a record that isn't playing, a track change leaves it.
- [x] 15.4 Slide and fade the card's content in from the direction of the step over ~220 ms on every record change, restarting on each. Verify with a unit test of the easing (offset and opacity at 0, 110 and 220 ms) and a headless test: right after ›, the card's rect is unchanged and its artist text is drawn offset to the right; after 15 frames it is back in place.
- [x] 15.5 Update README, the CHANGELOG entry, and run the four checks.

## Workflow follow-up

- Archive with `/opsx:archive` once merged. That creates `openspec/specs/record-card/` and folds the playlist, album-entries and record-view deltas in.
