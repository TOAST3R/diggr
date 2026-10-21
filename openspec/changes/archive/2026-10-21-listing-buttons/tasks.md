## 1. Shared ▶

- [x] 1.1 `content.js`: a `playButton(after, onClick)` helper (shadow root, `PLAY_STYLE`, `wr-item-play`, no duplicates); `itemButtons` and `bleepButtons` use it, behaviour unchanged

## 2. Discogs and Bandcamp

- [x] 2.1 Discogs placer: ▶ after release and master title links (one per id, text links only) on every Discogs page but a release's or master's own; sends the absolute address with mode Play
- [x] 2.2 Bandcamp placer: ▶ after album and track title links on `bandcamp.com/search` and account music pages; sends the address without its query

## 3. Stores

- [x] 3.1 `juno.js`: read one record by product id (from the list code); Juno placer after `/products/` title links, sending that record with `sendStore`
- [x] 3.2 `traxsource.js`: read one track by id; Traxsource placer after `/track/` title links
- [x] 3.3 `beatport.js`: read one track by id from the page's payloads (read once per load); Beatport placer after `/track/` title links, only for tracks the data has: a Beatport search's results carry no sample, so none get a ▶ (the page button can't read search pages either)
- [x] 3.4 Bleep: checked on `/search/query` results: the same tiles, 22 ▶, one record sent per click

## 4. Docs and checks

- [x] 4.1 README: the extension section names the per-record ▶ on every store; manual checklist items per store (search page and one other listing each)
- [x] 4.2 Checked on the live sites (the extension's scripts injected into each page with a stand-in for its Chrome API): ▶ placed and one item sent per click on Discogs search and label, Bandcamp search and music page, Juno search, Traxsource search, Beatport Top 100, Bleep search; none on a Discogs release page or a Beatport search. Loading it unpacked in Chrome and running the README checklist is left to the user. CHANGELOG entry added
