## Context

- Bandcamp album, track and label pages are read through yt-dlp (`dig/src/bandcamp.rs`, `BandcampKind::{Label, Album, Track}`), and merged into crates by normalized artist-title within a catalogue number (`ui/src/app/bandcamp.rs`). An entry keeps its Bandcamp track page in `Origin.bandcamp`, and a Discogs record coming in later takes it over in place.
- Fan pages live on the bare host: `bandcamp.com/‹user›` (collection) and `bandcamp.com/‹user›/wishlist`. yt-dlp answers "Unsupported URL" for them.
- Checked on 2026-10-12 for the user `toast3r`:
  - The page holds a `data-blob` JSON with `fan_data.fan_id` (425839), `collection_data` and `wishlist_data` (`item_count` 99 and 103, `batch_size` 20, `last_token`), and the first 20 items of each in `item_cache.collection` / `item_cache.wishlist`.
  - `POST https://bandcamp.com/api/fancollection/1/collection_items` (and `…/wishlist_items`) with `{"fan_id", "older_than_token", "count"}` returns `{items, more_available, last_token, tracklists, …}` with no cookie. The other 79 and 83 items came back in one request each.
  - Each item has `item_url` (an album or track page on the artist's subdomain), `band_name`, `item_title`, `tralbum_type` (`a`/`t`), `item_type` (`album`, `track`, or `package` for a physical purchase) and a `token`. `tracklists` holds only the featured track, so albums still have to be read.
  - Bandcamp puts a JavaScript "Client Challenge" in front of its HTML pages for some clients. Plain curl got it every time, on fan and label pages alike. Python's urllib and yt-dlp got the real page. The JSON endpoint answered curl, after refusing one request once.
- The user's Discogs collection cache (`dig/src/collection.rs`) keeps, per owned release, its master, catalogue number and year. `owned()` counts any release of the same master as "another pressing", and the wantlist cleanup uses the same lookup.
- Refresh collection already has a progress window with a bar and Stop. Refresh wantlist has none.
- The Discogs client (`dig/src/discogs/client.rs`) has no database search yet. All requests go through its rate limiter (60 a minute with a token).

## Goals / Non-Goals

**Goals:**
- Import a Bandcamp fan's collection and wishlist into the collection and wantlist crates, with no doubles, and keep them up to date with Refresh.
- Put each item on the user's Discogs account when Discogs has exactly that record, never guessing.
- A report at the end that says what happened to every item.
- Digital and physical copies count as different records for OWNED.

**Non-Goals:**
- Logging in to Bandcamp, hidden items, or private collections (the endpoint only returns what the public page shows).
- Following another fan (friends' Bandcamp collections), or the "following" lists.
- Removing anything from Discogs or Bandcamp.
- Downloading purchases. Playback uses the same 128 kbps streams as any Bandcamp album.
- Matching a Bandcamp album to a Discogs master or to "a close enough" release.

## Decisions

**1. A fan page is its own kind, not a `BandcampKind`.** `bandcamp::parse` keeps refusing the bare host. A new `bandcamp::fan::parse` accepts `https://bandcamp.com/‹user›` and `https://bandcamp.com/‹user›/wishlist`, with or without `www.`, a trailing `/`, a query or a fragment. ‹user› is 1 to 64 lowercase letters, digits, `_` or `-`. The address is rebuilt from the checked parts, as for other Bandcamp pages. Bandcamp's own top-level paths (`discover`, `tag`, `search`, `about`, `help`, `login`, `signup`, `settings`, `artists`, `fans`…) are refused by a short list. Any other name whose page has no fan data is refused after the read ("not a Bandcamp fan page").
   - *Alternative:* a fourth `BandcampKind`. Rejected: every other kind has a subdomain, and `BandcampPage` assumes one.

**2. Reading over `ureq`, with yt-dlp as the fallback.** A `FanReader` in `dig` gets the page with the existing `ureq` agent and a browser `User-Agent`, parses the blob, then posts to the paging endpoint with `count: 100` until `more_available` is false, at least 1 s apart. When the page comes back as the challenge (no `data-blob`, or the title "Client Challenge"), it asks yt-dlp for the page instead (`--write-pages` into a temp folder: yt-dlp still saves the page it fetched before answering "Unsupported URL", which was checked on 2026-10-12) and goes on with the endpoint over `ureq`. Spike result (2026-10-12): `ureq` got the real fan page, wishlist page and a label page 9 times out of 9, and the endpoint 3 out of 3, so no fallback is built; a challenge page is reported as such ("Bandcamp asked for a browser check; try again later"). The challenge depends on the client's TLS fingerprint: curl and the system Python got it, the newer OpenSSL of yt-dlp's Python didn't. If Bandcamp tightens it, the yt-dlp fallback above is the way back.
   - *Alternative:* have the extension read the fan id from the page. Rejected: the extension promises to read only the address and title, and pasting the address must work too.

**3. The fan's state lives in `<config>/dig/bandcamp.ron`**, as the `fan` field of the existing `BandcampMemory` (which already keeps the label crates' read albums). It holds one imported fan: `username`, `fan_id`, `read_at`, and per list (`collection`, `wishlist`) the items by key (`sale_item_type` + `item_id`). Each item stores `item_url`, `kind: Digital | Physical`, `artist`, `title`, `catno`, the Bandcamp track pages read from it, and `discogs: Unsearched | Found(release) | NoMatch | Ambiguous(n)`. The last report is kept there too. It's a plain serde file: a missing or broken file loads empty and means "nothing imported". A refresh reads the lists again and works only on keys it hasn't seen (new items), plus dropping keys no longer listed.
   - *Alternative:* keep it on crate entries. Rejected: one item spans many entries, a Discogs takeover rewrites an entry's origin, and items with no playable track would leave no trace.

**4. Membership is computed, not stored on entries.** A wantlist crate entry is "wanted by Bandcamp" when its `Origin.bandcamp` track page is among the wishlist items' track pages. It is "wanted by Discogs" when its release is in `Memory.wanted`. Refresh wantlist removes an entry only when neither holds. The collection crate works the same way with the owned releases and the collection items. No field is added to `Origin`.

**5. Items go through the existing album path.** Each new item's `item_url` is rebuilt with `bandcamp::parse` (anything that fails the checks is reported as unreadable) and read with the same yt-dlp request as a pasted album. It is then merged into the target crate with `SendMode::Crate` semantics and the existing merge rules (skip what plays, fix what failed, add what's missing). Reads are queued at least 1 s apart, behind the playhead's own work, on the dig worker. Nothing touches the audio path.

