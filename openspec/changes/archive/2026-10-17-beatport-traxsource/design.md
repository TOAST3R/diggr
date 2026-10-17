## Context

- **Juno is the template.** `juno-intake` already does "a store's page, read by the extension, sent as records, played from the store's samples":
  - a bounded, checked payload (1 MiB, 500 records);
  - the app rebuilds every address from ids, and never fetches a store page;
  - merging by normalized artist and title, narrowed by catalogue number;
  - one Discogs search per product when a token exists;
  - a dated stock and price note, and the samples fetched with `JunoHttp`;
  - its own `Source::Juno` with the shared limiting.
- **Beatport (Next.js):** its pages carry their data in `window.__NEXT_DATA__`, under `props.pageProps.dehydratedState.queries[].state.data`.
  - Track objects have `id`, `name`, `mix_name`, `artists`, `remixers`, `bpm`, `key { name, camelot }`, `length_ms`, `release { id, name, label, image }`, `price`, `sample_url` (`https://geo-samples.beatport.com/track/‹uuid›.LOFI.mp3`), and `sample_start_ms` / `sample_end_ms` (2 minutes).
  - Release objects add `catalog_number`. Charts, labels, artists, genres and search results list the same track objects.
- **Traxsource:** it refuses non-browser clients (HTTP 403). Its track rows show artist, title, version, label, key, BPM, length, price and release date, and each row has a track id.
  - How its player gets a preview isn't documented, and no public tool rebuilds one. It may be a URL issued per request.
- **What the player already has:**
  - **Preview search:** `preview-search` finds a YouTube upload for a track with no clip, by artist, title and length, near the playhead.
  - **Tempo:** the entry tempo comes only from analysis.
  - **Badges:** the YT, BC and JN badges, and Play from ‹source›.

## Goals / Non-Goals

**Goals:**
- A Beatport or Traxsource chart, release or label page becomes a crate in one click, playable at once and filterable by BPM before anything downloads.
- Beatport plays its own samples; Traxsource plays the best YouTube match.
- The same guarantees as Juno: bounded checked payloads, no store page fetched by the app, nothing sent anywhere else.

**Non-Goals:**
- Buying, carts or logins on either store.
- A KEY column or key filter: that's the `musical-key` change. This one keeps the store's key on the entry for it.
- Traxsource's own samples, until a stable way to rebuild them is found (D4).
- Fetching more of a listing than the page shows: as on Juno, the user pages through the store.

## Decisions

### D1. One shared store layer, three stores
- **The decision:** `dig::stores` holds a `Store` enum (Juno, Beatport, Traxsource), one record and track model (`StoreRecord`, `StoreTrack`), the per-store memory, and the checks Beatport and Traxsource share. Each store's module checks its own send and turns it into that model.
- **What is shared:** merging into crates, Discogs linking, price notes, "Open on ‹store›" and the refusal of pasted addresses (the player's `app/stores.rs`, generalized from Juno's).
- **Per store:** payload checks, address rebuilding, the sample source, and the entry's origin fields (`juno*`, `beatport*`, `traxsource*`), following the pattern the origin already uses per source (`bandcamp*`).
- **Juno's files:** `juno.ron` keeps its name and format, and the new stores get `beatport.ron` and `traxsource.ron`.
- **Why not copy Juno twice:** three copies of merge and link code would drift. The model fits: a record holds tracks; a track has artist, title, length, ids and a sample or none.

### D2. Beatport sends
- **The payload:** the extension reads `__NEXT_DATA__` from the loaded page (never fetching), walks the dehydrated queries for track objects, and sends up to 500 tracks grouped by release.
- **What each track carries:**
  - `id` (1–10 digits) and its page slug (`[a-z0-9-]{1,200}`);
  - artist names, the track name with `mix_name` ("Original Mix" omitted);
  - the remixers;
  - `bpm` (an integer from 40 to 250);
  - the key's Camelot code (`^(1[0-2]|[1-9])[AB]$`) and name;
  - `length_ms`, the sample's UUID (`[0-9a-f]{8}-…-[0-9a-f]{12}`);
  - the release id, name, label and catalogue number when the page shows them;
  - the price as shown, and the image id (a UUID, as in `geo-media.beatport.com/image_size/250x250/‹uuid›.jpg`).
- **Checks:** every field is checked in the app as `juno-intake` checks Juno. A track breaking a rule is refused alone.
- **Addresses:** the sample URL is rebuilt as `https://geo-samples.beatport.com/track/‹uuid›.LOFI.mp3`. The track page is rebuilt as `https://www.beatport.com/track/‹slug›/‹id›`.
- **Why read `__NEXT_DATA__`, not the DOM:** it's the data the page renders from, with exact BPM, Camelot key and sample ids, and it changes less often than markup.

### D3. Beatport samples
- **The fetcher:** `JunoHttp` generalized into `StoreHttp`, with one HTTPS GET per sample.
- **Rules:**
  - redirects only within `*.beatport.com` over HTTPS;
  - `audio/mpeg` only, at most 10 MB, and the body must start as an MP3;
  - the file is written as `bp.‹uuid›.mp3`.
