## 1. Collection data

- [x] 1.1 `dig`: `Collection` (username, fetched time, release set, master → pressings) with `owned(release, master)`; RON cache written atomically; unit tests
- [x] 1.2 Full sync (every page, newest first, per_page 100) and incremental sync (stop at the first known instance; full sync when the total shows removals); a partial fetch never replaces the cache; tests with the fake transport: 3-page first sync, a 1-request weekly sync, a removal forcing a full sync, a failure midway
- [x] 1.3 Run it on the intake worker at the lowest priority (behind waiting expansions); a test that an expansion waiting first is served first

## 2. Refresh

- [x] 2.1 App timing: sync only with a token, when a crate with Discogs entries is shown and the cache is missing or older than 7 days, or on demand; headless tests: no request with only local files shown, none without a token, none before the first frame
- [x] 2.2 OPT ▸ Discogs…: "Collection: N records, updated … ago" (or why not), and a Refresh button

## 3. Marks and Keep

- [x] 3.1 Skin colour `pl_owned` (amber); the OWNED badge at the start of the title (single line and columns) without changing the row colour; headless test
- [x] 3.2 Tooltip line: "Owned: this pressing" / "Owned: another pressing (catno, year)"; unit test in `entry_details`
- [x] 3.3 Keep confirmation modal for owned entries (Keep anyway / Cancel, Enter / Esc, other keys blocked); not on un-keep, not without a token; headless tests for both answers

## 4. Browser check

- [x] 4.1 Share the collection read-only (`ArcSwap`) with the bridge; `GET /v1/owned?url=` (paired): release (set, then master from the cached release data), master, marketplace item (cached mapping, else "checking" and a one-time lookup on the intake worker); tests: owned, another pressing, not owned, unknown, checking then owned, no Discogs request for a release
- [x] 4.2 Extension: ask on release, master and item pages; show the note under the button; ask once more after 3 s on "checking"; follow in-page navigation

- [x] 4.3 Extension: New crate… as a field inside the menu (the suggested name as real, editable text, cursor at the end, Enter/Create sends, Esc closes) instead of Chrome's prompt, which showed it like a hint; the suggestion drops a trailing format ("– Vinyl, 12\"…")

- [x] 4.4 Say what's missing without a token: `/v1/owned` answers "no-token" and the extension shows a dimmed hint; the player says it once per session when a Discogs crate is shown; OPT ▸ Discogs… has an "Open that page" button to https://www.discogs.com/settings/developers

- [x] 4.5 A crate created by a send (Crate mode) is shown, and the playlist opens if hidden; sending to an existing crate leaves the view alone; headless test

- [x] 4.6 Collection crate: the collection is a page (`/user/‹name›/collection`, listed like a wantlist, 100 records a request); saving a token sends it into "Collection: ‹username›" (once) and shows it; no OWNED badge inside it; the extension shows its button on collection pages and suggests "Collection: ‹user›"; tests (parse, listing, the token flow, no badge)

## 5. Docs and checks

- [x] 5.1 README (collection sync, the OWNED mark, the Keep dialog, the cache file in "Where files are kept") and the test count; help panel
- [x] 5.2 Launch time with a token and an old cache stays < 300 ms; fmt, clippy, the workspace tests and the wasm check
