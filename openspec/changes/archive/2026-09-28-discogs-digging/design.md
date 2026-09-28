## Context

- **Builds on `crates`:** entries can wait for their audio (`add_waiting`, `set_status`, `set_audio`, `set_unavailable`), carry an `Origin`, and be copied between crates. The play order includes waiting entries, the engine queue only playable ones, and M3U export writes an entry's `source` URL.
- **Analysis:**
  - `analysis::eval::analyze_file` analyzes a whole file offline, and `ScoreCache` stores scores by content hash.
  - `OverviewService` builds and caches waveform overviews by content hash, but keeps each requested one in memory, and the player requests one only after the analyzer's first horizon.
  - A cached score and overview make a track open complete (`track-overview`: "whole waveform in the first frame of playback").
- **Engine:** it only ever sees local files. Its ring holds 0.5 s of audio, sized for disks, so network reads must never happen on the decode path.
- **Workers:** background work runs on `platform::Spawner` threads and reports to the UI over channels with a wake callback (`MetaWorker` is the model). There is no async runtime in the workspace.
- **Skin:** the title line is drawn in the 5×7 pixel font, which has no `·` or currency glyphs. Playlist rows use egui's proportional font.
- **Networking:** the app has none today. This is the first HTTP client and the first external program run per track (ffmpeg is only used by show rendering).

## Goals / Non-Goals

**Goals:**
- From a Discogs address to hearing the first preview in seconds, with a label of hundreds of releases filling in while you listen.
- Every preview arrives prepared (waveform, sections, drops), so you can navigate across it immediately.
- Keep and pass in one key, with the Discogs wantlist as the buying list.
- Nothing on the playback path changes, and every test runs offline.

**Non-Goals:**
- Discogs search, seller shops and marketplace searches, aliases and sublabels, and year or price filters (later).
- Digital stores (Bandcamp, Beatport, Juno).
- Discogs login (OAuth); the personal token is enough for one user.
- A cover and tracklist window, and automatic needle-drop previews.
- Streaming audio from the network while it plays.

## Decisions

### D1. A native-only `crates/dig`, threads and channels
```
 crates/dig
   discogs/   transport, client (auth, rate limit), cache, url, expand, matching, model
   preview/   fetcher (trait, yt-dlp), scheduler (horizon, slots), store (size limit)
   prepare.rs analysis and overview ahead of playback
   memory.rs  kept and passed clips, wantlist changes to retry
   jobs.rs    send jobs, persisted
```
- **Threads**, all at `Priority::Low` through the `Spawner`:
  - one intake worker, which makes every Discogs request in turn, so the rate limit is simple to enforce;
  - one preview scheduler, which owns up to 2 yt-dlp child processes;
  - one prepare worker.
- **Talking to the UI:** the UI sends commands over channels, and polls the results each frame; a wake callback asks for a repaint when results arrive.
- **Web build:** `ui` depends on `dig` only for non-wasm targets, and the dig UI code sits behind the same `cfg`.

*Alternatives:*
- Putting this in `ui`: rejected, because `ui` is already large and this logic is pure and deserves its own tests.
- tokio with reqwest: rejected, because a few requests a second don't need async, and `ureq` keeps the dependency tree and build time small.

### D2. Data flow
```
 paste / bridge ─Send{page, mode, crate, filters}─▶ ui ─Job─▶ intake worker
                                                               │ listing pages ─▶ Listed(records)
                                                               │ details, focus first ─▶ Record{clips, for-sale} | NoClip
      ui ◀────────────────────── IntakeEvent ───────────────────┘
      │  "listed" placeholders ─▶ clip entries ("queued") or "no clip"
      │  horizon = armed, playing, next 3 in play order
      ├─Want(horizon)─▶ preview scheduler ─ yt-dlp ×2 ─▶ Progress(clip, %) | Done(clip, path) | Failed(clip)
      │                   ◀───────────────────────────────────────────┘
      │  set_status / set_audio / set_unavailable
      └─Prepare(paths)─▶ prepare worker (waits for the gate) ─ analyze_file → ScoreCache; overview::precompute
```

