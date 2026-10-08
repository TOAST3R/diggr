## Context

- **Playlist model:** `Playlist` (`crates/ui/src/playlist.rs`) holds entries with stable ids, a selection and the current id. `SavedPlaylist` is written to `playlist.ron` through `Store` (atomic RON writes, debounced by `playlist_dirty`) and restored at launch with `from_saved`. Entry status is Pending, Ready or Failed, and Failed rows are drawn red.
- **Queue:** the app builds the engine queue from the playlist in play order (`sync_queue`, `play_order`). `Engine::set_queue` keeps the playing track when the queue changes. A track the engine can't open is skipped seamlessly and reported as `TrackFailed`, which today marks the entry Failed.
- **Metadata:** `MetaWorker` reads tags and durations on a low-priority thread, and `set_info` overwrites the entry's title and artist.
- **Title line:** the main window's scrolling title takes artist and title from the engine's `TrackInfo` (the file's tags) when it has them.
- **Skin:** the playlist title bar is a generated sprite (`pl_top`) with a fixed "… PLAYLIST" caption baked in, plus a drag region (`pl_titlebar`). Skin text is a 5×7 uppercase pixel font; `fold` maps accented letters to plain ones. Playlist rows use egui's proportional font.

## Goals / Non-Goals

**Goals:**
- Crates as a thin layer over the existing `Playlist`: each crate is a `Playlist`, one is shown and one is playing.
- Entries ready for remote audio (origin, waiting, unavailable) without the engine knowing anything about it.
- No launch regression, no data loss on upgrade, and a safe rollback.

**Non-Goals:**
- Folders of crates, smart crates, sorting or filtering views.
- Syncing crates anywhere (cloud, Discogs lists).
- Fetching audio or talking to any network (`discogs-digging`).
- A separate crate browser window; the title-bar menu is the only crate UI.

## Decisions

### D1. A crate is a `Playlist`; the app keeps two handles
```
 Crates ─┬─ index: [(id, name, entries, created)], next_id, shown   crates/index.ron
         ├─ loaded: HashMap<CrateId, Playlist>                      crates/<id>.ron (lazy)
         ├─ shown: CrateId    what the playlist window draws and edits
         └─ playing: CrateId  what sync_queue() builds the engine queue from
```
`Playlist` keeps its selection, current entry, reordering and totals unchanged. The app's single `playlist` field becomes `crates`, with `shown()`/`shown_mut()` for the window and `playing()` for the queue, `current_index`, `TrackFailed` mapping and pre-warm. Each crate keeps its own current entry, so returning to a crate returns to where you were.

*Alternative:* one global list with a crate tag per entry. Rejected: selection, current entry and order are per list, and every list operation would have to filter.

### D2. Switching versus starting
- Showing another crate touches nothing in the engine.
- Starting a track (double-click, Enter, Play while stopped) in the shown crate makes it the playing crate and re-syncs the queue.
- Edits to the playing crate (add, remove, reorder) mark the queue dirty, as today. Edits to other crates don't.
- Deleting the playing crate stops playback, then shows the Playlist crate.

### D3. Entry model: origin, source, waiting, unavailable
```rust
pub struct Entry {
    pub id: EntryId,
    pub track: TrackRef,          // local file; empty while waiting
    pub source: Option<String>,   // where the audio comes from (a URL), for remote entries
    pub origin: Option<Origin>,
    pub title: String, pub artist: String, pub duration: Option<f64>,
    pub status: EntryStatus,
}
pub enum EntryStatus { Pending, Ready, Failed, Waiting(String), Unavailable(String) }
pub struct Origin {
    pub page: String,             // the page it was sent from
    pub release: Option<u64>, pub master: Option<u64>,
    pub label: String, pub catno: String, pub year: Option<u16>,
    pub position: String,         // side, e.g. "A1"
    pub clip: Option<String>,     // e.g. a video id
}
```
- `Waiting(text)` means the audio isn't local yet. The producer sets the text ("listed", "downloading 40%").
- `Unavailable(reason)` means the entry will never play ("no clip"). It is kept for the record.
- Producer API: `add_waiting(..)`, `set_status(id, text)`, `set_audio(id, path)` (switches to Pending, so `MetaWorker` fills in the duration) and `set_unavailable(id, reason)`.
- `set_info` on an entry with an origin updates only the duration; tags never overwrite the record. The title line uses the playlist entry's artist and title when the entry has an origin, instead of `TrackInfo`'s tags.
- `SavedEntry` gains `source`, `origin` and the status, all with `#[serde(default)]`, so an old `playlist.ron` still loads. Later changes add origin fields the same way (for example the for-sale snapshot in `discogs-digging`), with no migration.

*Alternative:* a remote variant of `TrackRef`. Rejected: `TrackRef` is the engine's type in `platform`, which must not change, and the engine never needs to know where audio came from.

### D4. The engine queue only contains playable entries
- `play_order` runs over every entry that can eventually play (Pending, Ready and Waiting), so a waiting entry keeps its place in a shuffled order when its audio arrives. `sync_queue` builds the engine queue from that order, keeping only the playable entries (Pending and Ready). Waiting entries never reach the engine, so `TrackFailed` keeps meaning "this file is broken".
- The same order tells audio producers what comes next (`discogs-digging` downloads the next few entries in it).
- M3U export writes an entry with a `source` as that URL even after its audio has arrived, so a crate never exports downloaded audio.
- When a waiting entry's audio arrives (`set_audio`) in the playing crate, the queue is re-synced. `Engine::set_queue` keeps the current track, and the next pre-warm targets the new next track. Arrivals happen at most a few times a minute, and `set_queue` is cheap.
- **Arming:** `armed: Option<(CrateId, EntryId)>`. While armed, the main window's message line says "Waiting for ‹title›", followed by the entry's status. When the audio arrives, the entry is played like a double-click. Starting any other track clears the arming.

### D5. Storage layout and lazy loading
- `crates/index.ron` holds `next_id`, the shown crate id, and each crate's id, name, entry count and creation time. The menu lists crates in creation order, with Playlist first.
- `crates/<id>.ron` holds the crate's name and its `SavedPlaylist`.
- **Launch:** read the index and the shown crate only. Other crates load synchronously when first shown, played or sent to. A 500-entry crate is about 100 KB of RON, which parses in a few milliseconds.
- **Saves:** each crate has its own dirty flag with the existing debounce, and writes through `Store::save` (atomic). The index is saved when the list of crates changes. Quitting saves every dirty crate.
- **Damage:**
  - An unreadable crate file stays listed in the menu as "(unreadable)", isn't loaded, produces one message, and is left on disk for the user.
  - A missing or damaged index is rebuilt by reading the name in each `crates/*.ron`.

### D6. Migration
If `crates/index.ron` doesn't exist and `playlist.ron` does, crate 1 "Playlist" is created from it (entries and current entry), and the index is written. `playlist.ron` is left untouched and no longer read. A fresh install starts with an empty Playlist crate.

### D7. Title bar and crate menu
- **Skin:**
  - The generator draws `pl_top` without a caption, with the decorative lines across the whole bar.
  - It also adds `pl_title_fill`, a 1-pixel-wide strip of the plain title gradient.
  - The app stretches that strip behind the crate name, then draws the name in the title gold with the skin font. The name is folded to uppercase, glyph-less characters are skipped, and it is cut to fit between the left decoration and the close button.
- **Click versus drag:** the title bar keeps its drag behaviour. A click that doesn't move past egui's drag threshold opens an egui popup menu under the title bar, like the MISC and OPT menus.
- **Menu:** every crate (✓ on the shown one, ▶ on the playing one), a separator, New crate…, Rename crate… and Delete crate…. Rename and Delete are disabled for Playlist.
- **Name entry:** reuses the EQ preset name dialog (`preset_dialog`), with the 1–40-character and case-insensitive uniqueness checks.
- **Delete:** a small confirmation, "Delete crate "X" (40 entries)?", with Delete and Cancel buttons.

### D8. Opening files goes to the Playlist crate
Eject and command-line files clear and fill the Playlist crate, make it both shown and playing, and play its first entry. ADD, drag-and-drop, Cmd+O and M3U import add to the shown crate.

### D9. Send to crate
The entry context menu gains "Send to crate ▸", listing the other crates and New crate…. It copies the entries' saved form with fresh ids, in order, to the end of the target, which is loaded first if needed. The duplicate key is the origin clip when there is one, otherwise the file path.

## Risks / Trade-offs

- [Two notions, shown and playing, can confuse] → The menu marks both, the title bar names the shown crate, and starting a track brings them together. Switching never disturbs playback, which is what matters while digging.
- [Lazy loading hides a damaged crate until it is shown] → The index still lists it, and the problem is reported when it is first loaded.
- [After the upgrade, `playlist.ron` goes stale] → It is kept only as a backup. The README says the Playlist crate is now the source of truth.
- [Regenerating the skin changes the committed atlas] → The skin test compares the committed skin with the generator, so the atlas is regenerated in the same commit.
- [The Discogs change also modifies the playlist] → Its entries only use the producer API from D3, so it doesn't touch the playlist spec again.

## Migration Plan

D6 runs automatically on the first launch after the upgrade; there are no manual steps. Rolling back means running the previous build, which still reads `playlist.ron` as it was at the upgrade.
