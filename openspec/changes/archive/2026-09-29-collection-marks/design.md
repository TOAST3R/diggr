## Context

- The app holds a Discogs personal token (OPT ▸ Discogs…) and knows the username (`Client::identity`). All requests go through one rate-limited client, with 60 a minute with a token.
- The API lists a collection with `GET /users/{username}/collection/folders/0/releases?per_page=100&page=N`. Folder 0 is "All". Each item carries `id` (the release) and `basic_information` (`master_id`, `year`, `labels[].catno`). A private collection is readable with the owner's token.
- Entries carry an `Origin` with `release` and/or `master`. Dig marks (kept, passed) are computed at draw time from `DigMemory`, not stored in crates.
- Keep (`DigAction::Keep`) copies the entry to Keepers and, with a token, adds the release to the wantlist.

## Goals / Non-Goals

**Goals:**
- Never miss that a record in the list is already owned, in any of its pressings.
- No extra requests per entry, whatever the crate's size.

**Non-Goals:**
- Showing ownership in the browser extension.
- Editing the collection from the app (adding a bought record).
- A "skip owned" filter (it could come later, like "skip passed").

## Decisions

**1. Fetch the collection, never each release, and then only what changed.**
Asking per release would cost one request per record in every crate (312 for a big label).
- **First sync:** every page of `GET /users/{user}/collection/folders/0/releases?sort=added&sort_order=desc&per_page=100`. A 1,000-record collection is 10 requests.
- **Later syncs:** the same query, newest first, page by page, stopping at the first item whose `instance_id` is already cached. When the cached count plus the new items equals the response's `pagination.items`, nothing was removed, and the sync ends: usually after 1 request. Otherwise records were removed (sold, deleted), and a full sync follows.
- **Other pressings:** no request. `master_id` comes from release data already fetched while digging (`/releases/{id}`), and from the collection items' `basic_information`.

The fetch runs on the intake worker at the lowest priority, after any expansion that is waiting, so it never slows a dig.

**2. The cache is `collection.ron` in the cache folder.**
It holds:
- the username, the time it was fetched, and the item count and instance ids (for the incremental sync);
- the set of owned release ids;
- master id → the owned pressings (release id, catalog number, year).

It is replaced atomically when a fetch completes, and partial fetches never replace it. With a different token user, the old cache is ignored.

**3. When to refresh.**
- Only with a token, and only when there is something to mark: a crate holding Discogs entries is shown, and the cache is missing or older than 7 days. Records bought in the last few days are remembered anyway, so a week-old collection is fresh enough. Launching, or digging nothing, sends no collection request.
- From the Refresh button in OPT ▸ Discogs… (incremental too).

A failed fetch (offline, rate limited) keeps the old cache and retries on the next occasion. The dialog shows "Collection: 1,234 records, updated 2 h ago", or why it's missing (no token, not fetched yet, private or failed).

**4. Owned = the same release or the same master.**
`owned(origin)` checks, in order:
1. `origin.release` is in the release set → "this pressing";
2. otherwise `origin.master` is in the master map → "another pressing", listing the owned pressings.

A release-only origin whose master isn't known can only match exactly. Expanded releases carry their master when Discogs gives it, so this is rare.

**5. The mark.**
A small "OWNED" badge is drawn in amber (a new skin colour, `pl_owned`), at the start of the title:
- in the single-line format, right after the number;
- in columns, at the start of the Title cell.

The row's own colour doesn't change, so owned and playing, or owned and passed, still read. The tooltip gets an "Owned" line.

**7. The browser check.**
- The bridge gets `POST /v1/owned` with `{"url": …}` (paired key required; a JSON body like a send, so the address needs no escaping). It parses the address like a send and answers from the shared, read-only collection (`ArcSwap`), without waiting for the UI or Discogs. The answer is `{"owned": "this" | "another" | "no" | "checking" | "unknown"}`, with `catno` and `year` for "another".
- **Release:** is it in the set? If not, its master id comes from the disk cache of release data when the release was dug before, and is checked against the owned masters.
- **Master:** is it in the owned masters?
- **Marketplace item:** use the cached item → release mapping. "Another pressing" is only found when that release's data is cached too (it was dug before): the item lookup gives the release, not its master, and fetching the release as well would double the cost. An exact match, the usual case when buying, always works.
- **Marketplace item, when not cached:** answer `checking` and queue a one-time lookup on the intake worker (lowest priority, cached for good). The extension asks once more after 3 s.
- **Other pages:** not asked.
- **Without a token or a collection,** the answer is `unknown` and the extension shows nothing.

In the extension, the button shows a small line under it, "✓ In your collection" or "✓ Another pressing in your collection (AF014, 2018)", only when owned. It sends nothing but the page address, which it already sends.

**6. Keep confirmation.**
When keeping (not un-keeping) an entry that is owned and a token is set, a modal names the record and what's owned: "You already own this record (AF001R, 2019). Keep it anyway? It will be added to your Discogs wantlist." It has Keep anyway (Enter) and Cancel (Esc). While it's open, other keys don't act. Without a token there's no collection and no wantlist, so there's no dialog.

## Risks / Trade-offs

- [A record bought since the last refresh isn't marked] → accepted: it's recent enough to remember. The Refresh button updates the collection at once after a buying spree.
- [A large collection, for example 10,000 records, is 100 requests on the first sync] → about 2 minutes at 60 a minute, in the background, behind digs, once. Later syncs are incremental (usually 1 request), and only a removal forces a full one.
- [Another pressing isn't always the same record for everyone] → the user chose to treat it as owned. The tooltip always says which pressing is owned.
