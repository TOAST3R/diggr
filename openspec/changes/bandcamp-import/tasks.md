## 1. Bandcamp addresses and reading (crates/dig)

- [x] 1.1 Add the `bandcamp` module with `parse(&str) -> Option<BandcampPage>` (Label, Album, Track) and `canonical()`. Unit-test the accepted forms, query and fragment dropped, uppercase host, look-alike hosts, bad slugs, `/merch`, and non-https addresses.
- [x] 1.2 Add `catno_and_album("[AF070] The Ooze EP")` → (`AF070`, `The Ooze EP`), with tests for no bracket, empty bracket and spaces.
- [x] 1.3 Add `YtDlp::read_args(page)` for album and track pages (`-J --ignore-config -- <url>`) and label pages (`--flat-playlist -J … <root>/music`). Parse the JSON into `BandcampTrack { track_id, url, artist, title, duration, album, catno, cover, streamable }`, keeping only results that pass the address and id checks. Test with fixture JSON from the spike.
- [x] 1.4 Add the label's display name: `label_name(title, page)`, taking the part after the last " | " of the page title the extension sends, else the subdomain. Test both.
- [x] 1.5 Extend the fake fetcher to serve Bandcamp reads from fixtures, for the UI tests.

## 2. Typed clip and Bandcamp previews

- [x] 2.1 Add `Clip::{YouTube(id), Bandcamp { track_id, url }}`, with the preview key (`id` or `bc<track_id>`) and download args (`-f mp3-128/bestaudio`, `-o …/bc<id>.%(ext)s -- <url>`). Test that the args match exactly, as the YouTube ones do.
- [x] 2.2 Let the scheduler download a key through its `Clip`: the UI sends `(key, Clip)`, and the queues keep `String` keys. Test that a Bandcamp preview downloads, is cached under `bc<id>` and is protected like any other.
- [x] 2.3 Keep limiting per source: `Limited { source, until }`, `Unlimited { source }` and `TryNow`, with separate backoff. Test that a Bandcamp 429 pauses only Bandcamp while YouTube downloads go on.

## 3. Origin and sources (crates/ui)

- [x] 3.1 Add `Origin.bandcamp: String` and `Origin.source: Source`, defaulted. Map the clip from `source` to the key. Test that an old saved crate loads as YouTube.
- [x] 3.2 Add the YT/BC badge at the end of the row and "Source: …" in the tooltip. Headless test: a YT row, a BC row and a local row.
- [x] 3.3 Add Play from Bandcamp / Play from YouTube to the entry menu when both sources exist. The choice switches, persists and gets the preview. Test that it survives a restart.
- [x] 3.4 Add Open on Bandcamp. Disable the Discogs items, Y and I on entries with no release, with the "isn't on Discogs" message. Test both.

## 4. Merging into a crate

- [x] 4.1 Add `match_key(artist, title)` normalization, with unit tests on real variants: accents, "(Original Mix)", "feat.", punctuation and case.
- [x] 4.2 Add `merge(crate, album tracks) -> MergeOutcome { added, fixed, skipped }`, with catno narrowing, insertion after the last entry of the same catno, a fixed track going back to waiting with Bandcamp as its source, and the skip-passed check. Unit-test every row of the outcome table, plus "same title, other record".
- [x] 4.3 Run Bandcamp reads as jobs in the preview worker (one at a time, 1 s apart), listed with the sends, with progress ("40 of 78 albums") and the summary line "‹album›: N added, N fixed, N skipped". Mark tracks that aren't streamable "no clip (not streamable)".
- [x] 4.4 Paste: accept Bandcamp addresses in Cmd+V, and send them to the shown crate. Headless test: pasting an album into an empty crate gives 2 BC entries that play.

## 5. Label crates with Bandcamp

- [x] 5.1 Add `CrateInfo.bandcamp: Option<String>` beside `label`, with `is_label`, `bandcamp_of`, `find_bandcamp` and `set_bandcamp`; `labels()` and locking cover both. Test that an older index loads unchanged.
- [x] 5.2 Add `label_name_key()` normalization and `name_match() -> Exact | Close | None`, with tests: Siesta/"Siesta Records Ltd", Lowtide/"Lowtide Tapes", unrelated names.
- [x] 5.3 Follow a Bandcamp label: exact → merge (set `bandcamp`, read albums); none → new "Label: ‹name›"; close → queue a Merge/Separate dialog, one at a time, remembering Separate in settings. The same check runs when a Discogs label is followed while a Bandcamp-only crate exists. Headless tests for each.
- [x] 5.4 Merge Bandcamp album and track sends into the label crate that follows that Bandcamp, whatever the mode, without changing the shown crate.
- [x] 5.5 Refresh label across sources, keeping the album addresses seen per crate so only new albums are read. The summary counts both sources. Test: a new Bandcamp-only album is "1 new record"; nothing new is "up to date".
- [x] 5.6 Download all tracks' window: one pause line per limited source. Tooltip "waiting for Bandcamp".

## 6. Bridge and extension

- [x] 6.1 The bridge accepts Bandcamp addresses through `bandcamp::parse`, plus an optional `title` (≤ 200 characters, Bandcamp only); labels follow without coming to the front and answer Added, Merged or Refreshed; albums and tracks use the mode. Tests in `crates/dig/tests/bridge.rs`: an album, a merged label, and a look-alike host refused.
- [x] 6.2 Extension manifest: add the `*://*.bandcamp.com/*` host permission (and content script match).
- [x] 6.3 `pages.js`: Bandcamp kinds and link patterns. `content.js`: the button on album and track pages (normal menu) and on label pages (Send label only), placed by the title or floating, with the "Artist - Title" suggestion; sends the page title with Bandcamp pages. `background.js`: context menu items on Bandcamp links.
- [ ] 6.4 Load the unpacked extension and check by hand: album, track, label and merch pages, plus a link in a forum.

## 7. Discogs and Bandcamp together

- [x] 7.1 A Discogs record arriving in a crate takes over its Bandcamp-only entries of the same tracks (`absorb_into_bandcamp`), keeping their Bandcamp audio and setting the YouTube clip aside. Headless test: Bandcamp album first, then the release: three entries, not five.
- [x] 7.2 `ReadBandcamp { only }`: a label listing narrowed to the albums whose address holds a needle (`bandcamp::album_matches`). Unit and scheduler tests.
- [x] 7.3 Fallback: "not found" and "clip failed" Discogs tracks are looked for on the followed or guessed Bandcamp, once per session, by catalogue number (and title when followed), fixing only those tracks; a missing guessed Bandcamp isn't asked again; Retry failed tracks and Refresh label look again. Headless tests: found on the guessed Bandcamp; a missing one asked once.

## 8. Docs and checks

- [x] 8.1 README: a Bandcamp section (pages, merging, badges, switching, Open on Bandcamp), the extension's new permission, and the test count.
- [x] 8.2 Run `cargo test --workspace`, clippy `-D warnings`, `fmt --check` and the wasm check, and `openspec validate bandcamp-import --strict`.