- **Limiting:** 403, 429 or an HTML answer pauses Beatport with the 10 → 20 → 40 → 60 minute waits; network failures use the unreachable pause from `preview-fetch`; 404 is a failed try.
- **Scheduling:** download-ahead, slots and cache are as for every preview. Download all tracks skips Beatport, as it does Juno.

### D4. Traxsource previews by search, its samples left for later
- **The decision:** a Traxsource track enters waiting "to search", with a `SearchRequest` built from its artist, title with version, and length. `preview-search` then finds and checks a YouTube upload as for Discogs tracks without a clip.
- **Why:** the app can't rebuild a Traxsource sample address from data the page shows; the preview appears to be fetched by the site's player. Taking a URL from the extension would break the rule that the app never fetches an address the extension sends.
- **The first task:** inspect the real player in Chrome. If a sample address is a stable function of the track id, a later change adds `Source::Traxsource` samples like Beatport's.
- **What still works:** store BPM and key, price, Discogs linking and Open on Traxsource.
- **What changes for the user:** a preview may be "not found" where Beatport's never would be.

### D5. The store's BPM as the entry tempo until analysis
- **The rule:** an entry from Beatport or Traxsource takes the store's BPM as its tempo when it has none. Analysis replaces it when it gives one, folded into 88–176 as now.
- **Why:** this is what makes a 100-track chart filterable by BPM before any preview downloads, which is the point of digging a chart.
- **Why the analyzer wins:** the store's figure can be half or double time, and the analyzer listened. The tooltip names the source ("128 BPM, from Beatport" until analysed).
- **What doesn't change:** tags are still never used.

### D6. The key is kept, not yet used
- **Storage:** an entry gets `origin.key: Option<String>`, holding a checked Camelot code, saved with the crate.
- **Display:** the tooltip shows "Key: 8A (A minor)".
- **Later:** filtering, a column and compatibility belong to `musical-key`, which will prefer the store's key over its own estimate when both exist.

### D7. Badges, notes and links
- **Badges:** BP for a Beatport sample. A Traxsource track playing its YouTube match shows YT, and its tooltip says "Source: YouTube (found for Traxsource)".
- **Price note:** "Beatport: $1.69 (seen 16 Oct)" and "Traxsource: $2.49 (seen 16 Oct)", with "may have changed" after 30 days, as Juno does.
- **Menu:** "Open on Beatport" and "Open on Traxsource" open the rebuilt track page.
- **Discogs:** linking uses the same single search as Juno: artist, title and catalogue number when present.

## Risks / Trade-offs

- **[Beatport changes `__NEXT_DATA__`]** → The reader is one function with a saved-page fixture. When nothing can be read, the menu says "Couldn't read this Beatport page" and sends nothing, as on Juno.
- **[Beatport rotates or protects sample hosts]** → Answers are checked (MP3, size, host). A refusal pauses Beatport only. If samples stop working entirely, Beatport tracks can fall back to preview search like Traxsource's (a one-line policy switch per store).
- **[Traxsource search finds the wrong version]** → `preview-search` already checks title, artist and duration. The version ("Extended Mix", "Dub") is part of the query.
- **[Store BPM wrong or half time]** → It's shown with its source and replaced by the analyzer's figure as soon as one exists.
- **[Terms of use]** → The app requests only the sample files the store publishes for previewing, one per track near the playhead, with an honest user agent. It never scrapes pages; the user's browser shows them.

## Open Questions

- **Traxsource's own samples** (D4): settled by the first task's inspection, then a follow-up change if they're stable.

## Findings (task 1)

- **Beatport samples (1.2):** `https://geo-samples.beatport.com/track/‹uuid›.LOFI.mp3` answers `200 audio/mpeg` to a plain request with no cookies and no redirect (a CDN cache in front of Google Cloud Storage). The file is 2 ch, 44.1 kHz, 96 kbps MP3, 120.0 s, 1.44 MB.
- **Beatport and Traxsource pages:** both answer anything but a browser with a Cloudflare challenge (HTTP 403, "Just a moment..."), which confirms that only the extension can read them. Traxsource's track id is in `<input id="tpTrid">`, and key and BPM are in `div.trk-cell.key-bpm` (from the Mp3tag web-source scripts, checked against the live site in 2026). Beatport changed its track page data in September 2026, so the `__NEXT_DATA__` path must be confirmed on saved pages (1.1).
- **Traxsource previews (D4):** the player loads `https://geo-preview.traxsource.com/files/previews/‹n›/‹32 hex›.mp3`, which answers `200 audio/mpeg` without cookies (1.68 MB for a 6-minute track). The hash isn't derived from the track id, so the app can't rebuild it alone; a follow-up change could have the extension send `‹n›/‹hash›` (if the page's markup holds it) and rebuild the address as Beatport's is.
- **Readers written before fixtures:** the extension's readers are tolerant (Beatport: any object with a sample URL in `#__NEXT_DATA__` or the `self.__next_f` stream; Traxsource: each `/track/‹id›/‹slug›` link and the largest element around it linking to no other track), and are checked by the live test (6.3) instead of saved fixtures.
- **Should a Beatport chart send keep its chart order** when merged into an existing crate? Proposed: new entries go to the end in the page's order, as for Juno.
