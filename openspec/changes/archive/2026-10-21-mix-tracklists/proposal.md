## Why

Half of digging is "what was that track in that set?". DJs find music in other DJs' mixes, on YouTube, SoundCloud, Mixcloud and in 1001tracklists, and then hunt each track down one by one. Diggr can turn any page of records into a crate of previews, but not a mix's tracklist.

## What Changes

- **A mix becomes a crate:** "Mix: ‹title›", with the tracks in mix order. Each track is found as a preview the way Diggr already finds tracks without a clip (YouTube search, checked by artist, title and length), and linked to Discogs when exactly one release fits.
- **Sources:**
  - **YouTube and SoundCloud:** paste the mix's address, or use the extension's button on the mix's page (it sends only the address). The app reads its chapters and description with yt-dlp, downloading nothing.
  - **1001tracklists and Mixcloud:** the app takes a tracklist send from the browser extension (both sit behind bot checks). The extension's readers for those pages are a **later change**: 1001tracklists refuses automated browsers, so their markup must be taken from pages saved in a real browser.
- **IDs stay visible:** an unidentified track ("ID - ID") is listed as such, unplayable, so the crate keeps the mix's shape.
- **Find on Discogs, on demand:** a mix track's menu looks it up by artist and track; one fitting record links it (wantlist, for sale, owned, cart), several or none open Discogs' search. Nothing is looked up unless asked, since a mix can hold 300 tracks.
- **Back to the moment:** each entry remembers its time in the mix. "Open in mix at 12:34" opens the mix there in the browser (YouTube `&t=`, SoundCloud `#t=`, Mixcloud and 1001tracklists at the mix page).

## Capabilities

### New Capabilities
- `mix-tracklists`: reading a mix's tracklist (YouTube and SoundCloud by yt-dlp; 1001tracklists and Mixcloud from the extension), parsing tracklist lines, the Mix crate, IDs, and Open in mix.

### Modified Capabilities
- `preview-fetch`: Safe invocation also allows reading a YouTube video's or a SoundCloud track's metadata, without downloading.
- `browser-bridge`: a send may also be a tracklist send.
- `chrome-extension`: access to www.youtube.com and soundcloud.com (address only), and a button on their mix pages.

(The deltas are rebuilt on the current main specs. The 1001tracklists and Mixcloud readers move to the later change.)

## Impact

- **`crates/dig`:**
  - `tracklist`: parsing (lines and chapters → artist, title, version, label, time), mix addresses, the tracklist send's checks;
  - a yt-dlp metadata read (`--dump-single-json --skip-download --no-playlist`) on the preview worker;
  - the bridge's tracklist send; a track search for Discogs lookups (`track=`).
- **`crates/ui`:** the Mix crate, ID entries, Open in mix, Find on Discogs, and the paste of YouTube and SoundCloud mix addresses.
- **`extensions/chrome`:** YouTube and SoundCloud mix pages recognised (`pages.js`), their permissions, and the button.
- **README:** mixes as a source, Open in mix, test areas and count.
