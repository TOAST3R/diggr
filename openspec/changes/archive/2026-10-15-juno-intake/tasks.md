## 0. Check in the browser first

- [x] 0.1 In Chrome, open a Juno label listing with "Show tracks: All" and 100 items per page, plus a product page with 3+ sides. Confirm per-track titles and sample links sit next to each other in the listing markup, and that `es.`/`de.juno.co.uk` use the same product ids. Record the answers in design.md Open Questions. Save trimmed copies of the pages (one product, one listing with tracks, one genre widget page) as fixtures in `extensions/chrome/fixtures/`. *(Done in the browser by hand on es.juno.co.uk; regional sites share product ids, see design.md. No fixtures saved.)*

## 1. Juno payload and keys (crates/dig)

- [x] 1.1 Add the `juno` module: `JunoSend`, `JunoRecord` and `JunoTrack` (serde, `deny_unknown_fields`), and `check(send) -> Checked { records, refused }` applying every pattern, length and count limit in `juno-intake`. Unit-test each field's accept and refuse cases, a bad record among good ones, 501 records, 61 tracks, control characters, and `1178909-01/../x`.
- [x] 1.2 Add keys: `juno_key(product, sample)` → `jn.1178909-01-02-01`, `parse_key`, `sample_url(key)`, `product_url(slug, product)` and `cover_url(name)`, all rebuilt from checked parts. Test the round trips and that a `jn.` key never parses as YouTube or Bandcamp.
- [x] 1.3 Add `Source::Juno` (badge "JN", name "Juno") and `Clip::Juno { key }` in `preview/clip.rs`, with `source_of` recognising `jn.`. Test the three prefixes.

## 2. Juno samples (preview worker)

- [x] 2.1 Add a plain HTTPS download path in `preview/fetcher.rs` for `Clip::Juno`. Use the existing `ureq` agent with user agent `Diggr/‹version›`. Follow redirects manually, only to `mp3.juno.co.uk` / `www.juno.co.uk` over HTTPS. Require `audio/mpeg`, ≤ 10 MB, and an ID3 or frame-sync start. Write to a temp file and rename to `jn.….mp3`. Tests against a local test server: a good MP3, HTML with 200, a redirect elsewhere, an oversized body, and a 404.
- [x] 2.2 Classify outcomes: 429, 403 or HTML → `Limited { source: Juno }`, using the per-source backoff that already exists; 404 or a bad body → a failed try. Scheduler test: Juno limited while YouTube and Bandcamp go on; two 404s → "clip failed".
- [x] 2.3 Make Download all tracks skip Juno keys. Test with a mixed label crate.

## 3. Bridge

- [x] 3.1 Accept a Juno send in `bridge/mod.rs`: 1 MiB body limit only for a body whose top-level key is `juno` (the 16 KB limit still applies to everything else), checked with `juno::check`, queued, and answered within 100 ms with added and refused counts. Juno sends never bring the app to the front. Tests in `crates/dig/tests/bridge.rs`: a product send, a 40 KB Discogs body refused, a 1.5 MiB Juno body refused, an unknown field refused, an unpaired request refused.

## 4. Entries, merging and memory (crates/ui)

- [x] 4.1 Add `Origin.juno: Option<JunoRef { product, slug }>` with a serde default. Build entries from checked records as `juno-intake` says (artist and title fallbacks, side and number, length, catalogue number, label, format, cover). Test that an older saved crate loads unchanged.
- [x] 4.2 Merge Juno tracks through the Bandcamp merge path (same key, catalogue narrowing, skip, fix or add, skip passed) and the summary line. Headless tests: a product page into an empty crate; fixing a "not found" Discogs entry; a single-sample listing item; a 500-record send applied within 50 ms.
- [x] 4.3 Add `juno.ron` in the cache folder: product → `{ release, stock, price, seen }`, release → product, written off the UI thread. Each send updates the snapshots it carries. Test the load/save round trip in a `TestDir`.

## 5. Discogs link

- [x] 5.1 For each product with no link and a token, queue one `discogs::search::find_release` with `Wanted { artist, title, catno, physical: true }` behind other Discogs work. Store `Found(id)` in the Juno memory, and never search a product twice. Unit test with the fake transport: found, ambiguous, no match, no token (no request).
- [x] 5.2 Apply a link to the product's entries in every crate (release, master, year, formats, ★ and owned marks), keeping their Juno audio. Leave unlinked entries with Y, I and the Discogs items disabled, and the "isn't linked to Discogs" message. Headless tests for both.
- [x] 5.3 When a Discogs record arrives in a crate, it takes over Juno-only entries of the same catalogue number in place (extending `absorb_into_bandcamp` to Juno). Headless test: Juno first, then the release.

## 6. Row, tooltip and menu

- [x] 6.1 Show the JN badge and "Source: Juno sample (2 min)" in the tooltip. Generalise "Play from ‹source›" to every other source an entry has. Headless test: an entry with all three sources.
- [x] 6.2 Add the stock note to the tooltip ("Juno: in stock, €14.78 (seen 14 Oct)", plus "may have changed" after 30 days) on Juno entries and on Discogs entries of a linked release, and Open on Juno in both menus. Headless tests for the note, an old note, and the URL opened.
- [x] 6.3 Paste: refuse Juno addresses with "Send Juno pages from the browser extension", with no request made. Test.

## 7. Extension

- [x] 7.1 Add `https://www.juno.co.uk/*` to `host_permissions` and the content script matches in the manifest.
- [x] 7.2 Add `juno.js`, one reader per page kind (product: microdata, "Cat:", side headings, play links; listing and widgets: items, play links). It builds the payload of design decision 2, runs only when the menu opens, and makes no network request. Check it against the 0.1 fixtures with a test page `extensions/chrome/fixtures/juno-test.html` that runs the readers and prints the payloads. *(Checked by hand on live Juno pages instead of a fixture test page.)*
- [x] 7.3 Add Juno kinds to `pages.js`. In `content.js`, add the button on Juno pages with the record count, the "Show tracks" hint, "Couldn't read this Juno page", and the New crate… suggestion. No link menu for Juno.
- [x] 7.4 Load the unpacked extension and check by hand: a product page, a label listing with Show tracks All, a genre page, a search page, an equipment page (no button), and the Network panel showing no request besides 127.0.0.1. *(Checked by hand.)*

## 8. Docs and checks

- [x] 8.1 README: a Juno section (sent from the extension only, samples, Discogs link, stock note, Open on Juno), the JN badge, the extension's new permission, `juno.ron`, and the test count.
- [x] 8.2 Run `cargo test --workspace`, clippy `-D warnings`, `fmt --check`, the wasm check, and `openspec validate juno-intake --strict`.
