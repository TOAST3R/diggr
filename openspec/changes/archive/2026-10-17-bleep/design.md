## Context

- **The store layer is ready for a fourth store.** `dig::stores` already holds what Juno, Beatport and Traxsource share: the `Store` enum, `StoreRecord` / `StoreTrack`, merging by normalized artist and title narrowed by catalogue number, one Discogs search per product, the dated stock and price note (`StoreMemory`, one RON file per store), "Open on ‹store›", and the generalized `StoreHttp` sample fetcher with per-store limiting.
- **Each store checks its own send:** `juno.rs`, `beatport.rs`, `traxsource.rs` turn the extension's payload into checked `StoreRecord`s, rebuilding every address from ids. The extension reads only the page already loaded, and only when the menu opens.
- **Bleep:** bleep.com (bleep.co.uk redirects there) sells releases in several formats (vinyl, CD, files). Every page sits behind an AWS WAF JavaScript challenge (`202`, `x-amzn-waf-action: challenge`) for anything but a browser.
- **What its pages hold (task 1.1, from 2025 Wayback copies of release and label pages):**
  - **Release page** (`/release/‹id›-‹name›`): `#product-‹id›` with `dd.artist` (artist link `/artist/‹id›-‹name›`), `dd.release-title`, `dd.label` (`/label/‹id›-‹name›`), `dd.catalogue-number`, the cover `https://d1rgjmn2wmqeif.cloudfront.net/r/s/‹id›.jpg`, and `.product-formats-list` (each format's `.media-format-name` and variants with a price and an Add to Cart carrying `data-itemtype="ReleaseFormat"` and `data-itemid`).
  - **Tracks:** `ol.track-list li.track[data-id]` with `.track-number`, the title in the play link, `.track-artist`, `.track-duration` (m:ss), and per-track formats (`TrackFormat`).
  - **List pages** (label, artist, search): `li.product[data-type=ReleaseProduct][data-id]` tiles with the release link, artist, title, label, catalogue number, release date, cover (`‹id›-‹n›.jpg` sometimes), a track count and format names. **No tracklist and no prices.**
- **Previews (task 1.2):** the player calls `GET /player/resolve/‹release›-‹disc›-‹number›` on bleep.com, which answers with an MP3 address issued for that request; it then plays a window of it (`startTime`, `playableTime`). Nothing stable to rebuild.
- **For `store-carts`:** Add to Cart is `/rest/cart` with `data-action="addTo"`, an item type (`ReleaseFormat`, `TrackFormat`) and an item id.

## Goals / Non-Goals

**Goals:**
- A Bleep release, label, chart or new-release page becomes a crate in one click, played at once.
- The same guarantees as the other stores: bounded checked payloads, every address rebuilt from ids, no Bleep page fetched by the app, nothing sent anywhere else by the extension.
- Records keep each format's price and stock, for the note now and for `store-carts` next.

**Non-Goals:**
- Carts, logins or buying: that's `store-carts`.
- Fetching Bleep pages from the app, or following "more" pages the user hasn't loaded.
- A KEY column (Bleep doesn't show keys; if a page does, it is kept like Beatport's).

## Decisions

### D1. Look first, then fix the details in the spec
- **Done** (see Context): the live pages refuse non-browsers, so 2025 Wayback copies of a release and a label page were read; the spec's rules come from them. Fixtures for the manual checklist are saved from the user's browser (task 5.2).

### D2. Records with formats, like Juno; list records stand as one entry
- **The model:** a Bleep send is records (releases), each with a label, catalogue number, its formats with prices, and its tracks when the page shows them. `bleep.rs` checks it and converts to `StoreRecord`s, so merging, linking, notes and Open on ‹store› come for free. The formats become the note's text ("LP 22.99 €, Download 2.49 €") and the entries' formats; stock isn't shown by Bleep's tiles and is left unknown.
- **List records have no tracks.** Chosen with the user (over the extension fetching each release page): such a record becomes one entry, "Artist - Title" with the record's title, searched like a track. With a token, the record is looked up on Discogs as any store record is; when exactly one release fits, `dig_fill` brings that release's tracks into the entry's crate (as linked Bandcamp collection items do) and the standing entry is removed. Without a token, or with no single fit, it stays.
- **The ▶ per record** on lists sends that one record, mode Play, the way the ▶ on Discogs for-sale lists does.

### D3. Previews by search
- Bleep's preview addresses are issued per request by bleep.com itself (Context), which the app must not call. So Bleep tracks wait "to search", found by the preview search from artist, title and length near the playhead, as Traxsource's are: the YT badge, "found for Bleep". No `Source::Bleep`, no Bleep limiting, no change to `preview-fetch`.

### D4. Price per format
- The note shows each format seen with its price as written ("Bleep: LP 22.99 €, Download 7.99 € (seen 18 Oct)"). `StoreMemory` keeps it by release, so a linked Discogs entry shows it too.

### D5. The extension
- `bleep.js`, run only when the menu opens (or a record's ▶ is clicked); no button on basket, account or checkout pages; "Couldn't read this Bleep page" when nothing parses. `host_permissions` gains `*.bleep.com`. No badge of its own: Bleep audio is always found on YouTube.

## Risks / Trade-offs

- **[Bleep changes its pages]** → The reader fails closed ("Couldn't read this Bleep page") and the fixtures make the break obvious; nothing else is affected.
- **[Bot checks]** → The extension reads the page the user already loaded, so it sees what they see; the app makes no request to bleep.com pages at all.
- **[Search previews]** (D3) → slower to first sound and sometimes not found, as for Traxsource; the entry keeps its price note and Open on Bleep.
- **[List records without a token]** → they stay one entry each, searched by the record's title; the README says a token brings in their tracks.
- **[Several bridges changes in flight]** → `mix-tracklists` and `artist-follow` also modify `browser-bridge` and `chrome-extension`; the later archive rebases its deltas.
