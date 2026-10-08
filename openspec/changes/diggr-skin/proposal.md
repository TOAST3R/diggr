## Why

The rebrand changed the name but not the look: blue metal panels, a green LCD and gold-striped title bars still read as the old player at a glance, and nothing on screen says "records". Diggr is for DJs who play vinyl first and digital second, so the default skin should feel like their gear (decks and mixers with amber displays) and show Diggr's own actions, digging and verdicts, on the player itself.

## What Changes

- **Amber LCD on warm graphite.** The default skin's panels become warm graphite and every LCD element (time digits, title line, kbps/kHz, mini visualizer, playlist text, EQ curve, the playlist footer's controls) turns amber on black. LCD text is drawn from a new skin colour instead of the green hard-coded in the app, so a skin decides it.
- **Groove title bars.** The gold double stripes either side of the caption become fine groove lines, like the edge of a record. The caption stays as DIGGR / DIGGR EQUALIZER in the skin font, now in amber; a drawn wordmark waits for the logo (`macos-release`).
- **Crate label.** The playlist title bar shows the crate's name on a small paper label, dark ink on cream, with cut corners, in place of the stripe band.
- **Idle LCD that teaches.** With the audio ready and nothing loaded, the title line scrolls "PASTE A DISCOGS LINK · CMD+V · OR DROP FILES" (CTRL+V off macOS) instead of "DIGGR - DROP FILES HERE".
- **Verdict flash.** After a verdict lands, the title line shows WANTED, UNWANTED, PASS or OWNED for 1.5 s, then the track line comes back.
- **OWNED badge in off-white.** With amber everywhere, the badge moves to off-white so it still stands out.

Not in this change: the logo, wordmark and app icon (`macos-release`), new layout or sprite positions, the fullscreen visuals, and a second "classic" skin.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `classic-skin`: "Original default skin" describes the graphite and amber palette and the groove title bars; new requirements say that LCD text comes from the skin's colours, and that the playlist title is a crate label.
- `player-window`: new requirements for the idle line and the verdict flash on the title line.
- `discogs-collection`: "Owned mark" says the badge's colour differs from the playlist text colour.

## Impact

- **`crates/ui/src/skin/generate.rs`:** palette constants, `titlebar`, the playlist title pieces, new crate-label sprites, colours; then `cargo run -p ui --bin skin-gen` rewrites `assets/skin/default/{atlas.png,skin.ron}` (the sync test keeps them honest).
- **`crates/ui/src/skin/mod.rs`:** `Colors` gains `lcd` (serde default: the old green, so older skin files keep their look).
- **`crates/ui/src/app.rs`:** the six hard-coded `[0, 236, 0]` LCD colours read `colors.lcd`; `engine_status` idle text; the title line draws a verdict flash; `draw_crate_name` draws the label.
- **`crates/ui/src/app/digging.rs`:** want, unwant, pass and collect set the flash when they change something.
- **Docs:** README's description of the player and the test count.
- No new dependency; nothing on the audio, analysis or visuals path changes.
