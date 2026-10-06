## Context

- **The listing check.** `Listed.vinyl` comes from a listing's `format` string (`vinyl_from_format`), which compares each comma- or space-separated part exactly against `Vinyl`, `LP`, `7"`, `10"`, `12"` and so on. `2x12"` and `3xLP` match nothing.
- **The details check.** `Record.vinyl` comes from the details' `formats[].name` and is correct.
- **What vinyl only does today.** It drops non-vinyl items twice: at listing time (`intake.rs` `list_page` partition) and at expansion (`Outcome::Excluded`).
- **How a page fills a crate.** Each listed record becomes a "listed" placeholder entry. Its origin is built from the `Listed` item: release or master, label, catno, album = title. When the record's details arrive, `dig_record` replaces the placeholder with one entry per clip, skipping clips the crate already holds (the `have` set). So today, of two releases sharing clips, the one expanded first owns them.
- **Expansion order.** It follows the user's focus (`Command::Focus`), nearest the playhead first, over the job's `pending` list.
- **Entry origins.** These are the release or master, label, catno, album, cover, styles and (since `artist-label-filters`) the record artist. Older crates are backfilled from the disk cache by `Command::Backfill`.
- **Where the switch lives.** `vinyl_only` is in `DigSettings` (Options ▸ Discogs…), in each job's `Filters` (saved in `jobs.ron`), and in the bridge's send request and the extension's storage.

## Goals / Non-Goals

**Goals:**
- Every record of a page reaches the crate: vinyl (including multi-disc), digital, CD and the rest.
- When a record exists on vinyl in a page's crate, its entries point at the vinyl release, whatever order the releases are fetched in, with no extra request.
- Entries show and filter by format.
- The vinyl only switch disappears without breaking old settings, jobs or extensions.

**Non-Goals:**
- Pairing releases across labels or pages, or looking up a master's versions to find vinyl (no extra requests).
- Changing which release a master expands through.
- Any twin handling in the wantlist and collection crates (what you want or own is kept as is).

## Decisions

### 1. Formats as a small set of groups, parsed in one place

`model.rs` gets `Format { Vinyl, File, Cd, Cassette, Other }` and two parsers:
- **From a listing string:** split on `,`, then strip a leading count matching `^\d+x` from each part, so `3x12"` reads as `12"` and `17xFile` as `File`.
  - **Vinyl:** `Vinyl`, `LP`, `7"`, `10"`, `12"`, `Flexi-disc`, `Lathe Cut`, `Shellac`;
  - **File:** `File`, `FLAC`, `MP3`, `WAV`, `AIFF`, `ALAC`;
  - **CD:** `CD`, `CDr`, `SACD`;
  - **Cassette:** `Cass`, `Cassette`;
  - descriptors (`Album`, `Comp`, `Ltd`…) are skipped;
  - when nothing matched but the string isn't empty, the record is **Other**.
- **From a `formats` array:** each `name` is mapped the same way. `Box Set` and `All Media` are containers and skipped, since their parts are listed separately.

`Listed` and `Record` carry `formats: Vec<Format>` (deduplicated, in the order above) instead of `vinyl`. `Listed` is saved in `jobs.ron`, so it keeps reading an old `vinyl: Option<bool>` (`serde(default)`) and maps `Some(true)` to `[Vinyl]` when `formats` is empty. `is_vinyl()` = formats contain `Vinyl`.

*Alternative:* keep the raw Discogs strings. Rejected: the mark, the filter and the twin rule all need the same few groups, and the raw strings mix descriptors with formats.

### 2. Entries carry formats as text, like styles

`Origin.formats: String` holds the group names joined (`"Vinyl"`, `"Vinyl, CD"`, `"File"`), skipped when empty. It's filled in `origin()` from the record (or the listing for placeholders), and backfilled with the album, styles and artist (`records_to_backfill` also asks for entries without formats). It feeds:
- **Mark:** a row whose record has formats but none of them Vinyl draws a dim mark from its first group (`FILE`, `CD`, `CASS`, `OTHER`) where OWNED goes, on single-line rows and record rows. Entries without known formats (local files, not yet backfilled) draw none.
- **Column:** `Field::Format` shows the text and sorts by it (Vinyl first, then File, CD, Cassette, Other, unknown last). It's added to `Field::ALL` after Album and shown by default; saved column settings without it get it at its default width.
- **Filter:** `Facet::Format`, with `Entry::values(Format)` splitting the text, like styles.

