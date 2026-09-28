# winamp_rust

A Winamp 2.x–inspired music player in Rust, built for **speed and zero perceived latency**,
with a fullscreen fractal visualizer that follows the rhythm and structure of the music.

Progress:

 **audio-core** ✅ → **classic-ui** ✅ → **music-analysis** ✅ → **visual-engine** ✅ → web-target

Implemented so far: the audio engine (`audio-core`), the classic Winamp-style player window
(`classic-ui`), music analysis ahead of the playhead (`music-analysis`), and the fullscreen
visual engine (`visual-engine`), which holds a steady 60 fps on an M2 MacBook. All four are
archived in `openspec/changes/archive/`. `web-target` is planned and not started yet.

Working on the code (or pointing an AI agent at it)? Start with [`AGENTS.md`](AGENTS.md).

## Launch the app

Starting from nothing on macOS, run these from the project folder:

```sh
xcode-select --install                                           # 1. once: the C linker (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # 2. once: install Rust
source "$HOME/.cargo/env"                                        # 3. put cargo on PATH in this terminal
cargo run --release -p winamp-native                             # 4. build (a few minutes the first time) and launch
```

The player window opens. Drag music files or folders onto it, or start with the bundled
test tones:

```sh
cargo run --release -p winamp-native -- crates/audio/tests/fixtures/tone.*
cargo run --release -p winamp-native -- ~/Music/album/*.flac    # your own files
```

After the first build you can also start the program directly, without `cargo`:

```sh
./target/release/winamp-native
```

Press `F` for fullscreen visuals, `Esc` to leave, and Cmd+Q to quit. Your playlist and settings
are kept for next time.

