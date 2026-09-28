## Why

In a dig crate, the duration column is mostly words: "listed", "downloading 40%", "needs yt-dlp", "no clip". They crowd out the names, and they all look alike at a glance. What an entry actually is (its label, year, side, the for-sale snapshot) only shows in the main window's title line, and only for the playing track. The right-click menu can send, keep and pass, but not play, remove or open the record's page.

## What Changes

- **Status icons:** the text in the duration slot becomes small skin icons.
  - listed: a hollow dot;
  - queued: a clock;
  - downloading: a mini progress bar;
  - preparing: a spinner dot;
  - needs yt-dlp: a warning sign;
  - unavailable ("no clip", "clip failed"): a barred circle.

  Playable entries keep their duration. Decode failures keep the error colour. The reason text moves to the tooltip.
- **Structured status:** waiting and unavailable reasons become an enum, not free text. Saved crates holding the old text still load (known strings map to their kind, anything else is kept as text).
- **Hover tooltip:** hovering an entry shows everything known about it:
  - the full name, even when the row truncates it;
  - label, catalog number, side and year;
  - BPM and duration;
  - status with its reason;
  - kept or passed, and wantlist pending;
  - the for-sale snapshot and how old it is.

  A local file shows its path.
- **Fuller right-click menu:** "Play" (or "Arm" for an entry that is waiting), "Remove", "Open release on Discogs" and "Copy Discogs link" join the existing Send to crate, Render show, Keep, Pass and Open for-sale page. The menu acts on the selection when the clicked entry is part of it, otherwise on the clicked entry alone.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `playlist`: waiting and unavailable entries show an icon instead of text in the duration slot; adds requirements for the entry tooltip and the entry context menu.

## Impact

- `crates/ui/src/playlist.rs`: `EntryStatus` carries `WaitKind` / `UnavailableKind`; the saved-crate format maps old strings.
- `crates/ui/src/app.rs`: `row_look` draws icons; tooltip; context menu entries.
- `crates/ui/src/app/digging.rs`, `crates/dig`: set the status kinds instead of strings.
- `crates/ui/src/skin/generate.rs` and `assets/skin/default/`: status icon sprites.
- **Depends on `dig-titles`:** the tooltip shows BPM, and this change's "Playlist display" text builds on the name format from that change. Archive `dig-titles` first.
