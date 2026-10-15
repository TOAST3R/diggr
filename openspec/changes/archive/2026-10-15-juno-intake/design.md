## Context

Entries get their audio from YouTube (Discogs videos or a `ytsearch5:` search) or from Bandcamp (through yt-dlp), and `Source` / `Clip` in `dig::preview::clip` already tell the two apart by key (`‹11-char id›` vs `bc.‹track id›`). Pages reach the app by paste or from the extension over the bridge (`browser-bridge`), which today accepts only an *address* and lets the app read the page itself.

A spike on 14 Oct 2026, on a residential connection, yt-dlp 2026.08.19:

| Request | Answer |
|---|---|
| Any `www.juno.co.uk` page with `curl` (any user agent) | **403**, `cf-mitigated: challenge` (Cloudflare managed challenge), even `/` |
| `yt-dlp` on product, label or genre pages | page fetched, then "Unsupported URL": no Juno extractor |
| `www.juno.co.uk/MP3/SF1178909-01-01-01.mp3` | 302 to `mp3.juno.co.uk/MP3/…`, **200 audio/mpeg, no challenge**: 120.0 s, 128 kbps, 44.1 kHz stereo, ID3v2.4, 1.9 MB |
| Product page `/products/0-mika-vainio-rontgen-vinyl/1178909-01/` | schema.org microdata: tracks (`name`, `duration` `P0H5M1S`, `position`, `byArtist`), label `name` ("Sahko Finland"), `availability` `InStock`, `Offer`, `image`; text "Cat: SAHKO 00012"; one sample per track `SF1178909-01-‹side›-‹track›.mp3`; side headings "Side 1", "Side 2" |
| Genre page `/techno/` | 64 records in widgets: artist, title, label, format (12"/LP), price, and **one** sample each (the first track); no tracklist |
| Search `?q[all][]=SAHKO 00012` / `SAHKO00012` / `SAHKO 12` | hit / hit / miss; "couldn't find what you are looking for **in stock**" |
| `robots.txt` | `Crawl-delay: 5`; `Disallow: /MP3/`, `/api/`, `show_out_of_stock=` |

## Goals / Non-Goals

**Goals:**
- Send the Juno page the user is looking at into a crate, from the extension, in the modes Discogs and Bandcamp pages use.
- Play Juno samples through the existing preview pipeline: horizon, slots, cache, preparation, limiting.
- Merge Juno tracks with what a crate holds, without doubles, fixing tracks that have no preview.
- Link a Juno record to its Discogs release, so the Discogs verbs (Y, I, ★, owned) work on it.
- Show where the record can be bought and what it cost when seen.

**Non-Goals:**
- Diggr fetching any Juno page, or searching Juno, in the app or in the extension's background. This includes looking up Discogs records on Juno when YouTube fails, which is what the exploration called "B".
- Getting past Juno's bot challenge in any way: header tuning, TLS impersonation, or using yt-dlp as a page fetcher.
- Paging: sending more than the page in view.
- Pasting Juno addresses into the app. They can't be read without the browser.
- Juno Download, cart, wishlist or login.
- Out-of-stock records beyond what the page in view shows.

## Decisions

### 1. The extension reads the page; the app never fetches it
- **Why:** the challenge is Juno's choice. A browser the user is driving has passed it. A program fetching pages has not, and shouldn't pretend it has.
- **How:** a content script on `www.juno.co.uk` reads the DOM the browser already loaded, builds a structured payload and sends it over the bridge. The script makes **no** network request, not even same-origin. The only Juno traffic Diggr itself makes is sample downloads (decision 5).
- **Alternatives rejected:**
  - Having yt-dlp fetch pages with `--write-pages`. It works today only because yt-dlp happens to pass the challenge, so it would be using an unrelated tool to get around Juno's choice.
  - A Rust HTTP client with browser TLS impersonation. That is bot evasion, and would add a BoringSSL dependency.
  - The extension fetching further pages in the background. It would be automated scraping, and was rejected for paging too.

### 2. A send carries data, not an address
A Juno send's body is `{"juno": {"page": ‹address›, "records": [...]}, "mode": …, "crate": …, "skip_passed": …}`. Each record:

```
product   "1178909-01"           ^[0-9]{1,9}-[0-9]{2}$
slug      "0-mika-vainio-rontgen-vinyl"  ^[a-z0-9-]{1,200}$, rebuilt as /products/‹slug›/‹product›/
artist    "0 aka Mika Vainio"    1–200 chars, no control chars
title     "Rontgen (reissue)"    1–200
label     "Sahko Finland"        0–200
catno     "SAHKO 00012"          0–40
format    "12\"" | "LP" | …       0–20
stock     "in" | "out" | "preorder" | ""
price     "€14.78"               0–20, shown only, never parsed for money
image     "CS1178909-01A-BIG.jpg"  ^[A-Za-z0-9-]{1,40}\.jpg$, rebuilt on imagescdn.juno.co.uk
tracks    [ { side "1"|"A"…, n 1–99, title 0–200, artist 0–200, secs 0–7200, sample "02-01" } ] 0–60
```

- **Sample paths:** `sample` is the part after the product (`‹pid›-‹rel›`) in `SF‹pid›-‹rel›-‹side›-‹track›.mp3`: two `[0-9]{2}` groups, side and track. The app rebuilds the URL from it; it never takes a URL from the extension.
- **Size limits:** the body may be at most 1 MiB, with at most 500 records and 5,000 tracks. A 500-item listing with "Show tracks: All" fits with room to spare. The 16 KB limit stays for every other request.
- **Strictness:** an unknown field, an out-of-pattern value or a record with no usable track refuses that record. A send with no usable record is refused whole. The answer says how many records were refused.
- **Alternative rejected:** HTML from the extension, parsed in Rust. It would be 300 KB per page, and the parser would sit in the app where it can't be tested against live pages. The extension already has the DOM.

### 3. What the extension reads
- **Product pages:** the microdata. Track `name`, `duration` (ISO 8601) and `position`, the release `byArtist`, the label `name`, `availability` and the offer price. The catalogue number comes from the "Cat:" field. The sample of each track comes from its play button's `href`, paired by position. Side comes from the preceding "Side N" or "Disc N" heading.
- **Listing pages** (label, artist, genre, chart, search, new releases): each item's artist, title, label, format, price and every play-button sample it shows. With Juno's "Show tracks" set to All, items list their tracks. Otherwise an item usually gives one sample, and becomes a record with one track whose title is unknown (`""`). The app titles that entry "Artist - Title" from the record.
- **Reading code:** one function per page kind in `extensions/chrome/juno.js`. It works only from markup the browser already has; it adds nothing to the page and observes nothing beyond the button.

### 4. Keys and the typed clip
- **Key:** `jn.‹pid›-‹rel›-‹side›-‹track›` (`jn.1178909-01-01-01`). The dot keeps it apart from YouTube ids; the prefix keeps it apart from `bc.`.
- **Types:** `Source::Juno` (badge "JN", name "Juno") and `Clip::Juno { key }`. The URL is derived from the key, never stored.

### 5. Downloading samples: plain HTTPS, not yt-dlp
- **Request:** one `GET https://mp3.juno.co.uk/MP3/SF‹pid›-‹rel›-‹side›-‹track›.mp3` through the existing `ureq` agent, rebuilt from the key, written to `previews/jn.….mp3`. It goes straight to `mp3.juno.co.uk`; the spike saw `www` redirect there. A redirect to any host other than `mp3.juno.co.uk` or `www.juno.co.uk` fails the download.
- **Checks:** the body must be `audio/mpeg`, at most 10 MB, and must start with an ID3 tag or an MPEG frame sync. Otherwise the try fails.
- **Same pipeline:** it runs on the preview worker, uses the same 2 download slots and 120 s timeout, retries once, and ends in "clip failed" like the others.
- **robots.txt:** it disallows `/MP3/` for crawlers. Diggr is not crawling. It fetches only samples the user is about to hear (the playing, armed and next 3 entries, see `preview-fetch`), one request each, like pressing play on the page. No "download all tracks" for Juno entries: a label crate's Download all tracks skips them.
- **User agent:** an honest one, `Diggr/‹version›`, not a browser string.
- **Alternative rejected:** yt-dlp's generic extractor on the MP3 URL. It works, but adds a process spawn and the yt-dlp dependency for a single GET.

### 6. Limiting and refusals
- **Limiting:** an HTTP 429 from Juno pauses Juno downloads with the same 10 → 20 → 40 → 60 min backoff as YouTube and Bandcamp, kept apart from the other two.
- **Refusals:** a 403, or an HTML answer (a challenge), is not a failure of the track: Juno downloads pause the same way, with "Juno is refusing downloads" in the main window. If Juno ever puts samples behind the challenge, entries wait instead of all turning red.

### 7. Merging and linking to Discogs
- **Merging** reuses the Bandcamp merge (`bandcamp-intake`, Merging into a crate): same key (artist and title, normalized), narrowed by catalogue number, with the same outcomes (skip, fix, add) and the same message ("Rontgen: 1 added, 0 fixed, 1 skipped").
- **Discogs link:**
  - each new record, with a token, gets one `discogs::search::find_release` with `Wanted { artist, title, catno, physical: true }`;
  - the search runs behind other Discogs work and through the existing rate limiter and cache;
  - only `Found(id)` links. Ambiguous or no match leaves the record unlinked, as for Bandcamp collection items.
- **What a link brings:** the entries of a linked record take `release`, `master`, `year`, `formats` and owned/★ state, and keep their Juno audio and their Juno product. A record already linked (by product) is not searched again. Without a token, nothing is searched, and Y says a token is needed.
- **When a Discogs record arrives later:** if its tracks come into a crate holding Juno-only entries with the same catalogue number, they take the record over in place, as `bandcamp-intake` does for Bandcamp-only entries.

### 8. Juno memory and the stock note
- **Storage:** `juno.ron` in the cache folder maps product → `{ release: Option<u64>, stock, price, seen: date }`, and release → product.
- **Updates:** every send overwrites the snapshot of the records it carries. Nothing else updates it; the app can't check stock on its own.
- **Where the note shows:** on a Juno entry's tooltip, "Juno: in stock, €14.78 (seen 14 Oct)". On a Discogs entry whose release is linked, the same line, and Open on Juno in its menu. A snapshot older than 30 days reads "seen ‹date›, may have changed".
- **Not a separate verb:** `I` (Open for-sale page) stays the Discogs marketplace. Juno is Open on Juno in the menu.

## Risks / Trade-offs

- [Juno changes its markup] → All reading is in `juno.js`, one function per page kind, tested against saved pages from the spike. The app refuses malformed payloads, so a broken reader sends nothing rather than garbage. The extension shows "couldn't read this page".
- [Juno puts samples behind the challenge or blocks the user agent] → Decision 6: entries wait, the main window says so, and nothing turns red. The feature degrades to "records with a Discogs link and a stock note" until the samples come back.
- [Samples are 120 s] → Long enough for sections and drops in most tracks, but a build-up past 2 minutes is never heard. The tooltip's source line says "Juno sample (2 min)". An entry linked to Discogs with a YouTube clip can switch source (Play from YouTube).
- [Single-sample listing items] → They give one entry per record. Product pages, or "Show tracks: All", give the full tracklist. The extension's button menu says "Shows 1 track per record: set Show tracks to All for more" when that's the case.
- [Out of stock is hidden by default] → Accepted. Juno is where to buy now; Discogs remains the archive.
- [Payload trust] → The extension is ours but runs in a page Juno controls. Every field is length- and pattern-checked, and URLs are always rebuilt from ids. Text is only displayed, never interpreted. The bridge's pairing and origin rules apply unchanged.
- [Terms of use] → Diggr fetches only what the user's browser would fetch when the user presses play, with an honest user agent, and keeps nothing it doesn't play (previews are already listen-only, see `preview-fetch`). If Juno objects, the source is one feature to switch off.

## Migration Plan

- Saved crates gain `Origin.juno` (product) with a serde default, so older saves load unchanged. `juno.ron` is new.
- The extension asks for one new host permission, `https://www.juno.co.uk/*`. Chrome asks the user to approve it when the extension updates.
- Rollback: remove the Juno send from the bridge. Juno entries already in crates keep playing from their cached previews, and say "clip failed" once evicted.

## Open Questions

- ~~Do Juno's regional hosts (`es.juno.co.uk`, `de.…`) serve the same markup and product ids?~~ Same product ids (`es.juno.co.uk/products/biosphere-the-way-of-time-vinyl/1090547-01/`). The extension runs on `*.juno.co.uk` and shows its button on `www` and two-letter regional hosts; the bridge accepts those page addresses. Samples, covers and Open on Juno still use `mp3.`, `imagescdn.` and `www.`. The markup still needs checking (task 0.1).
- Does a label listing with "Show tracks: All" give per-track titles next to the samples? The spike's label pages were empty for non-browser clients, so check in the browser before task 2.