If you get `zsh: command not found: cargo`, repeat step 3. To make it permanent, run
`echo 'source "$HOME/.cargo/env"' >> ~/.zshrc`. Other platforms and details are under
[Setup](#setup), and everything the window can do is under [Try it](#try-it).

## Setup

### 1. Prerequisites

| Platform | Needs |
|---|---|
| macOS | Xcode command-line tools (the linker): `xcode-select --install` |
| Linux | a C toolchain plus ALSA headers: `sudo apt install build-essential pkg-config libasound2-dev`; runs under X11 or Wayland |
| Windows | Visual Studio Build Tools (C++); rustup offers to install them |

### 2. Rust

The project uses stable Rust, edition 2024. It needs **Rust 1.95 or newer** (egui 0.36 requires
it) and is built and tested with 1.98. On an older toolchain, `rustup update stable`.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"                     # puts cargo on PATH in this shell
rustup component add clippy rustfmt           # for the lint/format commands below
rustup target add wasm32-unknown-unknown      # only for the web-portability check
```

> **`zsh: command not found: cargo`?** Cargo is installed in `~/.cargo/bin`, but that folder
> isn't on your `PATH`. Fix it for the current shell with `source "$HOME/.cargo/env"`, or for
> every new shell with:
>
> ```sh
> echo 'source "$HOME/.cargo/env"' >> ~/.zshrc    # bash: ~/.bashrc
> ```
>
> Check with `cargo --version`. Conda environments (`(base)`) don't matter here.

### 3. Build

```sh
cargo build --release
```

The first build downloads and compiles all dependencies, which takes a few minutes; later
builds are incremental. Dependencies are compiled with optimizations even in debug builds
(see `Cargo.toml`), because unoptimized decoders can't keep up with real-time playback.

## Try it

### The player

```sh
cargo run --release -p winamp-native                          # opens with the crate you last had open
cargo run --release -p winamp-native -- ~/Music/album/*.flac  # replaces the Playlist crate and plays
```

No music handy? The repo includes short test tones:
`cargo run --release -p winamp-native -- crates/audio/tests/fixtures/tone.*`

The window is the classic three-part Winamp layout: main player, equalizer and playlist. It's
drawn from an original pixel-art skin, at double size by default (switch with **OPT** in the
playlist).

- **Add music:** drag files or folders onto the window (folders are scanned recursively; `.m3u`
  playlists are expanded), use **ADD**, or press Cmd+O. These add to the crate on screen.
  **Eject** (and files given on the command line) replace the Playlist crate and play it.
- **Crates:** the playlist window shows one of several named playlists, and its title bar shows
  that crate's name.
  - Click the title bar for the crate menu: switch crate (• marks the one shown, ⏵ the one
    playing), **New crate…**, **Rename crate…** and **Delete crate…** (which asks first when
    the crate has entries). Dragging the title bar still moves the window.
  - Switching crates never interrupts playback: next, previous, shuffle and repeat follow the
    crate the playing track came from, until you start a track in another crate.
  - **Playlist** is the scratch crate. It always exists, can be cleared but not renamed or
    deleted, and is the only crate that Eject and command-line files replace. Your playlist
    from earlier versions becomes this crate on the first launch.
  - Right-click entries for **Send to crate**: it copies the selection, in order, to another
    crate or a new one, skipping entries that crate already holds.
- **Playlist window:**
  - double-click an entry to play it;
  - Shift/Cmd-click to select several;
  - drag to reorder;
  - Delete removes the selection;
  - drag the bottom-right grip to show more rows;
  - **MISC** imports and exports M3U/M3U8 (of the crate on screen).
  - Entries can wait for their audio (dimmed, with a status such as "downloading 40%" where
    the duration goes) or be unavailable (dimmed, with the reason). Both are skipped by next,
    previous and shuffle. Double-clicking a waiting entry arms it: the current track plays on,
    the main window says it is waiting, and the entry starts as soon as its audio arrives. Only
    files that can't be opened are drawn in red.
- **Main window:**
  - click the time to switch between elapsed and remaining;
  - click the mini visualizer to cycle spectrum → oscilloscope → off;
  - **SHUFFLE**;
  - **REP** cycles off → all → one (all gapless);
  - **EQ** and **PL** show or hide the other sections.
- **Equalizer:**
  - **ON** enables it;
  - drag the sliders, or double-click one to reset it to 0 dB;
  - **PRESETS** loads the built-in presets, and can save or delete your own.
- **Move the window** by dragging any title bar. Your crates, settings and presets are saved
  in the config folder (see [Where files are kept](#where-files-are-kept)). Set
  `WINAMP_CONFIG_DIR=/some/dir` to use another folder, for example for testing.

| Key | Action | | Key | Action |
|---|---|---|---|---|
| `Z` | previous | | `←` / `→` | seek −5 s / +5 s |
| `X` | play | | `↑` / `↓` | volume |
| `C` | pause / resume | | `F` | fullscreen visuals (`F`/`Esc` to leave) |
| `V` | stop | | `Delete` / `Backspace` | remove selected entries |
| `B` | next | | `Enter` | play the (first) selected entry |
| `[` / `]` | previous / next section | | `Cmd+O` / `Cmd+A` | add files / select all |
| `Shift+]` | jump to the next drop | | `W` | show/hide the waveform |
| `L` | loop the current section | | `Shift+L` | loop 4 bars (press again: 8, 16) |
| `H` or `F1` | all shortcuts (help panel) | | `S` | spectrogram window |
| `Y` | keep the playing track (again: undo) | | `Cmd+V` | paste a Discogs page into the crate on screen |
| `N` | pass the playing track | | `I` | open the playing release's for-sale page |

On Linux and Windows, `Cmd` is `Ctrl`.

**Fullscreen (`F`)** shows the fractal visuals (see [Visuals](#visuals)). Transport keys,
including the section jumps and loops, and `Y`, `N` and `I`, keep working in fullscreen.

### Digging Discogs

Paste a Discogs address (Cmd+V, anywhere in the player) and the page's tracks go into the crate
on screen as previews. They're the clips the page links to, fetched with yt-dlp a few tracks
ahead of what plays. Other pasted text is ignored.

- **Pages:** a label, an artist (their own and remix credits, oldest first), a release, a
  master, a user's wantlist, or a list. Addresses with or without a language prefix, the name
  part, a query or a fragment all work. Any other Discogs page shows which ones do.
- **Entries appear at once:** each listed record waits, dimmed, until its details arrive
  (records near the selected or playing entry are fetched first). It then becomes one entry per
  clip, matched to its tracklist, or "no clip". A send that's still going when you quit resumes
  when you next show or play its crate. The main window shows the progress
  ("12 of 250 releases").
- **Previews:** the playing entry and the next 3 download two at a time, with progress where
  the duration goes. While stopped, it's the current entry of the crate on screen and the next
  3. Each downloaded preview is analyzed and its waveform built before it plays, so section
  jumps and loops work from its first second.
- **Verdicts on the playing track:** `Y` keeps it. It's copied to the **Keepers** crate, marked
  ✓ wherever it appears, and its release is added to your Discogs wantlist (with a token).
  Pressing `Y` again undoes this, and the release comes off the wantlist only if the app put it
  there. `N` passes it: it's dimmed, the next track starts, and later sends leave it out.
  `I` opens the release's for-sale page in your browser. A Discogs entry's title line shows its
  side, catalog number, year and what's for sale (`· A1 · LT-012 · 1994 · 6 for sale from
  €9.00`). The entry menu (right-click) has Keep, Pass (or Undo pass) and Open for-sale page.
- **Setup (OPT ▸ Discogs…):**
  - Pages work without an account, at Discogs' lower rate limit (25 requests a minute
    instead of 60).
  - For the wantlist and the full rate, paste a personal access token (discogs.com ▸
    Settings ▸ Developers). It's checked before it's saved, is readable only by you, and is
    shown only by its last 4 characters.
  - Previews need [yt-dlp](https://github.com/yt-dlp/yt-dlp): `brew install yt-dlp` (or your
    package manager). The dialog shows the version found, and can take a path if it isn't on
    the `PATH`. Until it's found, entries say "needs yt-dlp". If clips keep failing, try
    `yt-dlp -U`.
  - The dialog also sets the default filters for every send (vinyl only, skip what you've
    passed) and the preview cache size (2 GB by default; the least recently played go first).
- Previews are for listening while you dig. They stay in the cache and are never exported.

### From the browser

A Chrome extension (in `extensions/chrome/`) adds a button to Discogs pages: Play in ‹App›,
Enqueue in ‹App› and Send to crate. It talks to the player through a small **browser bridge**
that the player starts once its window is up.

- **Local only:** the bridge listens on `127.0.0.1`, port 47800 by default. Other computers
  can't reach it. If the port is taken, the player works without the bridge and
  OPT ▸ Browser… says so; you can pick another port there (the extension's options need the
  same one).
- **Pairing (OPT ▸ Browser…):** the dialog shows a 6-digit code, valid for 2 minutes and only
  while the dialog is open. Enter it in the extension's options. The extension receives a long
  random key and sends it with every request; the player keeps only its hash, in
  `dig/bridge.ron`. After 5 wrong codes, pairing is locked for a minute. **Forget browsers**
  revokes every key, and each browser then asks to be paired again.
- **What it accepts:** only a Discogs page address (the pages that paste accepts), a mode
  (Play, Enqueue, or a crate of 1 to 40 characters), and the vinyl-only and skip-passed
  switches, in a body of at most 16 KB. Anything else, including file paths and other
  addresses, is refused and changes nothing. A send is exactly a paste: it is answered at
  once, and the crate fills in afterwards. The extension can also read the crate names, what's
  playing, and the progress of sends.
- **No web pages:** requests from web pages (a web origin, another host name, a preflight) are
  refused, and no answer allows other origins.

**Install the extension** (Chrome, or any Chromium browser: Brave, Edge, Arc):

1. Open `chrome://extensions`, turn on **Developer mode**, click **Load unpacked** and choose
   the `extensions/chrome/` folder. It asks for site access to discogs.com and 127.0.0.1 only.
2. Start the player and open OPT ▸ Browser…. The extension's options page opens on install (or
   right-click its toolbar button ▸ Options): enter the 6-digit code and click **Pair**.
3. On a Discogs release, master, artist, label, wantlist or list page, the button after the
   title (or in the bottom-right corner) offers **Play in ‹App›**, **Enqueue in ‹App›** and
   **Send to crate** (the player's crates, or New crate…), and the vinyl-only and skip-passed
   switches, which it remembers. A confirmation shows for 3 s ("Sent to ‹App›: Label: Lowtide
   Tapes → Playlist").
4. On any site, right-click a Discogs link for Play in ‹App› or Enqueue in ‹App›; the toolbar
   button shows ✓ or ! for 3 s. Clicking the toolbar button shows whether the player is running
   and paired, what's playing, and sends in progress.

‹App› is the name the player reports (`dig::APP_NAME`, a placeholder, "winamp_rust", until the
rebrand), so renaming the player renames every label. The manifest's name is a placeholder too,
and must change before any store publishing. The extension is plain JavaScript with no build
step and no dependencies: `manifest.json`, `background.js` (the only code that calls the
player, with the key), `content.js` (the button), `pages.js` (which pages are supported),
`options.*`, `popup.*` and `icons/`.

**Manual checklist** (the extension has no automated tests):

- [ ] Load unpacked in a fresh Chrome profile: the site access listed is discogs.com and
      127.0.0.1 only.
- [ ] Pair with the code from OPT ▸ Browser…; the dialog says a browser was paired, and the same
      code no longer works.
- [ ] Release, master, artist, label, wantlist and list pages each show the button, and each of
      Play, Enqueue, Send to crate and New crate… works; a forum thread shows no button.
- [ ] Moving between pages without a reload (Discogs' own links) shows and hides the button.
- [ ] Right-click a Discogs release link on another site (a forum post): Enqueue in ‹App› adds
      it and the toolbar shows ✓.
- [ ] With the player closed, an action says that it isn't running, and nothing else happens.
- [ ] After Forget browsers, an action opens the pairing screen.
- [ ] A hostile page cannot use the bridge: serve this file from `python3 -m http.server` and
      open it; every line should read "refused" or "blocked", and no crate changes:
      ```html
      <pre id=o></pre><script>
      for (const [m, p] of [["GET","hello"],["POST","send"],["POST","pair"]])
        fetch(`http://127.0.0.1:47800/v1/${p}`, {method: m, body: m == "POST" ? "{}" : undefined,
          headers: {"Content-Type": "application/json"}})
          .then(r => o.textContent += `${p}: ${r.status >= 400 ? "refused" : "ACCEPTED"} (${r.status})\n`)
          .catch(() => o.textContent += `${p}: blocked\n`);
      </script>
      ```
- [ ] From another computer on the network, `curl http://<this computer>:47800/v1/hello` fails
      to connect.

### Waveform and structure navigation

Under the main window, the **waveform** (`W`) has two rows:

- **Overview** of the whole track: coloured by frequency, with section bands, red markers where
  the energy jumps (the drops), and the playhead. Click or drag to seek.
- **Zoom** around the playhead: bass is red, mids green, highs blue, so kicks and hats are easy
  to tell apart. Beat ticks are shown, taller on bar starts. The scroll wheel zooms from 1 to
  64 bars.

The waveform fills in within a few seconds of a track starting. It's saved in the cache, so a
replayed track shows its whole waveform at once.

**Jump by structure:**
- `]` goes to the next section, and `[` to the start of this one (or the previous one if you're
  in its first bar).
- `Shift+]` goes to the next drop, meaning the next section that is at least 4 dB louder than
  the one before.

Jumps land exactly on a bar line, so the beat never stumbles. A yellow line on the waveform
shows where the jump will happen. The player uses the first bar line it can still reach: the
next one, or the one after if the next is less than half a second away.

**Loops:**
- `L` loops the current section, at most 32 bars. Press it again to stop.
- `Shift+L` loops 4 bars from the current bar line; press again for 8, then 16.

Loops repeat without a gap and show in yellow on the waveform. A seek, stop or track change
ends them.

`[` and `]` work by key position, next to `P`, so they work on any keyboard layout.

### Spectrogram

`S` (or **Spectrogram (S)** in the playlist's **OPT** menu) opens a separate, resizable window
with the current track's spectrogram: time runs left to right, frequency goes up on a log scale
from 20 Hz to the file's Nyquist frequency, and brighter means louder.

- **Track** shows the whole track with the playhead. Click to seek. Scroll to zoom time around
  the cursor, Shift+scroll to zoom frequency, drag to pan, and double-click to see the whole
  track again. Once you zoom past what the overview holds, the visible range is recomputed at
  full resolution in the background (the header shows the FFT size: short ranges use short
  windows so drum hits stay sharp, long ranges up to 8192 points for fine pitch detail).
- **Live** is a scrolling waterfall of what you hear right now, in step with the audio.
- **Mid / Side / L / R** picks the signal: mid is the sum of both channels, side their
  difference (stereo width). The whole-track view has mid and side; L and R need a zoomed or
  live view.
- **dB** sets the range the colours span (−120 to 0 dB by default), to bring out quiet detail.
- Hovering shows the time, frequency with the nearest note (e.g. `440 Hz A4`) and the level.

**Quality check:** the bottom right shows where the content ends. For a lossless file (FLAC,
WAV, ALAC) whose highs stop at a hard wall below 19.5 kHz it reads, for example, *Content ends
at 16.0 kHz: likely from a lossy source (≈128 kbps MP3)*: a sign that the file was made from an
MP3 or similar. Music that just gets quieter towards the top isn't flagged, and lossy files
(MP3, AAC, Vorbis) never are.

The whole-track view comes from the same background pass as the waveform, so it appears as the
waveform fills in and is instant on replay. That pass's cache format changed with this feature,
so tracks you played before are analyzed once more the first time you play them again.

To look at a file without playing it (for example a FLAC next to a transcoded copy):

```sh
cargo run --release -p ui --example spectrogram_probe -- song.flac
```

It prints the quality verdict and opens the window; clicks move the playhead.

### Visuals

Fullscreen visuals run on musical time: beats, bars and phrases from the analysis, with kicks,
snares and hats fired as the playhead crosses them, so motion lands on the beat you hear. At
each section change the **director** picks what to show. A big rise in energy cuts to your
highest-rated look with a flash. A drop in energy crossfades to something calm. A section that
comes back returns to the look it had before. Other changes morph to a sibling look, and the
`stretch` macro follows the track's tension. The same track always gets the same show.

Six scenes ship with it, each with at least two variants:

- **Julia Tunnel** (2D fractal);
- **Liquid Feedback** (the MilkDrop feel);
- **KIFS Cathedral** (raymarched 3D);
- **Flame** (a compute-shader fractal flame);
- **Polar Life** (cellular automata on the walls of a tunnel): spectrum onsets and kicks give birth
  at the centre, and the tunnel flies outward eight rings per beat. Cells are lit, bevelled tiles
  with halos and comet tails. Each kind of section runs its own rule: Life in grooves, Brian's
  Brain in builds, Star Wars in drops, Day & Night in breakdowns. Every kick or beat hits the tunnel with
  a burst of interference, and drops speed it up with their energy. The `hyperdrive` variant bends
  the tunnel into a swaying 3D pipe;
- **Coral Tunnel** (reaction-diffusion): onsets and kicks seed chemistry that grows into glossy
  coral or dividing cells as it streams outward.

The artist, title and progress (with section ticks) show for 5 s on entering fullscreen, on each
new track, and when you move the mouse or press a key.

| Key | Action |
|---|---|
| `D` | show/hide the fader deck: 6 macros (intensity, chaos, stretch, speed, hue, feedback) and the scene's parameters |
| `M` / `Shift+M` | mutate the current look (small / big step) |
| `K` | keep: save the current look, fader positions included, as a new variant |
| `Backspace` | undo back through the looks you had |
| `1`–`5` | rate the current look (the director prefers higher ratings) |

On the deck, dragging a fader switches it to MANUAL. When you let go, it holds, then glides back
to automation after the RETURN time (1 beat, 1 bar, 4 bars, a phrase, or ∞), landing on a bar
line. Click RETURN in the deck header to change the global setting. Right-click a fader to give it
its own RETURN, and double-click it to hand it back to automation now. Speed snaps to ¼, ½, 1, 2
and 4×.

**Make it yours:** on first use the scenes are copied to `visuals/` in the config folder
(`~/Library/Application Support/winamp_rust/visuals/` on macOS), and any file you save there is picked up
while the music plays:

```
visuals/
  director.ron              the rules for what happens at section changes (commented)
  prelude/*.wgsl            helpers every scene can call: complex math, noise, palettes, SDFs, tunnels
  scenes/<id>/scene.ron     name, tags, parameters (type, default, range), macro mappings, routes
  scenes/<id>/scene.wgsl    fn scene(uv: vec2f, m: Music, p: Params) -> vec4f
  variants/<id>/<name>.ron  saved looks (K writes these; ratings live here too)
```

A new look is one `.wgsl` and one `.ron` file in a new `scenes/<id>/` folder. You write only
`fn scene`. The engine generates `p.<param>` from your manifest and passes the music as `m`: for
example `m.beat` (the phase within the beat), `m.motion` (beats, scaled by the speed macro), `m.kick`
(beats since the last kick, so use `pulse(m.kick, 4.0)` for a punch), `m.energy`, `m.tension`,
and `band(m, i)` for the 19 spectrum bars. Routes in the manifest connect signals to parameters
without writing code, for example
`(source: Kick, shapers: [Envelope(attack: Ms(5), release: Ms(120)), Range(0, 0.35)], target: "zoom")`.
A scene can also be a cellular automaton: declare
`kind: Automaton(theta: 128, rings: 64, steps_per_beat: 4.0)` and write
`fn rule(c: vec2<i32>, m: Music, p: Params) -> vec4f` next to `fn scene`. `rule` returns a cell's
next state from the previous one, read with `cell(c)`: θ wraps around, and rings outside the grid
are empty. `inject_level(m, theta)` spreads the spectrum around the circle, `tick()` numbers
the steps, and `since_reset()` is 0 on the first step after a reset (a rule can seed its start
state there; `preroll: N` in the kind runs N steps right away, 16 by default). In `fn scene`, `state_at(depth, theta)` samples the grid and `tick_phase()` glides
between steps. The grid persists between frames, one per drawn layer. It steps on musical time,
so the show is the same at any frame rate, and it starts over on a seek, a new track or a reload.
A `step_rate` param, if the scene has one, is read once per beat and snapped to ×0.5, ×1, ×2 or
×4. For continuous rules such as diffusion, `substeps: N` in the kind runs `rule` N times per step
(up to 32; `substeps()` returns N). Any scene can fly through a tunnel with
`tube_hit(uv, focal, bend)` from `prelude/tunnel.wgsl`: it returns the depth, the angle and the
distance from the axis of a tube whose far end sits at `bend`.

Bundled scenes are copied only when missing, so after an update that changes a scene you've used
before, delete its folder under `visuals/scenes/` (and `visuals/variants/`) to get the new version.
If you save a broken shader or manifest, the previous version keeps running, and a message
shows the file, line and error for 8 seconds. To reset a file, delete it and it is restored from
the bundled copy the next time you enter fullscreen.

Resolution adapts to hold the frame rate. You can measure the scenes on your GPU:

```sh
cargo run -p visuals --example visual_bench --release        # offscreen, native Retina size
WINAMP_VISUAL_BENCH=1 cargo run -p winamp-native --release   # in the app: press F; 15 s with no visuals,
                                                             # then 15 s per scene; results in .../visuals/bench.txt
WINAMP_FRAME_STATS=1 cargo run -p winamp-native --release    # per-second frame timings (app, visuals, present)
cargo run -p ui --example fullscreen_probe --release         # what a blank eframe window can present
```

### Render a show to video

Any track's show can be rendered to an MP4 (H.264 video with the track's own audio). The show
is deterministic, so the file matches what you'd see in fullscreen with your hands off the
deck. Rendering needs ffmpeg (`brew install ffmpeg`).

```sh
winamp-native --render-show track.flac -o show.mp4                 # 1920×1080, 60 fps, the whole track
winamp-native --render-show track.flac -o clip.mp4 --from 1:00 --to 1:30 --size 1280x720 --fps 30
winamp-native --render-show track.flac -o card.mp4 --overlay       # with the artist/title card at the start
winamp-native --render-show track.flac -o one.mp4 --look julia_tunnel/solar   # one look, director off
```

(With cargo: `cargo run --release -p winamp-native -- --render-show …`.)

- **Frame timing:** frame n shows the music at exactly `start + n/fps`, so every kick lands on
  its frame.
- **Size:** the width must be divisible by 8 and the height even; vertical `1080x1920` works.
- **Progress:** the time left is shown as it renders. Ctrl-C stops it, and a cancelled or
  failed render leaves no file behind.
- **Analysis:** the track is analyzed first if it isn't in the cache yet.
- **Matching the live show:** a render matches the live show of a replay, because a first live
  play can react to sections the analyzer later corrects.

From the player, right-click a playlist entry (on a Mac: Control-click, or click with two
fingers) and choose **Render show…**. The dialog offers every command-line option:

- **Size:** Full HD, HD, 4K, vertical, square, or a custom size;
- **Frame rate:** 24–120 fps;
- **Range:** the whole track, or a From/To range such as `1:00`–`1:30`;
- **Title card:** the artist/title card at the start;
- **Look:** automatic (the director), or one of your looks.

**Render…** asks where to save. Size, frame rate and the card setting are remembered. The render
runs in the background while you keep listening, with its progress in the main window. It waits
while fullscreen visuals are on, and the same menu then offers **Cancel show render**.

To measure rendering speed without encoding:
`cargo run -p visuals --example render_speed --release -- track.flac`.

### Music analysis

While a track plays, the app analyzes it about 2 minutes ahead of what you hear: tempo, the beat
grid, bars, and sections (intro, build, drop, breakdown, groove, outro), with a countdown to the
next drop. The first 32 bars are ready about half a second after you press play, and results are
cached in the cache folder (`~/Library/Caches/winamp_rust/` on macOS), so a second play is instant. Set `WINAMP_CACHE_DIR`
to use another folder. It runs at low priority and never delays playback.

In fullscreen:

| Key | Action |
|---|---|
| `T` | show/hide the analysis strip: BPM, current section, drop countdown, section bands, beats (taller on bar starts), the tension curve and the playhead |
| `A` | annotation mode, for teaching the analyzer your music |
| `Space` *(annotating)* | tap along with the beat |
| `1`–`6` *(annotating)* | mark where a section starts: 1 intro, 2 build, 3 drop, 4 breakdown, 5 groove, 6 outro |

Annotations are saved immediately to `annotations/` in the cache folder. To see how the
analyzer scores against them:

```sh
cargo run --release -p analysis --bin analysis-eval
```

This prints each track's beat accuracy and how many of your section marks it found. The targets
are ≥ 90% and ≥ 70%.

No music annotated yet? You can try it on generated tracks:

```sh
cargo run --release -p analysis --example make_annotated -- /tmp/eval
cargo run --release -p analysis --bin analysis-eval -- /tmp/eval/annotations
```

### Terminal player

The earlier terminal harness is still available:

```sh
cargo run --release -p winamp-native -- --tui ~/Music/*.mp3
```

Run it in a real terminal (Terminal, iTerm, or the VS Code terminal): it reads keys directly,
so it won't work with piped input or in a non-interactive shell. By default it plays at 80%
volume; add `--volume 0.3` to start quieter.

| Key | Action | | Key | Action |
|---|---|---|---|---|
| `z` | previous | | `←` / `→` | seek −5 s / +5 s |
| `x` | play | | `↑` / `↓` | volume |
| `c` | pause / resume | | `e` | EQ on/off |
| `v` | stop | | `q` / `Esc` | quit |
| `b` | next | | | |

The status line shows the audible position (from the playback clock), track, volume, EQ, device
rate, the last start/seek latency, underruns, and, in debug builds, allocations detected
inside the audio callback (should always be 0).

Supported formats: MP3, FLAC, WAV, OGG Vorbis, AAC/M4A (pure-Rust decoding via symphonia).

### Measure latency on your machine

```sh
cargo run --release -p winamp-native -- --bench --volume 0 file1.mp3 file2.flac …
cargo run --release -p winamp-native -- --bench crates/audio/tests/fixtures/tone.*   # quick check
```

For each file this measures press-play → first audio at the device and three seeks. It then
plays the whole queue gaplessly. Underruns are counted over the whole session, including the
seeks. It exits non-zero if a target is missed:
start < 30 ms, seek < 50 ms, 0 underruns, 0 callback allocations (the allocation check needs a
debug build: drop `--release`). `--volume 0` runs the full pipeline silently (it is also the
default for `--bench`). Add `--analysis` to run the music analyzer during the whole bench and
confirm it doesn't cost underruns or latency.

Last measured on an M-series Mac (CoreAudio, 44.1 kHz, 512-frame buffer), 60 s files:
start 7.9–20.8 ms, seek 12.1–20.0 ms.

### Check A/V clock accuracy acoustically

```sh
cargo run --release -p winamp-native -- --click-test
```

Plays 16 clicks and records them with the default microphone. It reports how far the heard
clicks are from when the playback clock said they'd be audible (target ±2 ms). This needs your
speakers audible to the mic, and macOS will ask for microphone permission.

### Where files are kept

| | macOS | Linux | Windows | Override |
|---|---|---|---|---|
| config (settings, presets, `crates/`, `dig/`, `visuals/`) | `~/Library/Application Support/winamp_rust/` | `~/.config/winamp_rust/` | `%APPDATA%\winamp_rust\` | `WINAMP_CONFIG_DIR`* |
| cache (analysis scores, waveform `overviews/`, `annotations/`, Discogs responses in `discogs/`, `previews/`) | `~/Library/Caches/winamp_rust/` | `~/.cache/winamp_rust/` | `%LOCALAPPDATA%\winamp_rust\` | `WINAMP_CACHE_DIR` |

\* `WINAMP_CONFIG_DIR` covers settings, crates and presets; the editable `visuals/` folder
always lives in the platform config folder.

Crates are kept in `crates/`: `index.ron` lists them and each crate is `<id>.ron`. A crate file
that can't be read is reported once and left as it is; a damaged `index.ron` is rebuilt from the
crate files. The single `playlist.ron` of earlier versions is read once, on the first launch
with crates, to create the Playlist crate, and is then left untouched as a backup (the previous
version still opens it); the Playlist crate is now the one that counts.

Digging keeps its state in the config folder's `dig/`: `settings.ron` (filters, cache size,
yt-dlp path, the Keepers crate), `token` (readable only by you), `memory.ron` (kept and passed
tracks, and wantlist changes still to be sent), `jobs.ron` (sends still in progress) and
`bridge.ron` (the browser bridge's port and the SHA-256 of each paired browser's key, readable
only by you). In the
cache, `discogs/` keeps API responses: record details for good, listings for a day, and
for-sale numbers refreshed once a day when their track plays. `previews/` holds the downloaded
clips.

Other environment variables, mostly for unattended runs and measurements:

| Variable | Effect |
|---|---|
| `WINAMP_AUTO_FULLSCREEN=1` | enter fullscreen as soon as playback starts |
| `WINAMP_AUTO_QUIT_SECS=n` | close the app after `n` seconds |
| `WINAMP_VISUAL_BENCH=1` | benchmark every scene on the first fullscreen (see [Visuals](#visuals)) |
| `WINAMP_FRAME_STATS=1` | print per-second frame timings |

`winamp-native --help` prints all command-line modes.

## Tests

```sh
cargo test --workspace            # 442 tests, under a minute after the first build; no audio hardware or display needed
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo check -p audio -p platform --target wasm32-unknown-unknown   # core stays web-portable
```

Run a single test file or test with `cargo test -p audio --test engine` or
`cargo test -p audio gapless`.

Tests write their files under `<temp>/winamp_rust-tests/` and delete them when they finish.
Anything a background worker writes late, or a killed run leaves, is deleted by a later run
once it's an hour old.

The test fixtures in `crates/audio/tests/fixtures/` were generated with ffmpeg (2 s, 440 Hz, tagged
`M83 / Midnight_City`). You only need ffmpeg if you want to regenerate them.

What's covered:

- **Unit tests** per module: lock-free ring, seqlock clock (interpolation, latency, gapless
  boundary, monotonicity, concurrency), EQ (±0.5 dB at band centers, bit-identical when off, no
  clicks when sweeping, Nyquist bypass), tap, renderer, resampler.
- **`crates/audio/tests/decode_formats.rs`**: every format at 44.1 and 48 kHz (pitch, length,
  tags), accurate seeking, corrupt and garbage files. Fixtures are in
  `crates/audio/tests/fixtures/`.
- **`crates/audio/tests/engine.rs`**: the full engine against `platform::testing::ManualSink`, a
  sink the test drives by hand with fake time. Covers:
  - bit-exact playback, pause and seek;
  - bit-exact gapless playback, and gapless playback with resampling;
  - the clock hitting a click within ±2 ms with 12 ms of output latency;
  - device loss that comes back at a new sample rate;
  - the tap, EQ, volume and balance.
- **`crates/audio/tests/rt_alloc.rs`**: a counting global allocator proves the audio callback
  never allocates on any path.
- **`crates/ui`** (unit tests):
  - playlist selection, reordering and totals;
  - shuffle order, and a play order that skips entries waiting for their audio;
  - folder scanning and M3U round trip (remote entries exported as their source URL);
  - settings, crate and preset persistence;
  - crates: name rules, lazy loading, unreadable crate files, a damaged index, migration of a
    300-entry `playlist.ron`, and Send to crate without duplicates;
  - spectrum bars following the *audible* frame;
  - EQ curve;
  - skin validation, and that the committed skin matches its generator;
  - repaint policy;
  - headless egui click/drag tests of the skinned widgets;
  - the whole player driven headlessly against `ManualSink`: switching crates leaves playback
    alone, starting a track re-targets next/previous, an armed entry is audible within 100 ms
    of its audio arriving, Eject replaces only the Playlist crate, dimmed rows, the title bar's
    click (crate menu) versus drag (move), and the crate and entry menus.
- **`crates/dig`**: the Discogs client against recorded JSON (`crates/dig/tests/fixtures/`),
  covering:
  - request headers, and a rate limit that never exceeds 60 requests in any minute;
  - backing off on 429;
  - the disk cache;
  - every supported address form;
  - listings for each page kind;
  - clip extraction and matching;
  - focus-first expansion;
  - jobs resumed after a restart;
  - offline and back;
  - token checks and wantlist changes.

  The browser bridge (`crates/dig/tests/bridge.rs`) runs on an ephemeral loopback port:
  pairing (expiry, single use, lockout after 5 wrong codes), keys and Forget browsers, refusals
  by host, origin, preflight, size, unknown fields and non-Discogs addresses, no
  `Access-Control-*` header on any answer, the crates and status snapshot, a taken port, and a
  send answered in under 100 ms while Discogs is slow.

  The preview scheduler runs against a fake yt-dlp: the horizon, 2 slots, the armed entry
  first, cancelling, retries, the timeout, yt-dlp appearing later, and the cache limit. Also
  covered: the dig memory, and preparing a preview's score and overview behind the gate.
- **Digging in the player** (`crates/ui`, headless, with a fake Discogs, yt-dlp and browser):
  - a pasted release filling the crate with playable previews;
  - Play mode naming its crate and playing;
  - a missing page taking its crate away again;
  - `Y`, `N` and `I` with and without a token;
  - the entry menu;
  - OPT ▸ Discogs… checking a token;
  - "needs yt-dlp";
  - no request to Discogs before the window is interactive;
  - the browser bridge: started only after the first frame, a send answered in under 100 ms
    while Discogs is slow and then filling the crate, OPT ▸ Browser… pairing once and Forget
    browsers, and a taken port that leaves the player working.
- **`crates/ui/tests/dig_playback.rs`**: an 800-release label is expanded, and previews are
  downloaded and prepared, while the engine plays in real time. Zero underruns, and a prepared
  preview starts as fast as a local file.
- **`crates/ui/tests/waiting_skip.rs`**: track 3 is followed sample-exactly by track 5 while
  track 4 waits for its audio, and track 4 is not marked failed.
- **`crates/ui/tests/large_add.rs`**: 2,000 files get their metadata read while the engine plays
  in real time, with zero underruns.
- **`crates/analysis`**: generated electronica with exact ground truth, testing:
  - kicks within 20 ms;
  - tempo at 124/128/140/174 BPM, and tempo changes;
  - beatless intros;
  - downbeats and section boundaries on the right bar;
  - section kinds, and repeated drops sharing a label;
  - streaming, seek, cache and pre-warm.
- **`crates/ui/tests/analysis_playback.rs`**: real-time playback while two tracks are analyzed,
  with zero underruns.
- **Spectrogram** (`crates/analysis` spectral and detail modules, `crates/ui` spectrogram):
  - a tone lands on its row, and a full-scale sine reads 0 dB;
  - the whole-track overview stays between 2048 and 4096 columns for any length, and a 2-hour
    mix's complete overview within 16 MB;
  - zoomed detail resolves clicks 10 ms apart, and skips gaps by seeking in sparse views;
  - a brick wall at 16 kHz is flagged, a gradual roll-off and full-band content aren't, and
    lossy codecs never are;
  - `lossless` follows the codec for every fixture format;
  - note names, cursor readout, zoom limits, live columns lined up with the audio, and a
    headless click that seeks to the time under the pointer.
- **`crates/visuals`** (unit tests, plus GPU tests on a headless device that skip when there is
  no GPU):
  - musical time and triggers locked to the analyzed beats, including pause and seek;
  - every shaper, route determinism, and the base → manual → macros → routes → clamp order;
  - manifests, and WGSL generation checked with naga, with errors mapped to the author's line;
  - every bundled scene compiling and rendering;
  - crossfade, feedback trails, compute accumulation, hue, and broken shaders not panicking;
  - automata: steps on musical time at 30 and 144 fps, per-beat step rate with hysteresis
    (drops ×2 and ×4), catch-up, pre-roll and reset on seeks; grids that persist, wrap in θ and
    stay separate per layer; Polar Life starting from a soup, growing from kicks and emptying in
    silence; Coral Tunnel growing a kick splash with its chemistry staying in range, and its dividing cells outlasting the flow;
  - variants loading when params change, mutation, lineage and undo;
  - the director's default show (rise, fall, repeat, idle, track change, provisional boundaries);
  - fader RETURN glides landing on bar lines;
  - the overlay fade;
  - hot reload keeping the last good scene, and the file watcher;
  - the engine end to end (keys, crossfades, re-init);
  - offline show rendering: a kick at 12.500 s hitting frame 750 at 60 fps, identical frames
    across renders, BT.709 YUV conversion, the overlay card, pinned looks, an MP4 end to end
    through ffmpeg (skipped without it), cancel leaving no file, and background jobs.
- **`crates/visuals/tests/render_playback.rs`**: real-time playback while a show renders, with
  zero underruns.

Measure the player's launch time (the target is under 300 ms):

```sh
cargo run --release -p winamp-native -- --startup-time
```

On an M-series Mac it takes 142–171 ms, including with a 500-entry saved playlist, and
148–182 ms with a Discogs token and a send still in progress (nothing is sent to Discogs
before the window is up). The very
first launch after a build takes about 0.7 s while macOS compiles and caches GPU shaders.

## How it works

```
 decode thread ──lock-free ring (~0.5 s)──▶ audio callback ──▶ device (opened once, always running)
   symphonia → stereo → rubato             EQ → tap → volume/balance
   gapless + pre-warm next track           publishes the playback clock
                                                  │
                   visuals / analysis ◀── clock + tap (lock-free, never block audio)
```

- **Instant start:** the output device opens at launch and stays open (emitting silence), so
  pressing play only has to decode one packet.
- **Zero-offset visuals:** the callback publishes *which frame is at the speaker right now*
  (callback time + output latency). Consumers interpolate it at any frame rate.
- **Gapless:** the next track is opened and pre-decoded 30 s before the end. One continuous
  resampler carries across track boundaries, so even 44.1 kHz files on a 48 kHz device join
  seamlessly.
- **Real-time safe:** no allocations, locks, or waiting in the callback. Seeks flush by
  generation number instead of locking.
- **Resilient:** corrupt frames and unplayable files are skipped. If the output device
  disappears, the stream is rebuilt at the same position, even at a new sample rate.

Full details and the reasoning behind each decision:
[`openspec/changes/archive/2026-09-26-audio-core/design.md`](openspec/changes/archive/2026-09-26-audio-core/design.md).

## Layout

```
crates/platform   seam traits (AudioSink, Spawner, FileSource) + native impls + ManualSink for tests
crates/audio      decode, ring, renderer (callback), clock, EQ, tap, decode worker, Engine API
crates/analysis   music analysis: beat grid, tempo segments, sections, tension, cache, eval tools;
                  track overview (waveform + spectrogram, cutoff check) and zoomed spectrogram detail
crates/ui         the player window: skin, main/EQ/playlist sections, fullscreen host, playlist model,
                  waveform, spectrogram window
crates/dig        digging Discogs (native only): API client, rate limit and cache, page expansion,
                  send jobs, yt-dlp previews and their scheduler, preparing previews, dig memory,
                  the browser bridge (loopback server, pairing)
crates/visuals    the visual engine: signals, modulation, scenes (WGSL + RON), variants, director, GPU compositor, overlay, deck
crates/visuals/assets  the bundled scenes, variants, prelude and director rules
extensions/chrome the Chrome extension (plain JS, loaded unpacked) that sends Discogs pages to the bridge
assets/skin       the bundled original skin (atlas.png + skin.ron), generated by `cargo run -p ui --bin skin-gen`
apps/native       the desktop app: GUI (default), --tui, --bench, --click-test, --startup-time, --render-show
openspec/         specs and plans for every milestone (see below)
.claude/, .opencode/  OpenSpec agent commands (/opsx:propose, :explore, :apply, :archive)
AGENTS.md         conventions and checks for anyone (human or AI) changing the code
```

## Known limitations

- **AAC/M4A is not sample-exact gapless:** symphonia 0.6.1 ignores MP4 edit lists, so about
  43 ms of encoder priming and padding plays. MP3, FLAC, Vorbis and WAV are exact.
- **WAV tags** (RIFF `INFO`) are read by our own small parser, because symphonia 0.6.1 drops them.
- **Still to verify:** a 1-hour playlist with zero underruns, and the acoustic clock check.
- Surround files are played as their front left/right channels.

## Roadmap (OpenSpec)

Each milestone is an OpenSpec change with a proposal, design, specs and tasks in
`openspec/changes/`:

1. `audio-core`: the audio engine ✅ (done and archived, apart from the two checks above)
2. `classic-ui`: the Winamp-style skinned player, EQ and playlist (egui/wgpu), and the
   fullscreen key ✅ (done and archived)
3. `music-analysis`: beat grid, phrases, build/drop/breakdown detection that analyzes ahead of
   the playhead ✅ (done and archived)
4. `visual-engine`: fractal scenes (WGSL), a modulation matrix, a director, a fader deck, and an
   auto-fading track overlay ✅ (done and archived)
5. `web-target`: the same app in Chrome via WebAssembly, AudioWorklet and WebGPU (proposed in
   `openspec/changes/web-target/`, not started)
6. `waveform-navigation`: the coloured waveform, section/drop jumps on the beat, bar loops,
   and the shortcuts help ✅ (done and archived)
7. `spectrogram-window`: a spectrogram window with whole-track, zoomed and live views, and a
   check for files made from lossy sources ✅ (done and archived)
8. `beatmatch-automix`: tempo-matched, phrase-aligned DJ mixes between tracks (proposed)
9. `show-render`: render a track's visual show to an MP4, from the command line or the playlist
   ✅ (done and archived)
10. `crates`: named playlists ("crates") switched from the playlist's title bar, with entries that
    remember the record they came from and can wait for their audio ✅ (done and archived)
11. `discogs-digging`: paste a Discogs page (label, artist, release, master, wantlist or list) to
    dig it in a crate. Previews are downloaded and analyzed a few tracks ahead, so each one opens
    ready to navigate. `Y` keeps a track (and adds it to your Discogs wantlist), `N` passes and
    `I` opens its for-sale page ✅ (done and archived)
12. `browser-bridge`: a Chrome extension with Play in / Enqueue in / Send to crate on Discogs pages
    and links, talking to the player through a paired local bridge (proposed; needs
    `discogs-digging`)

Finished changes move to `openspec/changes/archive/`, and their requirements become the living
specs in `openspec/specs/`.

The `openspec` CLI is optional; it's only needed to browse or advance the plans:

```sh
npm install -g @fission-ai/openspec   # requires Node.js
openspec list                         # active changes and task progress
openspec list --specs                 # specs of what's already built
openspec show classic-ui              # read the next milestone
openspec show audio-playback --type spec
```
