## Why

The player needs the instantly recognizable, compact Winamp 2.x interface — main player, equalizer, and playlist stacked in one window — rendered with the same low-overhead sprite approach that made the original fast. It is also the launch point for the fullscreen visual mode.

## What Changes

- New `crates/ui` built on egui/eframe (wgpu backend), wired to the `audio-core` Engine.
- A sprite-based classic skin renderer: all widgets are drawn from an original skin atlas (not OS widgets, not Winamp's copyrighted bitmaps).
- Main player section: LCD time display (elapsed/remaining toggle), scrolling track title, kbps/kHz, mono/stereo, mini spectrum analyzer, seek bar, volume, balance, transport buttons, shuffle/repeat, EQ/PL toggles.
- Equalizer section: on/off, preamp, 10 band sliders, response curve preview, presets menu.
- Playlist section: track list with durations, current track highlight, add files/folders, drag-and-drop, remove, reorder, select, total duration, M3U load/save, persisted playlist.
- Keyboard shortcuts (classic Z/X/C/V/B, arrows, etc.) and a key (`F`) to enter/exit fullscreen visual mode.
- Fullscreen mode shell: borderless fullscreen surface handed to `visual-engine` for rendering (visual content, overlay, and fader deck are specified there).
- Reactive repainting: the UI costs ~0 CPU when nothing changes.

## Capabilities

### New Capabilities
- `classic-skin`: sprite-atlas skin rendering and the bundled original default skin.
- `player-window`: main player section (display, transport, seek, volume, balance, mini spectrum).
- `eq-window`: equalizer section UI and presets.
- `playlist`: playlist management, persistence, and M3U.
- `fullscreen-mode`: entering/leaving fullscreen and hosting the visual engine surface.

### Modified Capabilities
<!-- none -->

## Impact

- New crate `ui`; `apps/native` becomes the full GUI app.
- Dependencies: eframe/egui (wgpu), image (atlas decode), rfd (native file dialogs), serde/ron (settings, playlist persistence).
- Depends on `audio-core` (Engine, ClockReader, TapReader, TrackInfo). `visual-engine` plugs into the fullscreen surface.
