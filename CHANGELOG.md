# Changelog

What changed for people using Diggr, newest first. Each entry is written to be pasted onto
the landing page or into the release notes as it is: a bold headline, then one or two plain
sentences. `## Unreleased` collects entries until a release renames it.

## Unreleased

### New

- **Diggr on Linux.** A download for x86_64 Linux sits next to the Mac one: extract it and run
  the player, or run `install.sh` to add it to your applications menu. No root, and it works on
  any distribution from Debian 11 and Ubuntu 20.04 on.
- **Previews ready on Linux too.** Without yt-dlp installed, the first preview offers to download
  it for you, checks it and keeps it up to date, as it does on a Mac.

### Improved

- **Dig a giant shop by style.** Narrow down now lets you pick a genre or a style, a format and a
  year, so a seller with 150,000 records becomes the 4,000 Deep House ones in a second, across
  their whole stock, not just the newest. Every change answers at once, and the styles you
  collect and want come first in a list you can search.

### Changed

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