### D3. Discogs client
- **Transport trait:** `Transport::call(Request) -> Result<Response, NetError>`, where a `Response` carries the status, the rate-limit headers and the body.
  - The real transport is a `ureq::Agent` with rustls, a 10 s connect timeout and a 30 s read timeout.
  - The test transport serves recorded JSON from `crates/dig/tests/fixtures/discogs/`, keyed by path, and can inject 429s, delays and network errors.
- **Headers:**
  - `User-Agent: winamp_rust/<version> +https://github.com/toast3r/winamp_rust`;
  - `Authorization: Discogs token=<token>` when a token is set;
  - `Accept: application/vnd.discogs.v2.discogs+json`.
- **Requests** (listings use `per_page=100`):

| Need | Request |
|---|---|
| check the token, username | `GET /oauth/identity` |
| user's currency | `GET /users/{name}` → `curr_abbr` |
| release (clips, tracklist, formats, `num_for_sale`, `lowest_price`) | `GET /releases/{id}?curr_abbr=…` |
| master (its clips and tracklist, main release) | `GET /masters/{id}` |
| artist name and releases | `GET /artists/{id}`, `GET /artists/{id}/releases?sort=year&sort_order=asc` |
| label name and releases | `GET /labels/{id}`, `GET /labels/{id}/releases` |
| wantlist | `GET /users/{name}/wants` |
| list | `GET /lists/{id}` |
| keep, un-keep | `PUT` or `DELETE /users/{name}/wants/{release}` |

- **Currency:** prices are requested in the user's currency (`curr_abbr` from their profile), or in USD without a token. The currency is stored with each price.

### D4. Rate limit
- A sliding window of the last N request times (N = 60 with a token, 25 without) on an injectable clock. A request waits until the oldest of the N is more than 60 s old.
- When `X-Discogs-Ratelimit-Remaining` drops to 2 or fewer (another tool sharing the limit), the next request waits 10 s.
- A 429 waits 10, 20, 40 and then 60 s before retrying.
- A network error switches to offline mode: "Discogs offline" in the main window, the same request retried every 15 s, and all work resumes when it succeeds.

### D5. Disk cache
- **Layout:** `<cache>/discogs/{release,master,artist,label}/<id>.json` holds the raw response and the time it was fetched. Listing pages are cached for 24 h, so re-sending a page within a day costs nothing.
- **Freshness:** record data is reused indefinitely. Its `num_for_sale` and `lowest_price` count as stale after 24 h. When a stale entry's track starts playing, one request refreshes the release and every entry from it.
- **Size:** no size limit in this change. A release is 5–20 KB, and the folder can be deleted at any time.

### D6. Addresses
| Kind | Accepted paths |
|---|---|
| release | `/release/{id}`, `/release/{id}-{slug}`, legacy `/{slug}/release/{id}` |
| master | `/master/{id}`, `/master/{id}-{slug}`, legacy `/{slug}/master/{id}` |
| artist | `/artist/{id}`, `/artist/{id}-{slug}` |
| label | `/label/{id}`, `/label/{id}-{slug}` |
| wantlist | `/wantlist?user={name}`, `/user/{name}/wantlist` |
| list | `/lists/{slug}/{id}`, `/lists/{id}` |

