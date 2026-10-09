## Why

Diggers follow labels: they dig a label's catalogue once, then come back for what it released since. Today a label sent to Diggr is just a crate like any other. It can be edited by hand, so tracks get lost. Nothing says it belongs to a label. And getting a label's new records means sending the page again into the right crate. Top Sellers already shows the shape that works for following: a pinned group, one crate per followed thing, and a manual refresh. Labels should work the same way, and following a label from the browser should be one click, even for several labels in a row.

## What Changes

- **LABELS group** in the crate sidebar, under DISCOGS between the collection crate and TOP SELLERS. It has one crate per followed label, named "Label: ‹name›", and folds like TOP SELLERS, remembered across restarts. A label crate is grouped by record by default and stays its label's crate after a rename.
- **Filled only from its label.** A label crate takes no paste, drop or Send to crate, and no Delete, drag-out or Clear. Each is refused with a message. **Pass (`N`)** works as everywhere else: the track is dimmed and remembered, so a refresh never brings it back. Y, I and the entry menu work as in any crate.
- **Refresh label** (manual) sends the label page again into its crate. Only new tracks come in, because sends already skip what the crate holds and what was passed. The main window then sums it up, for example "Label: Siesta Records: 4 new records" or "Label: Siesta Records: up to date".
- **Label crate menu:** Refresh label, Create a crate from label (a normal crate with the same entries, named after the label; the label stays followed), Rename crate…, Remove label… (asks first, deletes the crate, keeps the passes).
- **Move to Labels** in the menu of an existing normal crate whose Discogs entries all came from one label's page.
- **BREAKING (extension):** on a Discogs label page, the button's menu offers only **‹App›: Add to Labels**. It reads **‹App›: In Labels · Refresh** once the label is followed. Right-clicking a label link offers the same single item. Choosing it follows the label (or refreshes it) in the app without changing the shown crate or playback. Several labels sent before going back to the app each get their crate, all filling in parallel.
- **Bridge:** a label page sent from the browser always follows or refreshes its label, whatever the mode, as seller pages do today. The answer says whether it was added or refreshed.
- Pasting a label address in the player (Cmd+V) is unchanged: it adds the label's tracks to the crate on screen.

## Capabilities

### New Capabilities

- `label-crates`: following labels: the LABELS group, label crates and their rules, Refresh label, Create a crate from label, Remove label, Move to Labels, and label pages sent from the browser.

### Modified Capabilities

- `crates`: "Crate sidebar" gains the LABELS group between the collection crate and TOP SELLERS.
- `chrome-extension`: "Page button" offers only ‹App›: Add to Labels on a label page; "Links anywhere" offers only that item on label links.
- `browser-bridge`: "Discogs references only" says a label page sent from the browser follows or refreshes its label.

## Impact

- **`crates/ui/src/crates.rs`:** `CrateInfo.label: Option<u64>` (serde default), `label_of`, `set_label`, grouped by default; a predicate for "no hand edits" that covers the Discogs crates and label crates.
- **`crates/ui/src/app.rs`, `app/digging.rs`:** the LABELS group in the sidebar and its fold setting; the label crate menu; Refresh label, with its summary; Create a crate from label; Remove label; Move to Labels; refusals; bridge label sends.
- **`crates/dig/src/bridge/mod.rs`:** `Snapshot.labels` (followed label ids) and the label-page rule in `send`.
- **`extensions/chrome/`:** `content.js` (the label-page menu and its followed state), `background.js` (link menu items split by link kind), `pages.js` if a label-link pattern is needed.
- **Docs:** README (labels, the extension, test count), help panel.
- No new dependency; playback, analysis and visuals are untouched. Label sends use the existing intake queue, cache and rate limit.
