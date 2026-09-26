## 1. App shell

- [x] 1.1 Create `crates/ui`; turn `apps/native` into an eframe (wgpu) app owning the Engine
- [x] 1.2 Settings store (RON in platform config dir): window position/scale, sections visible, EQ, volume
- [x] 1.3 Reactive repaint policy: 30 Hz only while playing and visible; none when idle/minimized
- [x] 1.4 Measure cold launch to interactive (target < 300 ms)

## 2. Skin system

- [x] 2.1 Define `skin.ron` schema (sprite rects, layout rects, colors, bitmap font glyph map) and loader
- [x] 2.2 Atlas texture loading with nearest filtering and integer scale (1×/2×, auto on HiDPI)
- [x] 2.3 Skinned widget primitives: button (normal/pressed), toggle, horizontal slider, vertical slider, bitmap text, LCD digits
- [x] 2.4 Draw original default skin atlas (main, EQ, playlist frames, buttons, sliders, digits, font)

## 3. Main player section

- [x] 3.1 LCD time display from ClockReader with elapsed/remaining toggle
- [x] 3.2 Scrolling title, kbps/kHz, mono/stereo indicators
- [x] 3.3 Transport buttons, shuffle/repeat toggles, EQ/PL toggles
- [x] 3.4 Seek bar (seek on release), volume and balance sliders
- [x] 3.5 19-bar spectrum with peak caps and oscilloscope mode from the audio tap, aligned to the clock

## 4. Equalizer section

- [x] 4.1 ON toggle, preamp, 10 band sliders bound to the Engine; double-click reset
- [x] 4.2 Response curve preview
- [x] 4.3 Presets menu: built-ins plus user save/load/delete; persistence

## 5. Playlist section

- [x] 5.1 Playlist model, list rendering with highlight, scrolling, total/selected duration
- [x] 5.2 Add files/folders (rfd dialogs), recursive filtering, drag-and-drop from OS
- [x] 5.3 Low-priority metadata/duration worker (headers only)
- [x] 5.4 Selection (single/shift/cmd), drag reorder, remove, clear
- [x] 5.5 Shuffle/repeat ordering feeding the Engine queue and pre-warm
- [x] 5.6 Persistence and M3U/M3U8 import/export

## 6. Keyboard

- [x] 6.1 Classic shortcut map and focus rules

## 7. Fullscreen host

- [x] 7.1 `F`/Esc toggles borderless fullscreen and restores previous geometry
- [x] 7.2 `FullscreenHost` trait/API: hand wgpu device/queue/surface via paint callback each vsync; egui layer on top
- [x] 7.3 Key routing in fullscreen (transport keys kept, others to visual engine); cursor auto-hide
- [x] 7.4 Placeholder visual (clock-synced beat flash) to validate hosting until `visual-engine` lands
