## Why

The player has exactly one playlist (`playlist.ron`). Digging needs several at once: a crate per label or artist you're going through, one for the records you keep, and a scratch list for whatever you open. Digging also needs entries that exist before their audio does. This change turns the playlist into named crates, and teaches entries to remember where they came from and to wait for their audio without looking broken. It needs no network, is useful on its own (a crate per gig), and is the foundation for `discogs-digging` and `browser-bridge`.

## What Changes

- **Crates:** any number of named playlists. The playlist window shows one crate at a time, and its title bar shows that crate's name instead of "WINAMP PLAYLIST".
- **Crate menu:** clicking the title bar opens a menu to switch crate, create one, or rename or delete one. Dragging the title bar still moves the window.
- **Playback follows its crate:** switching crates never interrupts playback. The playing track continues, and next and previous follow the crate it came from, until you start a track in another crate.
- **The Playlist crate:** your current playlist becomes a crate named "Playlist" on first launch. It stays the classic scratch list: files opened with Eject or on the command line replace its contents, as they replace the playlist today. No other crate is ever replaced implicitly. It can be cleared, but not renamed or deleted.
- **Send to crate:** the entry right-click menu copies the selected entries to another crate or a new one, without duplicates.
- **Where an entry came from:** an entry can carry its origin, meaning the record it belongs to (source page, release, label, catalog number, year, side, clip). The origin survives reordering, copying between crates, saving and restarting. An entry with an origin keeps its own artist and title instead of taking the file's tags.
- **Entries waiting for audio:** an entry can exist before its audio does.
  - It is drawn dimmed, with a short status where its duration would be.
  - It is skipped by next, previous and shuffle, and never reaches the engine.
  - Double-clicking it plays it as soon as its audio arrives, while the current track keeps playing.
  - An entry that will never have audio (for example a record without a clip) stays visible and dimmed, with its reason.
  - Only files that can't be opened or decoded are drawn in the error colour.
- **Persistence:** each crate is saved in `crates/` in the config folder. At launch only the shown crate is loaded, so 50 crates of 500 entries keep launch under 300 ms. M3U export writes an entry with a remote source as that source's URL, so the crate still opens in other players and never exports downloaded audio.

## Capabilities

### New Capabilities
- `crates`: named crates, the crate menu, the Playlist crate, playback following its crate, sending entries between crates, per-crate persistence, migration from the single playlist, and launch cost.

### Modified Capabilities
- `playlist`: entries can wait for their audio or be unavailable (display, play order, arming a waiting entry); entries keep their origin; persistence and M3U export apply per crate and cover waiting entries.

## Impact

- `crates/ui`:
  - `playlist.rs`: entry origin, audio source, the waiting and unavailable states, and a play order over playable entries only;
  - a new `crates.rs`: crate index, lazy loading, saving, migration, send-to-crate;
  - `app.rs`: shown and playing crates, the title-bar click and crate menu, the name dialog (reusing the EQ preset dialog pattern), "Send to crate" in the entry menu, the queue built from the playing crate, and armed play;
  - `files.rs`: M3U export of waiting entries.
- **Skin:** the generator draws the playlist title bar without its caption, plus a plain strip to draw the crate name on. The committed skin is regenerated.
- **Files:** `<config>/crates/index.ron` and `<config>/crates/<id>.ron`. `playlist.ron` is read once, for migration, and left untouched.
- No change to `audio`, `platform`, `analysis` or `visuals`. No new dependencies, no network.