- **General rules:**
  - The host is `discogs.com` or `www.discogs.com`, over http or https.
  - An optional two-letter language segment may come first (`/de/`, `/es/`, `/fr/`, `/ja/`…).
  - The query (except the wantlist's `user`) and the fragment are ignored.
- **Page names:** messages and Play crate names use the name from the API response: "Label: ‹name›", "Artist: ‹name›", "Release: ‹artist› – ‹title›", "Wantlist: ‹user›", "List: ‹name›". Until that arrives, a provisional name is made from the slug.

### D7. Expansion
- **Records per kind:**
  - release: one record.
  - master: `GET /masters/{id}` gives the clips and tracklist. Its `main_release` gives the label, catalog number, year and for-sale numbers (2 requests). Masters carry their own clip list, so walking every pressing (possibly hundreds of requests) isn't needed.
  - artist: listing items with role `Main` or `Remix`, oldest first. Items of type `master` expand as masters. For `Remix`, only clips whose normalized title contains the artist's normalized name are kept.
  - label: every listing item as a release. Represses repeat clips, and clip dedupe removes those.
  - wantlist and list: release and master items.
- **Vinyl only:**
  - It is decided from the listing when the listing gives the format: the format string for labels and artists, `basic_information.formats` for wantlists.
  - Markers: `Vinyl`, `LP`, `7"`, `10"`, `12"`, `Flexi-disc`, `Lathe Cut`, `Acetate`.
  - Otherwise it is decided from the record's details (`formats[].name == "Vinyl"`).
- **Placeholders:** each listed record becomes one waiting entry, "listed", whose origin holds what the listing knows (release or master id, label, catalog number, year). When its details arrive, it is replaced in place by one entry per clip, in tracklist order and marked "queued", or it turns unavailable with "no clip".
- **Focus first:** the worker keeps the records still to fetch. Whenever the UI reports a new focus (the playing entry of that crate, or else the selected entry), it takes the first record at or after the focus, wrapping around.
- **Dedupe:** by clip id, within a send and against the target crate. The UI applies it when replacing a placeholder.
- **Jobs:** `<config>/dig/jobs.ron` holds each unfinished send: its page, mode, target crate id, filters and next listing page. When its crate is shown or played, the listing continues, and the remaining "listed" placeholders are queued again. Dedupe makes resuming idempotent.

### D8. Clips and matching
- **Clip ids** come from `videos[].uri` in these forms: `youtube.com/watch?v=ID`, `m.youtube.com/watch?v=ID`, `youtu.be/ID` and `youtube.com/embed/ID`. They must match `^[A-Za-z0-9_-]{11}$`; any other clip is unusable.
- **Normalizing titles:** lowercase; fold accents; drop the release's artist names; drop noise words and bracketed noise ("official video", "hq", "hd", "vinyl rip", "original mix", "full", sizes like `12"`, and years); keep letters, digits and spaces.
- **Matching:**
  - Each clip is scored against each track by token overlap (Jaccard). The best pair at 0.5 or more wins, greedily, so one clip goes to one track.
  - When two scores are within 0.1 of each other, the tie goes to the clip whose duration is within 10 s of the track's.
- **Entries:**
  - Artist: the track's artists, or the release's, using their Discogs names without the " (2)" disambiguation, joined as Discogs joins them.
  - Title: the track's title. The side goes into the origin's position.
  - An unmatched clip keeps the release's artist and the part of its own title after " - ". A clip longer than 15 minutes (a whole side or a mix) that doesn't match is kept the same way.

### D9. Previews
- **`Fetcher` trait:** `fetch(clip, dest, progress, cancel) -> Result<(), FetchError>`, implemented by `YtDlp { program }` and, for tests, `FakeFetcher`, which copies a fixture file with scripted progress, delays and failures.
- **Invocation:**
  - `Command::new(program)` with an argument list and no shell:
    `--ignore-config --no-playlist --no-mtime --no-warnings --newline -f "bestaudio[ext=m4a]/bestaudio[acodec^=mp4a]" --progress-template "download:progress %(progress._percent_str)s" -o <previews>/<id>.%(ext)s -- https://www.youtube.com/watch?v=<id>`
  - As built: yt-dlp reads the `download:` prefix as the template's type and doesn't print it, so the template itself starts with `progress ` to tell progress lines from the rest of the output.
  - The URL is rebuilt from the validated id, never copied from Discogs.
  - `--ignore-config` means a user config can't change paths or formats.
  - AAC in M4A is what symphonia decodes, and YouTube offers it for nearly every clip.
- **Completion:** yt-dlp writes `.part` files and renames them when complete, so a half-downloaded file is never played. The app accepts the download only if yt-dlp exits with 0 and `<id>.m4a` exists and isn't empty. After 120 s, the child is killed and the download retried once.
- **Finding yt-dlp:** `<program> --version` runs at first need, then every 30 s while entries wait for it, and whenever the dialog opens. A configured path overrides the PATH.
- **Horizon:**
  - The UI computes it from the playing crate's play order, which includes waiting entries: the armed entry, the playing entry, then the next 3 that aren't unavailable. When stopped, it uses the shown crate's current entry (or its first) and the next 3.
  - It sends `Want { wanted (in priority order), protected }` whenever the horizon changes.
  - The scheduler runs up to 2 downloads, and cancels a download that leaves the horizon, deleting its partial file.
- **Paths:** an entry's `track` is `<cache>/previews/<id>.m4a`, and its `source` is `https://www.youtube.com/watch?v=<id>`.
- **Size limit:**
  - When a preview starts playing, its file's modification time is set to now.
  - After each download, if the folder is over its limit, the oldest files are deleted first, excluding the protected set. The folder is scanned each time; it holds a few hundred files at most.

### D10. Preparing previews
- **Overview:** `analysis::overview::precompute(files, dir, track)` is a new public function. It reuses the private `build` and `save_cached`, without `OverviewService`'s in-memory map.
- **Score:** `analyze_file`, then `ScoreCache::save`. This is skipped when the cache already has a full score for that content hash.
- **Gate:** every frame, the UI sets an `AtomicBool` that is true when the playing track's score covers 32 bars or more, or when nothing is playing. The worker checks it before each preview, and between the score and the overview.
- **Order:** one preview at a time, in horizon order.
- **Budget:** the target is 20 s for a 6-minute preview on an M-series Mac, and it is measured in the tasks. If the target is missed, the overview is built first, so the waveform is always there.

### D11. Title line and glyphs
- **Suffix:** `format::title_line` gains a suffix from the origin: ` · A1 · LT-012 · 1994 · 6 for sale from €9.00`. Empty parts are left out, and a count of 0 reads "none for sale".
- **Prices:** the symbol for EUR (€), GBP (£), USD ($) and JPY (¥, with no decimals); any other currency as its ISO code and a space (`CHF 12.00`).
- **Glyphs:** the skin generator adds 5×7 glyphs for `·`, `€`, `£`, `$` and `¥`, and the committed skin is regenerated.
- **Snapshot:** `Origin` gains `for_sale: Option<ForSale { count, lowest, currency, fetched_at }>` with a serde default, as planned in `crates` D3.

### D12. Verdicts and memory
- **Keys:** Y, N and I are handled with the transport keys. In fullscreen, the host keeps them. The entry menu calls the same actions.
- **Memory:** `<config>/dig/memory.ron` holds:
  - `kept: clip → { release, added_to_wantlist, at }`;
  - `passed: clip → at`;
  - `wantlist_pending: [(release, Add | Remove)]`.

  The key is the clip id, or the file path for local entries, so Y works on any entry.
- **Keepers crate:** its id lives in `<config>/dig/settings.ron`. If it has been deleted, it is recreated as "Keepers" on the next keep. If a crate named "Keepers" already exists, it is adopted.
- **Wantlist:**
  - Once per session, the intake worker reads the user's wantlist ids (100 per request), and each keep or undo updates that set. A release not already in it is added with `PUT`, recording `added_to_wantlist = true`.
  - Undo sends `DELETE` only when the app added the release.
  - Failures go to `wantlist_pending`, which is retried every 60 s and after launch, once the window is interactive.
- **I:** `webbrowser::open("https://www.discogs.com/sell/release/<id>")`, behind a small seam so tests don't open a browser.
- **Marks:** rows with a kept clip show ✓ before the title, and passed rows are dimmed. Rows use the proportional font, which has ✓.

### D13. Settings and token
- **`<config>/dig/settings.ron`:** vinyl only, skip passed, the cache size in GB (2 by default), the yt-dlp path and the Keepers crate id.
- **`<config>/dig/token`:** only the token. It is written atomically with mode 0600 on Unix; the config folder is per-user on Windows. The dialog shows only its last 4 characters. On save it is checked with `/oauth/identity`, and the username and currency are fetched.
- **Folder overrides:** everything above sits under the config `Store` folder, so `WINAMP_CONFIG_DIR` covers it, and `WINAMP_CACHE_DIR` covers `discogs/` and `previews/`.

### D14. Offline tests
- **`crates/dig` unit tests:** addresses, matching, vinyl detection, the rate limiter on a fake clock (800 requests, never more than 60 in any 60 s window), cache freshness, clip id validation, the horizon, eviction and memory.
- **`crates/dig/tests/`:** expansion end to end against the fake transport. The fixtures are:
  - a 3-page label with releases with and without clips;
  - a master;
  - an artist with Main and Remix roles;
  - a wantlist and a list;
  - injected 429s and offline periods.

  The scheduler is tested with `FakeFetcher`: the horizon, 2 slots, cancelling, retrying, and a hanging fetch for the timeout.
- **`crates/ui/tests/dig_playback.rs`:** like `large_add.rs`, an 800-release fake label expands while the engine plays, and its previews come from `FakeFetcher` and get prepared.
- **Manual checks:** `#[ignore]` tests call real Discogs (`crates/dig/tests/intake.rs`, using `WINAMP_DISCOGS_TOKEN` if set) and a real yt-dlp (`preview::fetcher`), when run by hand.

## Risks / Trade-offs

- [YouTube's terms don't allow downloading] → yt-dlp is the user's own tool and is never bundled. Downloads happen just in time for listening, into a cache with a size limit, and can't be exported. The README says this plainly.
- [yt-dlp breaks when YouTube changes something] → A failed download shows "clip failed" on its entry. After 3 failures in a row, the main window suggests running `yt-dlp -U`.
- [Bot checks when downloading a lot] → Only 4 previews ahead, 2 at a time.
- [Clips on Discogs are sometimes wrong or mislabeled] → Unmatched clips fall back to their own title, and a bad clip is one N away. A "wrong clip" action can come later.
- [Big labels take minutes to expand] → Listings come first (100 per request), details near the playhead come next, and everything is cached, so a second visit is instant.
- [The preparation budget is an estimate] → It is measured in the tasks, and the overview goes first if it's over.
- [`beatmatch-automix` also modifies the Keyboard shortcuts requirement] → Whichever change is archived second merges the two key lists.
- [The Discogs cache has no size limit] → Entries are small and the folder is safe to delete. A limit can come later.
- [The token sits in a plain file] → Only the user can read it. The OS keychain is a candidate before the app is shared.

## Verification

Measured on an Apple M2 MacBook with release builds, on 2026-09-28:

- **Preparing a preview (D10):** a 6-minute, 128 kb/s AAC preview had its full score and overview cached in **3.5 s** while another track played, with 0 underruns (target: 20 s or less). The overview doesn't need to go first. The measurement is the ignored test `preparing_a_long_preview_while_a_track_plays` in `crates/ui/tests/dig_playback.rs`, run with `DIG_PREPARE_FILE=<file>`.
- **Digging during playback:** an 800-release label expanded, and 4 previews were downloaded and prepared, with 0 underruns (`dig_playback.rs`). A prepared preview then started in **15.5 ms**, the same as a local copy of the file (budget: 30 ms). Unoptimized test builds hold the preview to the local file's time instead, because the `audio` crate isn't optimized there and a start takes about 30 ms.
- **Launch:** `--startup-time` with a saved token and an interrupted label send took **148–182 ms** over 5 runs (target: under 300 ms). No request left before the process exited after its first frame, and no `discogs/` cache was written. Digging starts on the frame after the first; the headless test `nothing_reaches_discogs_before_the_window_is_interactive` checks the same thing with the fake transport.
- **By hand, against real Discogs and YouTube (2026-09-28):** release, label and artist pages were pasted and their previews played. Master, wantlist and list pages, and `Y` / `N` / `I` against the live wantlist, are covered by the fixture and headless tests only so far. The hand test also found that an out-of-date yt-dlp gets HTTP 403 from YouTube for every clip, and that a clip that failed is never retried in that session.

## Open Questions

- Which wantlist address forms the current Discogs site uses: both known forms are accepted, and this gets checked against the live site during implementation.
