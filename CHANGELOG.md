# Changelog

What changed for people using Diggr, newest first. Each entry is written to be pasted onto
the landing page or into the release notes as it is: a bold headline, then one or two plain
sentences. `## Unreleased` collects entries until a release renames it.

## Unreleased

### New

- **A short setup when you open Diggr.** It shows what your Discogs account and the browser
  extension add, connects the one and installs the other in a click or two, and stops asking
  about anything you say you don't have. Open it again any time from OPT ▸ Set up Diggr….
- **Connect with Discogs.** Click Connect with Discogs, then Authorize in your browser, and your
  account is connected: no developer settings, no token to copy. A personal token still works
  under "Use a personal token instead", and Disconnect forgets either one. What's set
  up and working shows in green: your account, your collection, yt-dlp and your paired browsers.
  Discogs turns away a computer whose clock is wrong, so if yours is off Diggr says by how much and where to set it right.
- **A record card, one click away.** In a grouped crate, click a record and its card opens in
  the middle of the window: the cover, the label, catalogue number and year, its styles, what's
  for sale, and its tracklist with BPMs. Its tracks start downloading the moment it opens, so
  they're ready when you press play, and a slim bar lets you scrub through the one playing.
  Want it or put it in your cart from the card, and slide to the next or previous record with
  ‹ › or the arrow keys. When a record finishes, the card slides on to the next one with the
  music. × or Esc puts it away.

### Changed

- **Tempo to a hundredth.** SYNC's tempo now goes to 0.01 BPM, shown small beside the big digits.
  Click the digits and type a tempo like 126.35, or fine-tune it with Shift+drag or Shift+scroll on
  the pitch fader, or Alt+↑ / Alt+↓; with SYNC on, plain ↑ / ↓ move it by 0.1 BPM.
- **A seek bar in the player.** Under the transport buttons, a bar shows how far into the track
  you are; click or drag it to jump, and it tells you the time before you let go.
- **Jumps without a click.** Seeking, or skipping to the next or previous track, now fades the
  sound out and back in over a few milliseconds instead of cutting it, so it no longer clicks.
- **The turntable is always there, and the band is in thirds.** The player, the turntable
  and the waveform each take a third of the top, so the waveform is no longer squeezed to a
  sliver when you turn SYNC on, and the band no longer jumps taller. SYNC is now a button on
  the turntable, which sits dimmed until you use it, and you can drag the window by any empty
  part of the band.
- **A header across the top.** DIGGR sits in the middle of a title bar like the skin's others,
  with help, Options, minimize and close at its right: no more three small buttons in a mostly
  empty corner, and the waveform gets the width back. Drag the header to move the window,
  double-click it to maximize or restore, and click `?` for the shortcuts.
- **SYNC says what it's doing.** The turntable has a line that reads LOCKED +3.3%, ANALYZING,
  or OWN · OUT OF ±8% (a record too far from your tempo), a pitch meter showing how hard the
  record is pushed, a flash round the platter when it locks, and digits that count to your
  tempo when a new track starts. The target digits are easier to read too.
- **A second line in the player.** Under the track it shows the record's label, side, year and
  what's for sale, or a file's album.
- **A tidier window.** The gear for Options sits in the top-right corner between minimize and
  close, ▤ (group by record) and SYNC are in the player beside EQ and the spectrogram, and the
  player's two rows now line up. The playlist's footer is gone: its time readout is in the
  filter bar after the search, and the list gets about three more rows.
- **The waveform is always on.** Its button and the W key are gone, and the list's bottom edge
  matches its left one.
- **Records first.** Every crate opens grouped by record, new or old, until you turn it off with
  ▤. A crate of loose tracks keeps its column header so you can still sort it.

- **Lighter track tooltips.** Hovering a track now shows just its details as text, with no
  cover. The cover and the record's details are on its record card: press Shift+G to group
  the crate by record.
- **Fewer buttons under the playlist.** The + button is gone: right-click below the last track
  for Add files…, Add folder… and Export M3U…, next to Select all and Sort. Add files… opens
  M3U playlists too, and any crate's right-click in the sidebar exports it without opening it.
  An empty crate now has an Add files from your computer link, and Cmd+O and dropping files
  work as before.
- **Size on the zoom keys.** Cmd+= switches Diggr to double size and Cmd+− (or Cmd+0) back to
  classic size, and the pixels stay crisp at both. Options now holds just Discogs…, Browser…
  and Set up Diggr…. The spectrogram is on the player's spectrum button and S, as before.

### Fixed

