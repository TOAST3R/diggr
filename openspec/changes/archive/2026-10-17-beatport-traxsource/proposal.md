## Why

Most DJs buy electronic music digitally on Beatport and Traxsource, and that's where charts, new releases and label catalogues are browsed. Diggr can dig Discogs, Bandcamp and Juno pages but not those two, so a digger has to listen there one track at a time. Both stores also show what the analyzer would otherwise have to find out: a BPM and a musical key for every track, before anything is downloaded.

## What Changes

- **Beatport pages from the extension:** on track, release, chart, label, artist, genre and search pages of beatport.com, the extension's button reads the tracks the page already shows and sends them over the bridge. The app plays Beatport's 2-minute samples, rebuilt from each track's sample id, with one HTTPS request each and without yt-dlp.
- **Traxsource pages from the extension:** on track, title, label, artist, genre, chart and search pages of traxsource.com, the button does the same. Traxsource tracks are previewed by the YouTube search Diggr already does for tracks without a clip, because Traxsource's own samples can't yet be rebuilt from a page's data (see design).
- **The store's BPM and key:**
  - **BPM:** each entry shows the store's BPM until the analyzer has one, so the BPM filter works on a whole chart before anything downloads.
  - **Key:** shown in the tooltip ("Key: 8A, A minor"), and kept for later key features and DJ software export.
- **Like Juno:**
  - **Merging:** entries are merged into crates without doubles, and fix Discogs entries that are "clip failed" or "not found".
  - **Discogs links:** a track links to Discogs when exactly one release fits, so Y and I work.
  - **Prices:** the price is remembered with its date ("Beatport: $1.69 (seen 16 Oct)").
  - **Menu:** Open on Beatport, or Open on Traxsource.
  - **Badges:** BP and TX.
  - **Limits:** a store that limits or refuses requests pauses only itself.
- **Never fetched by the app:** neither store's pages. A Beatport or Traxsource address pasted into the app says to send it from the extension, as Juno does.

## Capabilities

### New Capabilities
- `beatport-intake`: what a Beatport send holds, its tracks in a crate, the store BPM and key, Beatport samples and their limiting, Discogs linking, the price note and Open on Beatport.
- `traxsource-intake`: what a Traxsource send holds, its tracks in a crate with preview search, the store BPM and key, Discogs linking, the price note and Open on Traxsource.

### Modified Capabilities
- `browser-bridge`: a send may also be a Beatport or Traxsource send, checked like a Juno send.
- `chrome-extension`: access to beatport.com and traxsource.com, and a button on their pages.
- `bandcamp-intake`: "Source of the sound" adds the BP badge (Beatport sample) and TX (a Traxsource track played from YouTube shows YT, with "found for Traxsource" in its tooltip).
- `preview-fetch`: the plain downloads allowed without yt-dlp include a Beatport sample.
- `playlist`: an entry's tempo may come from the store until the analyzer gives one.

## Impact

- **`extensions/chrome`:**
  - two content readers: Beatport's page data (`__NEXT_DATA__`), and Traxsource's track rows;
  - the button on their pages;
  - `host_permissions` for `*.beatport.com` and `*.traxsource.com`;
  - fixtures of saved pages for the manual checklist.
- **`crates/dig`:**
  - the bridge accepts and checks the two new send kinds;
  - a `stores` module shared with Juno for merging, Discogs linking and price notes;
  - a Beatport sample fetcher (`geo-samples.beatport.com`) with its own limiting;
  - `Source::Beatport`, and `beatport.ron` / `traxsource.ron` in the cache.
- **`crates/ui`:** BP badges, tooltips (key, price, source), Open on Beatport and Open on Traxsource, refusing pasted store addresses, and the store BPM on entries.
- **README:** the two stores in "From the browser", the extension's site access, the cache files, test areas and count.
- **No change** to `audio`, `analysis` or `platform`.
