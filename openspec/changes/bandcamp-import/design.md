## Context

Today every entry's audio comes from YouTube. A Discogs release's video becomes the entry's clip; otherwise a `ytsearch5:` search finds one. A track that has neither ends as "not found" or "clip failed". Label crates (`label-crates`) follow one Discogs label each, keyed by its id in `CrateInfo.label: Option<u64>`.

A spike with yt-dlp 2026.08.19 (already required for previews) showed:

| Request | Answer | Time |
|---|---|---|
| `--flat-playlist -J https://analogicalforce.bandcamp.com/music` | 78 album addresses, without titles or tracks | ~3 s |
| `-J https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep` (simulates, no download) | per track: `track_id` (digits), `artist`, `track`, `duration` (403.5), `album` "[AF070] The Ooze EP", `release_date`, `thumbnail`, `webpage_url`, mp3 128 kbps | ~2.6 s |

Neither answer has the label's display name. `uploader_id` gives only the subdomain ("analogicalforce"). A plain HTTP GET of the label's page is answered with a bot challenge (403 / "Client Challenge"), so the app can't read the name from the page itself.

## Goals / Non-Goals

**Goals:**
- Send Bandcamp album, track and label pages by paste or from the extension.
- Merge them into the crates that already exist, so failed tracks get audio and nothing is doubled.
- Play Bandcamp audio through the existing preview pipeline: horizon, slots, cache, limiting, Download all tracks.
- Show and switch each entry's source.

**Non-Goals:**
- Searching Bandcamp on our own when YouTube fails. The user picks the page.
- Labels on custom domains (`music.label.com`) in the extension. They may still be pasted when they redirect to a `bandcamp.com` address; otherwise they are refused.
- Buying, logging in to Bandcamp, or downloading purchased files.
- Linking Bandcamp-only entries to Discogs releases (no wantlist or collection for them).

## Decisions

### 1. Read Bandcamp through yt-dlp, not a scraper
- **Album or track:** `-J --ignore-config -- <url>`.
- **Label:** `--flat-playlist -J --ignore-config -- <root>/music`, then one album read per album, run one after another with ≥ 1 s between them.
- **Why:** yt-dlp maintains the Bandcamp extractor, is already a dependency and runs under the same rules as for YouTube. Our own HTML parser would break on every redesign.
- **The one exception: the label's display name.** The extension already reads the page's title ("Music | Analogical Force"), so it sends that with the address. The app takes the part after the last " | ". A paste has no title, so the name falls back to the subdomain.
- **Alternative rejected:** the Bandcamp mobile API. It is undocumented, and its terms are unclear.

### 2. Strict address check, rebuilt before use
- **Accepted:** `https`, host `<sub>.bandcamp.com` with `<sub>` matching `[a-z0-9][a-z0-9-]{0,62}`, and path `/`, `/music`, `/album/<slug>` or `/track/<slug>` with `<slug>` matching `[a-z0-9-]{1,200}`.
- **Dropped:** query and fragment.
- **Rebuilt:** the canonical address is rebuilt from the parts, and only that string is passed to yt-dlp, after `--`.
- **Results:** a track's `webpage_url` from yt-dlp's answer must pass the same check, and its `track_id` must be 1–20 digits, or that track is skipped.
- **Shared code:** one function, `bandcamp::parse(&str) -> Option<BandcampPage>`, is used by paste, the bridge and the fetcher.

### 3. A typed clip
- `Clip::YouTube(id)` or `Clip::Bandcamp { track_id, url }` (`dig::preview::clip`).
- **Preview key and file name:** the YouTube id as today, or `bc.‹track_id›` for Bandcamp (`bc.3020153053.mp3`). A YouTube id never has a dot, so the two never collide; `bc` plus nine digits would be a valid YouTube id.
- **Download arguments** for Bandcamp: `-f mp3-128/bestaudio[ext=mp3]` and `-o previews/bc.‹id›.%(ext)s -- ‹url›`, with the same timeouts, retry and "clip failed" rules. The fetcher checks again that the address is a track page and is exactly the rebuilt form.
- **Scheduler and store** keep using `String` keys. The UI tells the worker each Bandcamp key's page with `PreviewCommand::Locate` before wanting it; a Bandcamp key with no page is given up on, never guessed.
- **Reading pages** happens in the same worker (`PreviewCommand::ReadBandcamp`), one page at a time and at least 1 s apart, because it runs the same yt-dlp and shares Bandcamp's limit. A label's albums are queued right after its listing, except those read before (`skip`).

### 4. Origin carries both sources
- `Origin` gets `bandcamp` (the canonical track address), `bandcamp_clip` (`bc.‹id›`) and `youtube_clip` (the YouTube clip set aside while Bandcamp is in use), all defaulting to empty, so older saves load unchanged.
- **Which clip plays:** `clip` is the clip in use; its key tells the source. Play from Bandcamp or Play from YouTube swaps `clip` with the other, and the entry waits for that preview. The other stays in the cache. A pass follows the track to its new key.
- **Bandcamp-only entries:** `release` is `None`, `catno` comes from the album title's leading bracket when there is one, `label` is the label crate's name (else the subdomain), `album` is the album title without the bracket, and `cover` is `thumbnail`.
- **Where it lives:** `ClipSource` in `playlist.rs` mirrors the key rule so the playlist doesn't depend on the native-only `dig` crate.

### 5. Matching tracks to entries
- **Key:** normalized `artist + " - " + title`:
  - lowercase;
  - strip accents and punctuation;
  - drop "(original mix)" and "feat. …";
  - collapse spaces.