- **Previews work on a Mac again.** The yt-dlp Diggr downloads for you now starts in about a
  second instead of never, after a one-time "Preparing yt-dlp…" of up to a minute. If you had
  the old copy, Diggr swaps it by itself, and a slow yt-dlp can no longer pile up in the
  background and slow your Mac down. To win back the disk space the old copy wasted, you can
  delete the `_MEI` folders in your temporary folder.
- **A track's menu holds still.** Analyze BPM is always in the right-click menu now, greyed
  out when there's nothing to analyze, so the items below it no longer jump as a preview
  finishes downloading while you aim for one.

## 0.1.5 — 2026-11-04

### Improved

- **Previews set themselves up on Windows.** The first time you open Diggr, and whenever you
  play a preview without yt-dlp, Diggr offers to install it: click Install, watch it come in,
  and the track plays. Nothing to download by hand any more, and every program it runs for
  previews is signed.

### Changed

- **Diggr is free software.** It now comes under the GNU GPL, version 3 or later: you may
  use it, share it and change it, and anything built from it stays free in the same way. Each
  download carries the license.

## 0.1.4 — 2026-11-02

### New

- **Diggr on Windows.** A download for 64-bit Windows 10 sits next to the Mac and Linux ones:
  extract the zip and open Diggr, with nothing to install. Windows asks once whether to run it
  (click More info, then Run anyway) because the app isn't signed yet. Previews need yt-dlp
  there, and the note in the zip says how to get it.
