## Why

Discogs is where DJs find records, but listening there means opening releases one at a time and playing their YouTube clips in a small embedded player. There's no way to move around a tune, no memory of what you skipped, and nothing that collects what you want to buy. This change turns a Discogs page (a label, an artist, a release, your wantlist, a list) into a crate in the player:
- clips are downloaded a few tracks ahead and analyzed before they play, so every preview opens with its waveform, sections and drops ready to navigate;
- one key keeps a record (and adds it to your Discogs wantlist), and another passes on it.

It is vinyl first, so buying goes through the Discogs marketplace.

## What Changes

- **Paste a Discogs page:** with a Discogs address on the clipboard, Cmd+V (Ctrl+V on Linux and Windows) adds that page's tracks to the shown crate.
  - Supported pages: release, master release, artist (main releases and remixes), label, a user's wantlist, and a user list. Language prefixes and addresses without the name part work too.
  - Three send modes, which `browser-bridge` will also use: Enqueue into the shown crate, Play as a new crate named after the page, or add to a named crate.
- **A crate that fills in progressively:**
  - Entries appear as soon as each page of Discogs' listing arrives.
  - They then get their clips and audio, ahead of what you're hearing.
  - Records without clips stay in the crate as "no clip", because they're still buyable.
- **Filters:** "vinyl only" and "skip what I've passed", both on by default.
- **Discogs client:**
  - Uses a personal access token (OPT ▸ Discogs…), and still works without one at Discogs' lower limit.
  - Never goes over Discogs' rate limit: 60 requests a minute with a token, 25 without.
  - Caches release data on disk and reuses it; marketplace numbers are refreshed daily.
  - Gives clear messages for a bad token, a missing page or being offline, and resumes by itself when the connection returns.
- **Previews:**
  - Each clip is downloaded as audio only, with a yt-dlp that the user installs; the app never bundles or downloads yt-dlp.
  - The playing track and the next 3 are kept ready, with 2 downloads at a time, in a preview cache limited to 2 GB.
  - Previews are only for listening: they can't be exported or saved anywhere.
- **Ready to navigate:** each preview is analyzed (sections, drops, waveform) in the background right after it downloads. It starts with its whole waveform and structure, so `[`, `]`, `Shift+]`, `L` and seeking work from its first second.
- **Verdict keys**, in the player window and in fullscreen, also offered in the entry right-click menu:
  - `Y` keeps the track: it is copied to the Keepers crate and its release is added to your Discogs wantlist. `Y` again undoes it.
  - `N` passes: the clip is remembered as passed, dimmed, and playback moves to the next track. Passed clips are left out of later sends.
  - `I` opens the record's for-sale page on Discogs in your browser.
- **Title line:** for a Discogs entry, the scrolling line goes on with its side, catalog number, year and a for-sale summary, for example `3. Nightcraft - Glasshouse (6:12) · A1 · LT-012 · 1994 · 6 for sale from €9.00`.
- **OPT ▸ Discogs…:** the token (checked, showing your username), yt-dlp's status with how to install it, the default filters, and the preview cache size.

## Capabilities

### New Capabilities
- `discogs-intake`: the Discogs account; supported pages and send modes; how each page becomes tracks; filters; progressive filling; the rate limit and caching; the for-sale snapshot; errors and offline behaviour; isolation from playback.
- `preview-fetch`: finding yt-dlp; which previews are downloaded and when; running yt-dlp safely; the preview cache and its limit; analysis ahead of playback; previews being only for listening.
- `dig-verdicts`: keep, pass and open-for-sale from keys and the entry menu; the Keepers crate; syncing keeps with the Discogs wantlist; remembering kept and passed clips.

### Modified Capabilities
- `player-window`: the title line shows a Discogs entry's side, catalog number, year and for-sale summary; keyboard shortcuts gain Y, N, I and Cmd+V (paste a Discogs page).
- `fullscreen-mode`: Y, N and I keep working in fullscreen instead of going to the visual engine.

## Impact

- **New crate `crates/dig`** (native only, left out of the web build):
  - `discogs`: API client, rate limiter, disk cache, address parsing, expansion, matching clips to tracks;
  - `preview`: the yt-dlp runner, download scheduling, cache limit;
  - `prepare`: analysis and waveform ahead of playback;
  - `memory`: kept and passed clips.
- **`crates/ui`:**
  - paste handling, the Discogs dialog, Y/N/I and the entry menu items, the Keepers crate, and the title-line details;
  - it turns dig results into waiting entries through the `crates` producer API.
- **`crates/analysis`:** a public function that builds and caches a track's waveform overview without keeping it in memory.
- **Skin:** `·`, `€`, `£`, `$` and `¥` glyphs in the pixel font; the committed skin is regenerated.
- **Files:**
  - `<cache>/discogs/` for API responses and `<cache>/previews/` for downloaded audio;
  - `<config>/dig/` for memory, send jobs and settings, with the token in a file only the user can read.
- **Dependencies:** `ureq` (HTTP with rustls) and `webbrowser` (opening pages). `serde_json` is already in the workspace. yt-dlp is an optional program the user installs.
- **Depends on** `crates` (waiting entries, origin, Send to crate). No change to `audio` or `platform`.
