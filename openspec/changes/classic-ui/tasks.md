## 1. App shell

- [ ] 1.1 Create `crates/ui`; turn `apps/native` into an eframe (wgpu) app owning the Engine
- [ ] 1.2 Settings store (RON in platform config dir): window position/scale, sections visible, EQ, volume
- [ ] 1.3 Reactive repaint policy: 30 Hz only while playing and visible; none when idle/minimized
- [ ] 1.4 Measure cold launch to interactive (target < 300 ms)

## 2. Skin system

- [ ] 2.1 Define `skin.ron` schema (sprite rects, layout rects, colors, bitmap font glyph map) and loader
- [ ] 2.2 Atlas texture loading with nearest filtering and integer scale (1×/2×, auto on HiDPI)
- [ ] 2.3 Skinned widget primitives: button (normal/pressed), toggle, horizontal slider, vertical slider, bitmap text, LCD digits
- [ ] 2.4 Draw original default skin atlas (main, EQ, playlist frames, buttons, sliders, digits, font)

## 3. Main player section

- [ ] 3.1 LCD time display from ClockReader with elapsed/remaining toggle
- [ ] 3.2 Scrolling title, kbps/kHz, mono/stereo indicators
- [ ] 3.3 Transport buttons, shuffle/repeat toggles, EQ/PL toggles
- [ ] 3.4 Seek bar (seek on release), volume and balance sliders
- [ ] 3.5 19-bar spectrum with peak caps and oscilloscope mode from the audio tap, aligned to the clock

## 4. Equalizer section

- [ ] 4.1 ON toggle, preamp, 10 band sliders bound to the Engine; double-click reset
- [ ] 4.2 Response curve preview
- [ ] 4.3 Presets menu: built-ins plus user save/load/delete; persistence

## 5. Playlist section

- [ ] 5.1 Playlist model, list rendering with highlight, scrolling, total/selected duration
- [ ] 5.2 Add files/folders (rfd dialogs), recursive filtering, drag-and-drop from OS
- [ ] 5.3 Low-priority metadata/duration worker (headers only)
- [ ] 5.4 Selection (single/shift/cmd), drag reorder, remove, clear
- [ ] 5.5 Shuffle/repeat ordering feeding the Engine queue and pre-warm
- [ ] 5.6 Persistence and M3U/M3U8 import/export

## 6. Keyboard

- [ ] 6.1 Classic shortcut map and focus rules

## 7. Fullscreen host

- [ ] 7.1 `F`/Esc toggles borderless fullscreen and restores previous geometry
- [ ] 7.2 `FullscreenHost` trait/API: hand wgpu device/queue/surface via paint callback each vsync; egui layer on top
- [ ] 7.3 Key routing in fullscreen (transport keys kept, others to visual engine); cursor auto-hide
- [ ] 7.4 Placeholder visual (clock-synced beat flash) to validate hosting until `visual-engine` lands