- **Record stores to dig from the first launch.** Top Sellers now comes with six Spanish
  record stores that sell on Discogs: Melting Pot in Madrid, Rotor in Madrid, Rhythm Control in
  Barcelona, KH Vinyl Store in Valencia, Mita in Gijón and Adepta online. Double-click one to dig
  its stock (connect Discogs first if you haven't). If you already had a list, it keeps its
  sellers, a shop already in it takes its proper name, and the missing ones are added after yours.
  Remove any you don't want: it stays removed. A seller you haven't dug yet says "click
  here to preview its records for sale" in its empty list, and "here" starts it: one click, as
  well as the double-click.

## 0.1.3 — 2026-10-29

### New

- **Play any track from a Discogs release page.** A ▶ now sits before each track of a
  release's tracklist: click it and Diggr plays that track, not the first one, adding the record
  to the crate on screen. The playing track shows ■, and clicking it stops the player.
- **Insights into your collection.** Right-click your collection and choose Open Insights:
  your vinyl by year, style and decade, your top labels and artists, what it's worth and your
  favourite tempos. Click any bar to see those records, and follow a label in one click.
- **Diggr on Linux.** A download for x86_64 Linux sits next to the Mac one: extract it and run
  the player, or run `install.sh` to add it to your applications menu. No root, and it works on
  any distribution from Debian 11 and Ubuntu 20.04 on.
- **Previews ready on Linux too.** Without yt-dlp installed, the first preview offers to download
  it for you, checks it and keeps it up to date, as it does on a Mac.

### Improved

- **Nothing to buy twice.** A record you already own, this pressing or another, no longer
  offers + CART, in your collection or any other crate. If one is already in your cart, it
  stays there for you to take out, and its tooltip says you own it.
- **Straight to checkout.** Everything in a cart now has a PAY » button beside IN CART: click it to
  open your Discogs cart, or your Juno or Bleep basket, and pay. Right-click any crate's list
  to open a cart that holds something.
- **One cart button per record.** In the list, a record's tracks share one + CART, centred
  beside them, and a thin line sets each record apart from the next.
- **Your wantlist and collection stay put.** Their menus no longer offer Rename crate or
  Delete crate, and Delete in the sidebar leaves them alone: they mirror your Discogs account.
- **Dig a giant shop by style.** Narrow down now lets you pick a genre, a style, a format and a year,
  so a seller with 150,000 records becomes the 4,000 Deep House ones in a second, across
  their whole stock, not just the newest. Every change answers at once, and the styles you
  collect and want come first in a list you can search.

- **Dig up to 5,000, a thousand at a time.** When up to 5,000 copies match, Dig brings the newest
  1,000 at once, and the end of the list offers the next 1,000 whenever you want them.

- **Tracks where you look.** A record still waiting for its tracklist blinks "tracks coming…",
  and the records you scroll to get their tracks first.
- **See a dig at work.** The status line says how many records a dig has added, then how many
  have their tracks, how many tracks came in, and the record that just got them.
- **A new search in the same shop.** Right-click a seller and choose New search… to clean
  its crate, previews included, and dig the shop again for something else.
- **Cleaner sidebar names.** Sellers, labels and friends are listed by their names alone under
  TOP SELLERS, LABELS and FRIENDS, without "Seller:", "Label:" or "Friend:" in front.

### Changed

- **Full screen, one layout.** Diggr opens full screen: the mini player, the turntable and the
  waveform share a band across the top, and the playlist, with its crate sidebar, fills the rest.
  The classic side-by-side window, its ⇔ button and Shift+P are gone.
- **Everything in the mini player.** Shuffle and repeat sit beside the volume. Three icons at the
  end of its LCD line open the equalizer, in a window of its own that stays on top, show or hide
  the waveform, and open the spectrogram. Its title line flashes WANTED or PASS after a verdict,
  minimize and close sit in the band's top-right corner, and a seek bar takes the waveform's
  place while it's off. The little spectrum, the bitrate readout and the CRATES panel are gone:
  the spectrogram and the crate sidebar do their jobs.
- **A quicker Narrow down.** Price, condition and ships-from are gone, so the dialog never waits
  for pages of listings. Each copy still shows its price, condition and country; a seller you
  narrowed by them keeps its other choices.

## 0.1.2 — 2026-10-22

### New

- **Deleting frees the space.** Remove a track or a record, or delete a whole crate, and its
  downloaded previews leave the cache too, unless another crate still has them.
- **Your crates beside the player.** The new crates button next to EQ swaps the equalizer for a
  list of your own crates, the same size, so you can switch crates and drop tracks on them even
  with a narrow playlist. Click EQ to get the equalizer back.

### Improved

- **A sidebar that never runs out of room.** Labels, Top Sellers and Friends now open one at a
  time, your own crates fold under CRATES, and a long list scrolls instead of being cut off, so
  every crate stays in reach. Show a crate from anywhere and the sidebar opens and scrolls to
  it; hold dragged tracks at the edge of a list to scroll it. After updating, every group starts
  closed once.
- **A tidier crate menu.** In a narrow playlist, the title-bar menu now lists labels, sellers and
  friends under their own headings and scrolls, starting at the crate you're on.

### Fixed

- **Deleting a crate no longer closes Diggr.** Deleting the crate you were looking at right after
  launch used to quit the app; now Playlist simply takes its place.

## 0.1.1 — 2026-10-21

### New

- **Space plays and pauses.** Like every other player: Space plays and pauses wherever the
  keyboard is, and Shift+Space stops. The classic Z X C V B keys still work.
- **Buy from any list.** Every record now has a **+ CART** button, not just the ones from a
  seller or a store. If Diggr knows one place to buy it, a click takes you there; if it knows
  several, or none, you choose a store, or search Discogs, Bandcamp, Juno, Bleep, Beatport and
  Traxsource for it in one click.
- **Know when a record has no preview anywhere.** Once Diggr has tried YouTube, the album upload
  and Bandcamp for every track, it tells you, says when it's probably just not released yet,
  and lets you retry later or search Bandcamp (or Discogs) for it in one click. A record with
  only some tracks found shows "2/5 previews".
- **See what's being searched.** A track Diggr is looking for on YouTube now says SEARCHING… on
  its row, pulsing, so you can tell it apart from tracks still waiting their turn.
- **Click a track, hear the record.** If the track you clicked has no preview, Diggr moves on to
  the next track of the same record by itself, until it finds one that plays.
- **Dig a DJ's mix.** Paste a YouTube or SoundCloud mix, or click the extension's button on its
  page, and its tracklist becomes a crate, in mix order: every track is found and plays, unidentified ones keep their place, and "Open in
  mix" takes you back to the moment it played. Find on Discogs links a track to its record.

### Improved

- **Play any record you see in a store.** With the browser extension, every record on Discogs,
  Bandcamp, Juno, Traxsource, Beatport and Bleep listings and search results gets a small ▶:
  one click plays just that record in Diggr, without leaving the page.

- **See at a glance what's playing.** The play, pause or stop button lights up to show the
  current state, in the player and in the mini player of the maximized playlist.
- **A full mini player.** The maximized playlist's mini player now has its own pause button and
  the SYNC button, and the turntable opens right beside it, so you never have to leave the big
  view to steer a set.
- **SYNC tells you what it does.** Turn it on and Diggr says every track in the list now plays at
  your set's BPM; turn it off and they're back at their own tempo.

- **A clearer empty crate.** An empty crate now shows every store you can dig from, each a
  click away in its own colour, and how to set up the browser extension for the full
  experience.

### Changed

- **Open records with the arrows.** With the playlist focused, → opens the record under the
  cursor and ← closes it (Space used to do this).