**6. Discogs lookup: exact or nothing.** With a token, each item whose `discogs` is `Unsearched` gets one `GET /database/search?type=release&artist=…&release_title=…` request, with `catno=…` when the album title gave one (`catno_and_album`). From the results, it keeps those whose format matches the item: `File` for a Digital collection item, and anything but `File` for a Physical one and for every wishlist item (Bandcamp wishes carry no format, and the user wants the record; decided 2026-10-12). Their normalized artist and title must equal the item's, and the catalogue number must equal it when both have one. Exactly one left → `Found`; none → `NoMatch`; more → `Ambiguous`. The answer is saved, so a refresh never searches the same item twice.
   - `Found` in the collection list, and not already in the collection cache as this release → `POST` an instance to folder 1, through the existing Add to collection write (which updates the local cache).
   - `Found` in the wishlist, and not already wanted → the existing Add to wantlist write.
   - Then, as for any Discogs record, the record's tracks merge into the crate. Bandcamp entries are taken over in place ("Discogs records take over Bandcamp tracks"), so there are no doubles.
   - *Alternative:* score candidates and take the best. Rejected: a wrong pick writes to the user's real account.

**7. Digital is not "another pressing".** `Pressing` gains `digital: bool`, set when `basic_information.formats` names `File`. `owned()` and the wantlist cleanup ignore digital releases when looking by master, so a File release marks only itself. Physical pressings still mark each other, and a physical pressing still marks a File release of its master. The cache file gets a version number; an older cache is read whole once on the next sync, so the flag gets filled.
   - *Alternative:* store formats on entries and compare at render. Rejected: the cache is the only place that knows what's owned.

**8. Crates without a token.** A Bandcamp import with no crate marked `collection` (or `wantlist`) creates "Collection: ‹bandcamp user›" (or "Wantlist: ‹bandcamp user›") and marks it. When a Discogs token is saved later, the crates already marked are used and not duplicated. Their names stay as they are.

**9. One progress window, three steps.** A "Bandcamp import" window (beside the Discogs refresh window when both run) shows, in order:
   - "Reading your Bandcamp collection… ‹k› of ‹n› items" (or wishlist);
   - "Reading albums… ‹k› of ‹n›";
   - "Matching on Discogs… ‹k› of ‹n›" (with a token).

   Refresh collection runs its Discogs sync first, as today. Stop ends after the current request and keeps what's done: items read stay, unsearched items stay `Unsearched` for next time. Refresh wantlist gets the same window.

**10. The report is built from the run's events.** When a run ends or is stopped, the UI shows a "Bandcamp import" window. It lists the counts per crate (added, already there), the Discogs results (added, already owned or wanted, no match, ambiguous, skipped for no token), and two foldable lists: not added to Discogs, each item with links to its Bandcamp page and to a Discogs search for it, and couldn't read, with the reason. Copy puts a plain-text version on the clipboard. The report is saved in `bandcamp.ron` and reopens from the crate menu's Last Bandcamp import….

**11. Measured match rate.** On the user's lists (2026-10-12, by the rules of decision 6):

| | exactly one | several | none |
|---|---|---|---|
| Collection (99; 60 physical) | 40 | 20 | 39 |
| Wishlist (103; all digital items) | 33 | 20 | 50 |

Only 12 items had a catalogue number in their title. About 4 in 10 items reach Discogs; the rest stay in the crates and in the report.

## Risks / Trade-offs

- [The endpoint and the blob are undocumented] → A failed read changes no crate and no file, and the report and main window say why. Parsing is tolerant: missing fields skip the item, not the run.
- [Bandcamp's challenge may block `ureq`] → The task 1 spike decides; yt-dlp is the fallback for the one page read. The endpoint itself answered plain clients.
- [Exact matching skips many items, since Bandcamp-only releases often aren't on Discogs] → They're still in the crates, and the report lists them with a search link.
- [Time: about 200 items → about 3–4 minutes of album reads, plus 1–2 Discogs requests per item] → Progress window, Stop, and only new items on refresh.
- [Breaking: a File release no longer marks its vinyl owned] → Called out in the proposal and README.

## Migration Plan

- New file `bandcamp.ron`. The collection cache gets a version field, and an older cache is read whole once.
- No change to crate files.
- Rollback is a revert. Imported entries stay as ordinary Bandcamp entries.

## Open Questions

None. Settled: sending either fan page imports both lists; each Refresh reads only its own Bandcamp list (collection or wishlist).