### 3. Vinyl first: one rule in `dig_record`, plus queue order

When a record's details arrive for crate `c`, and `c` is not the wantlist or collection crate:

- **Twin key:** the record's master when it has one, else its normalised catalog number + title (lower-cased, trimmed). An entry's key is computed the same way from its origin.
- **Non-vinyl record arriving:** if `c` holds an entry or placeholder of a vinyl release with the same key (formats known to contain Vinyl), the record adds nothing and its placeholder leaves. The progress still counts it as done.
- **Vinyl record arriving:** for each of its clips already held in `c` by an entry of a non-vinyl release with the same key, that entry is **handed over**. Its origin becomes the vinyl's (position from the vinyl's clip), keeping its id, audio and place, so a playing track carries on. The clip counts as present, so no duplicate is added. Other entries of those non-vinyl releases, the tracks the vinyl lacks, then leave the crate. Their placeholders, if any, leave too.
- **Neither:** today's behaviour, clips deduplicated by `have`.

**Queue order:** when `list_page` adds items to a job's pending list, and when the focus reorders it, a non-vinyl item whose catalog number + title matches a pending vinyl item is placed right after that vinyl item. The twins then usually resolve vinyl-first without a hand-over and without fetching the twin's clips twice. (Masters aren't known at listing time; the hand-over covers the rest.)

*Why both:* ordering alone can't cover twins with different catalog numbers, or a focus jump that fetches the digital one first. The hand-over alone would work, but it makes the digital entries appear and then change. Neither costs a request.

*Bonus tracks:* tracks only on the digital twin leave with it, as the user chose ("the vinyl release gets the tunes"). If that proves wrong, keeping them is a one-line change: skip the "other entries leave" step.

### 4. Removing vinyl only without breaking anything saved

- `DigSettings.vinyl_only` and `Filters.vinyl_only` are removed from the structs. The serde structs don't deny unknown fields, so old settings and `jobs.ron` files load and the key is dropped on the next save.
- The bridge's send struct drops the field too. It currently denies unknown fields (`browser-bridge`: "an unknown field SHALL be refused"), so `vinyl_only` is named as an accepted-and-ignored field, which keeps older extensions working.
- The extension removes the toggle and stops sending it, and `chrome.storage` drops `vinylOnly`.
- `Outcome::Excluded` and both vinyl partitions in `intake.rs` go.

### 5. The format filter reaches dig crates

`offered_facets` currently returns nothing outside the Discogs crates. It changes to:
- **Style, Artist, Label:** in the wantlist and collection crates only, as before;
- **Format:** in any crate where an entry has an origin, when the crate has at least two formats.

The footer tiers stay: in a dig crate the only control is a FORMATS button (or nothing, when it doesn't fit). `facet_counts`, the lists, ☰ entries, `TogglePick` and `ClearPicks` already work per facet. `SavedPlaylist` gains `formats`.

## Risks / Trade-offs

- **[A master's vinyl and digital releases with different track lists]** → Only shared clips move. The digital-only ones leave with the twin, which is the chosen rule and is documented above.
- **[Releases without a master and with different catalog numbers aren't paired]** → Both stay, and the clip dedupe gives the tunes to the first fetched. On labels this is rare (Discogs creates masters when versions exist).
- **[A record's listing says vinyl but its details don't, or the reverse]** → The details win: placeholders use the listing's formats, and entries use the record's.
- **[Removing a twin while its entry plays]** → Hand-over keeps the entry, so playback continues. A twin entry with no vinyl counterpart that is playing is kept until it stops (it's removed when it isn't the current entry, or at the next settle), so playback never jumps.
- **[Requests]** → Twins are now fetched where they were dropped before, about 80 extra on a 629-item label with many digital twins, within the existing rate limit and cache. Queue order puts vinyl first, so the music arrives before the twins are resolved.

## Migration Plan

No migration. Old settings, `jobs.ron` and crate files load. Crates made with vinyl only on keep what they have, and sending the page again adds what was missed (cached listings, no repeat requests for known records).

## Open Questions

None blocking. Whether bonus tracks of a digital twin should stay is noted in Decision 3.
