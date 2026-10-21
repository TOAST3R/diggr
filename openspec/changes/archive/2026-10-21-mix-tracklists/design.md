## Context

- **`preview-search`** already turns "artist, title, length" into a checked YouTube preview, near the playhead, remembered across restarts. A track from a tracklist is exactly that.
- **yt-dlp** is already run safely (`preview-fetch`, Safe invocation). For a YouTube video or SoundCloud track, `--dump-single-json --skip-download` returns `title`, `uploader`, `duration`, `description` and `chapters` (`start_time`, `title`), without downloading media.
- **1001tracklists** shows structured rows (artist, title, label, cue time, "ID" for unknowns) but blocks automated clients. **Mixcloud** shows a "Tracklist" section on shows whose uploader filled it in. Both work only from the user's browser, as Juno does.
- **The Juno-style store layer** (`beatport-traxsource`, D1) supplies merging and Discogs linking by artist, title and catalogue number.

## Goals / Non-Goals

**Goals:**
- A mix URL or page becomes a playable crate in one action, with no manual typing.
- Lines are parsed well enough for the common formats ("01. Artist - Title (Remix) [Label]", "12:34 Artist – Title", chapters titled "Artist - Title").

**Non-Goals:**
- Downloading or playing the mix itself: Open in mix sends you to it.
- Identifying unknown tracks (audio fingerprinting).
- Comments as a source (a pinned comment's tracklist): left out, because yt-dlp would have to fetch comments and they're unbounded.

## Decisions

### D1. Two paths, one tracklist model
- **The model:** `Tracklist { source, mix_url, title, tracks: [MixTrack { time, artist, title, version, label, is_id }] }`.
- **YouTube and SoundCloud:** pasted into the app (or sent by the extension like any address), read with one yt-dlp metadata call.
  - **Chapters first:** when there are at least 3, they're used.
  - **Otherwise the description:** its lines are parsed.
- **1001tracklists and Mixcloud:** arrive as a checked tracklist send from the extension (at most 300 tracks, text ≤ 200 per field, times 0–43,200 s, no control characters). The app never fetches either site.

### D2. Parsing a line
- **Stripping:** a leading time (`h:mm:ss`, `mm:ss`, `[mm:ss]`), a track number (`01.`, `1)`, `#1`) and bullets are taken off.
- **Splitting:** on the first spaced dash (`-`, `–`, `—`) into artist and title.
- **The rest:**
  - a trailing `[Label]` becomes the label;
  - a trailing `(… Remix|Mix|Edit|Dub|Rework|Version)` becomes the version;
  - "ID", "ID - ID" or "Unknown" mark an unidentified track;
  - lines with no dash are ignored, and so are sentences with a dash in them (an artist of more than 8 words or a title of more than 15), found in a real SoundCloud description.
- **Too little:** a description yielding fewer than 3 tracks is reported as "no tracklist found" rather than guessed at.
- **Tests:** a table pins ~30 real-world line shapes.

### D3. The Mix crate
- **The crate:** "Mix: ‹title›" (cut to 40 characters), a normal crate (not locked), in mix order. Entries are titled "Artist - Title (Version)" and wait "to search".
- **IDs:** an ID entry is unavailable with the reason "unidentified in the mix".
- **Origin:** each entry keeps the mix URL and its time.
- **Sending the same mix again:** it refreshes the crate (new lines added, nothing doubled), so a 1001tracklists page updated with new IDs fills the gaps.

### D4. Open in mix
- **The menu item:** "Open in mix at 12:34" opens the rebuilt address in the default browser:
  - YouTube `https://www.youtube.com/watch?v=‹id›&t=‹s›s`;
  - SoundCloud `https://soundcloud.com/‹user›/‹slug›#t=‹m›:‹ss›`;
  - Mixcloud and 1001tracklists: the mix page.
- **Without a time:** the item reads "Open mix".

### D5. Safe invocation
- **What yt-dlp may now read:** a YouTube video id (the existing 11-character check), and a SoundCloud track address `https://soundcloud.com/‹user›/‹slug›`, both `[a-z0-9_-]{1,100}` and rebuilt by the app.
- **How:** only for metadata (`--dump-single-json --skip-download --no-playlist`), after `--`, with the user's config ignored, as now.
- **Mixcloud and 1001tracklists addresses** are never given to yt-dlp.

### D6. Found while building
- **Real yt-dlp answers** (saved as fixtures: a 37-chapter YouTube mix, a mix whose chapters are titles only, a SoundCloud mix with its tracklist in the description) showed two things: chapters without an artist ("River (… Cover)") are left out like any line without a dash, and a chatty description line with a dash in it is a sentence, not a track (more than 8 words of artist or 15 of title).
- **Discogs linking on demand:** a Discogs search per track (`track=`, artist checked, not the release title) is too many requests to run for every track of a 300-track mix, so it runs when the user asks (Find on Discogs). Searching by track often fits several pressings; then Discogs' search opens rather than a guess.
- **The extension's readers** need the sites' real markup. 1001tracklists answers automated browsers with a human check, and the Mixcloud show checked had no tracklist, so the readers wait for pages saved from a real browser (a later change). The bridge already takes the send.

## Risks / Trade-offs

- **[Free-text descriptions vary wildly]** → Chapters are preferred, and the parser is conservative (fewer than 3 tracks means "no tracklist found"). 1001tracklists, the best source, is structured.
- **[Search finds the wrong version for edits and bootlegs]** → `preview-search`'s checks apply. A wrong match can be passed or switched like any entry.
- **[Site markup changes]** → One reader per site, each with a saved-page fixture; "Couldn't read this … page" when nothing parses.

## Open Questions

- Should a Mix crate offer "Search all now" for a long mix, beyond the playhead window? Proposed: not now. Download all tracks already covers label crates, and could be extended later.
