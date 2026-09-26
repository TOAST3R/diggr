## ADDED Requirements

### Requirement: Web file access
The web `FileSource` SHALL read dropped or picked files in chunks without loading whole files, and SHALL persist File System Access handles in IndexedDB so the playlist survives reloads.

#### Scenario: Reload with playlist
- **WHEN** the page is reloaded after adding 50 files via the picker
- **THEN** the playlist is restored and files become playable after granting permission once

### Requirement: IndexedDB persistence
SongScore cache, settings (EQ, volume, visual settings), playlist, and user-kept variants SHALL persist in IndexedDB.

#### Scenario: Cached score on web
- **WHEN** a previously analyzed track is played after reload
- **THEN** its score is available immediately

### Requirement: Scenes on web
Bundled scenes SHALL be compiled into the build; dropping a scene folder onto the page SHALL load or replace that scene for the session.

#### Scenario: Drop a scene folder
- **WHEN** a folder containing `scene.wgsl` and `scene.ron` is dropped
- **THEN** the scene loads (with the same validation and error toast as native)
