## Why

Most of what makes Diggr a digging tool needs two things people don't find on their own: their
Discogs account (wantlist and collection sync, the cart, the faster rate) and the Diggr Bridge
browser extension (sending any Discogs, Bandcamp, Juno, Beatport, Traxsource or Bleep page with
one click). Today the app mentions the account only after a want without a token, and never
mentions the extension, which is published on the Chrome Web Store but linked from nowhere
(the README still describes loading it unpacked). New users meet an empty player and don't
learn what it can do.

## What Changes

- **A welcome at launch**: after the first frame, a modal lists the setup steps still open:
  Discogs (Connect…, or "I don't have an account" with a link to create one) and the browser
  (Get the extension, which opens the Chrome Web Store listing, then pairing; or "I don't use
  Chrome"); on Windows, Previews (the yt-dlp install of `ytdlp-onboarding`) comes first, so
  there is one modal, not two. It opens at every launch while a step is open, until every step
  is done or declined, or the user ticks "Don't show at startup".
- **Steps remember "no"**: a declined step is never asked again, at launch or anywhere else,
  until the user reopens it.
- **The same panel on demand**: OPT ▸ Set up Diggr…, and "‹n› of ‹m› set up" in the status line
  while a step is open, open it at any time.
- **Empty crates show what to do**: an empty crate shows cards to paste a Discogs link, connect
  Discogs and get the extension (each hidden once done or declined).
- **Locked features say what unlocks them**: actions that need the account or the extension say
  so and open the panel at that step.
- **Nudges at the moment they pay off**, at most one a session and never for a declined step:
  pasting a store page only the extension can send offers Get the extension; anonymous sends
  slowed by Discogs' lower rate say connecting doubles the speed.
- **The README links the Chrome Web Store** instead of describing Load unpacked.

## Capabilities

### New Capabilities
- `welcome-setup`: the setup steps and their states, the welcome at launch, the panel on
  demand, empty-crate cards, locked features and contextual nudges.

### Modified Capabilities
- `discogs-write`: "Connect to Discogs" respects a declined Discogs step (a one-line message
  instead of the dialog) and its Connect… goes through the panel.
- `ytdlp-setup` (added by `ytdlp-onboarding`): on Windows the first-launch question becomes the
  welcome's Previews step. This change is archived after `ytdlp-onboarding`.

## Impact

- `crates/ui/src/app/`: a welcome module (the panel, its steps and states), the status line,
  empty-crate drawing in the playlist, the entry menu and Options, the paste and Connect paths.
- `crates/dig/src/config.rs`: the steps' states and "show at startup" in the dig settings.
- `crates/dig/src/bridge`: whether a browser is paired (for the step's state).
- `README.md`, `CHANGELOG.md`, and the diggr.es pages that mention the extension.
- Unchanged: launch time (the modal comes after the first frame), playback, the bridge's
  security model, how the token is entered (until `discogs-oauth`).