- **Narrowing:** when the album title has a catalogue number and some crate entries share it (normalized: uppercase, no spaces or dashes), only those entries are candidates. Otherwise all of the crate's entries are.
- **Why the catalogue number:** Discogs often has no track lengths, so a duration key would miss. The catalogue number keeps two tracks with the same name on different releases apart.
- **Outcome:**

  | The match… | Result |
  |---|---|
  | plays (ready, waiting or downloading on YouTube) | skipped |
  | is unavailable with NoClip, ClipFailed or NotFound | gets the Bandcamp track, its `source` becomes Bandcamp, and it goes back to waiting |
  | doesn't exist | a new entry is added after the last entry of the same catalogue number, or at the end of the crate |

- **Passed tracks:** a passed track matched by key stays passed. With skip passed on, an unmatched new track is checked against dig memory by the same key.

### 6. Label crates follow Discogs, Bandcamp or both
- **Key:** `CrateInfo.label: Option<u64>` stays as it is (the Discogs id), and `CrateInfo.bandcamp: Option<String>` (the subdomain) joins it. A label crate has at least one. This needs no migration: older indexes load unchanged.
- **Matching names** (`dig::bandcamp::name_match`, shared with the bridge): a Bandcamp label sent while not followed is compared with the followed labels that have no Bandcamp yet, by its page title's name and by its subdomain:
  - normalized: folded, no punctuation and no spaces (so the subdomain "analogicalforce" equals "Analogical Force"), without the trailing words records, recordings, music, label, rec, ltd;
  - **same** → merge into that crate, setting `bandcamp`;
  - **close** (one contains the other, or edit distance ≤ 2) → a modal asks: Merge into "Label: …" or Separate;
  - **different** → a new crate "Label: ‹name›" with only `bandcamp` set.
- **Remembering:** a Separate answer is stored in `dig/bandcamp.ron` (with the albums each crate has read), so the prompt doesn't come back. The same name check runs the other way when a Discogs label is followed while a Bandcamp-only crate of the same name exists.
- **Queued sends:** the prompt doesn't block them. Several labels can be sent from the browser; each question waits its turn, shown one at a time.

### 7. Refresh and Download all tracks across sources
- **Refresh label** runs the Discogs refresh (when `discogs` is set), then the Bandcamp listing (when `bandcamp` is set). Only albums not seen at the last refresh are read; the set of album addresses is kept per crate. The summary counts new records from both.
- **Download all tracks** needs no change: it walks the crate's entries, and each entry's key is already the right clip.

### 8. Rate limiting per source
- **Detection:** yt-dlp's stderr for Bandcamp is checked by the same `limited()` matcher (HTTP 429, "Too Many Requests").
- **Separate pauses:** the `Limited { until }` state is held per source (`Source` on the event). A YouTube pause doesn't stop Bandcamp downloads.
- **The window:** it shows one pause line per paused source.

### 9. Bridge and extension
- **The request:** the field stays `url`, and the bridge accepts it when it parses as Discogs or as Bandcamp.
- **What happens:**
  - a Bandcamp label (`/` or `/music`) → follow or refresh, answering "Added label ‹name›", "Merged into Label: ‹name›" or "Refreshed label ‹name›", without coming to the front;
  - an album or track → the mode, as for a release.
- **Extension:**
  - **Host permissions:** add `*://*.bandcamp.com/*`.
  - **`pages.js`:** gets `BANDCAMP_KINDS`.
  - **On album and track pages,** `content.js` shows the normal menu (Play, Enqueue, Send to crate). On label pages it shows only "‹App›: Send label".
  - **Placement:** the button goes next to `#name-section h2` / `#band-name-location`, and floats when neither is found.
  - **Links:** the context menu adds Bandcamp link patterns.

## Risks / Trade-offs

- [Bandcamp changes its pages, and yt-dlp breaks] → reads fail with a message naming yt-dlp; updating yt-dlp fixes it. Nothing else depends on Bandcamp.
- [A label page with hundreds of albums takes minutes: 300 albums ≈ 15 min at 1 per 3 s] → it runs in the background through the same queue, with progress ("Analogical Force: 40 of 78 albums"). Refresh reads only new albums.
- [Name matching merges the wrong labels] → only exact normalized names merge silently. Close names ask, and Separate is remembered. A merged crate can be split with Delete label… and following both again.
- [Normalized title matching misses remix or edit variants and adds a near-double] → the duplicate is visible, can be passed (N), and doesn't break anything. The normalization is tested on real title variants.
- [Pre-order and non-streamable tracks have no audio] → yt-dlp lists them without a format. They are added as unavailable "no clip (not streamable)" and get audio on a later refresh.
- [Wider extension permissions] → limited to `bandcamp.com` subdomains, still reading only the address and title. The Minimal access requirement is updated to say so.
- [A pasted label page has no title, so its crate is named after the subdomain ("Label: analogicalforce")] → it can be renamed, and it merges by the space-free name match anyway.

## Migration Plan

- Saved crates: `label` accepts the old number, and `Origin` gets two defaulted fields. Rollback to an older build loses only the Bandcamp fields of entries; those entries fall back to YouTube or "no clip".
- The extension needs reloading (or a store update), and Chrome asks the user to accept the new site access.

## Open Questions

- Should a Bandcamp-only label be offered Move to Labels for normal crates filled from one Bandcamp page, as Discogs pages are? Assumed yes, using the same "all entries from one label's page" rule.
